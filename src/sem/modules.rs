use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ordermap::OrderMap;

use crate::intern::{Interner, Sym};
use crate::load::{RESERVED_DIR, ROOT_FILE};
use crate::pkg::{PackageId, Packages};
use crate::sem::Error;
use crate::src::{Location, Sources, Span};
use crate::syn::nodes::{Expr, ExprId, ModuleItem, ModuleItemId, Place};
use crate::syn::{Chunk, ChunkId, Chunks};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ModuleId(u32);

impl ModuleId {
	pub fn index(self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug)]
pub struct Modules {
	pkg: PackageId,
	mods: Vec<Module>,
	by_file: HashMap<PathBuf, ModuleId>,
	by_name: HashMap<Vec<Sym>, ModuleId>,
	by_chunk: HashMap<ChunkId, ModuleId>,
	order: Vec<ModuleId>,
}

#[derive(Debug)]
pub struct Module {
	chunk: ChunkId,
	file: PathBuf,
	name: Vec<Sym>,
	name_span: Span,
	members: OrderMap<Sym, Member>,
	exports: Option<Vec<Sym>>,
	imports: Vec<Import>,
}

// A member of a module, named by the declaration that gives the module the
// member. Every item id indexes the arena of the module's own chunk, except a
// child's, which names the module that another file declares.
#[derive(Debug, Clone)]
pub enum Member {
	Type(ModuleItemId),
	Proto(ModuleItemId),
	Proc(ModuleItemId),
	// A top-level assignment. If there are later reassignments, this is the
	// first one.
	Var(ModuleItemId),
	// A module declared in a subdirectory file, whose leaf segment is a member
	// of this one.
	Child(ModuleId),
}

// What a dotted path names, as far as module structure can carry it. Only
// module member lookups are resolved, not type member lookups: a path that
// leaves module structure before it ends stops at 'Partial', and whatever its
// member names must resolve the segments that remain.
pub enum Resolved<'path> {
	Member(PackageId, ModuleId, Member),
	Module(PackageId, ModuleId),
	Partial(PackageId, ModuleId, Member, &'path [Sym]),
}

// What a name means in the file declaring a module.
#[derive(Debug, Clone)]
pub enum Binding {
	// A name the module declares itself. It lives in the module's own scope,
	// and a module always declares into its own package.
	Own(ModuleId, Member),
	// A member of another module, bound by a member-form import.
	Imported(PackageId, ModuleId, Member),
	// A module, bound by a module-form import.
	Module(PackageId, ModuleId),
}

#[derive(Debug)]
struct Import {
	pkg: PackageId,
	target: Target,
	span: Span,
}

#[derive(Debug)]
enum Target {
	Module(ModuleId),
	Member(ModuleId, Sym),
}

impl Target {
	// Imports are drawn at *module*, not *member* boundaries, so if the target
	// is a member of a module its imported module collapses to that module.
	fn module(&self) -> ModuleId {
		match self {
			Self::Module(id) => *id,
			Self::Member(id, _) => *id,
		}
	}
}

impl Modules {
	fn get(&self, id: ModuleId) -> &Module {
		&self.mods[id.index()]
	}

	pub fn pkg(&self) -> PackageId {
		self.pkg
	}

	pub fn ids(&self) -> impl Iterator<Item = ModuleId> + use<> {
		(0..self.mods.len() as u32).map(ModuleId)
	}

	pub fn chunk(&self, id: ModuleId) -> ChunkId {
		self.get(id).chunk
	}

	pub fn by_chunk(&self, chunk: ChunkId) -> ModuleId {
		self.by_chunk[&chunk]
	}

	// The module's declared dotted path, e.g. for a qualified name or the
	// module's printed form.
	pub fn path(&self, id: ModuleId) -> &[Sym] {
		&self.get(id).name
	}

	// The order that every module's top-level expression pass should run in. A
	// post-order DFS over import edges, so a module's dependencies are always
	// evaluated before it.
	pub fn order(&self) -> &[ModuleId] {
		&self.order
	}

	// A member of a module: what another module reaches through a dotted path,
	// and what this one makes available to anything importing it.
	pub fn member(&self, id: ModuleId, name: Sym) -> Option<Member> {
		self.get(id).members.get(&name).cloned()
	}

	// A name as the file declaring this module sees it: the members it declares
	// itself, then the names its imports bind, and nothing else. A module is
	// not visible to another that does not import it, however the two are
	// related.
	//
	// Neither namespace contains the other. An imported name is bound without
	// becoming a member, and a child module is a member without being bound.
	//
	// This binds one name. A dotted path binds its first segment here and walks
	// members from module to module for the rest.
	pub fn binding(&self, pkgs: &Packages, id: ModuleId, name: Sym) -> Option<Binding> {
		if let Some(member) = self.member(id, name) {
			match member {
				Member::Type(_) | Member::Proto(_) | Member::Proc(_) | Member::Var(_) => {
					return Some(Binding::Own(id, member));
				}
				// A child module is a member of this one but is declared by
				// another file, which this one no more sees into than any
				// other. Naming the child takes an import like anything else.
				Member::Child(_) => {}
			}
		}
		for import in &self.get(id).imports {
			match import.target {
				// A module-form import binds the module itself, under the leaf
				// of its declared name.
				Target::Module(owner) => {
					let owner_mods = if import.pkg == self.pkg {
						self
					} else {
						&pkgs.get(import.pkg).modules
					};
					if *owner_mods.get(owner).name.last().unwrap() == name {
						return Some(Binding::Module(import.pkg, owner));
					}
				}
				// A member-form import binds that one member and nothing else
				// of the module holding it.
				Target::Member(owner, bound) => {
					if bound == name {
						let owner_mods = if import.pkg == self.pkg {
							self
						} else {
							&pkgs.get(import.pkg).modules
						};
						return Some(Binding::Imported(
							import.pkg,
							owner,
							owner_mods.member(owner, name)?,
						));
					}
				}
			}
		}
		None
	}

	// Resolve a dotted path, starting from a particular module, to the member
	// it points at and the module that contains it. Only resolves *module*
	// member lookups, not *type* member lookups; the rest of the path that
	// couldn't be traversed (if any) is returned.
	//
	// Because the path is relative to the 'start' module, it can begin with a
	// *binding* of that module in addition to a member of it.
	pub fn resolve_path_from<'path>(
		&self,
		pkgs: &Packages,
		start: ModuleId,
		path: &'path [Sym],
	) -> Option<Resolved<'path>> {
		// The first part must be resolved relative to the current module's
		// *bindings*, but subsequent parts must be resolved relative to each
		// child module's *members*. Check the first part here, and the
		// subsequent parts separately.
		let (pkg, mut owner, mut member) = match self.binding(pkgs, start, path[0])? {
			Binding::Own(owner, member) => (self.pkg, owner, member),
			Binding::Imported(pkg, owner, member) => (pkg, owner, member),
			// A root module has no parent to be a member of, so seed the walk
			// with a self-reference instead. It never escapes this function:
			// the loop below only ever reads it to re-derive 'owner', and a
			// trailing 'Child' normalises back to 'Resolved::Module' rather
			// than being handed to a caller.
			Binding::Module(pkg, id) => (pkg, id, Member::Child(id)),
		};
		let mods = if pkg == self.pkg {
			self
		} else {
			&pkgs.get(pkg).modules
		};
		// For the remaining parts, look them up relative to each resolved part
		// in turn.
		for (i, part) in path[1..].iter().enumerate() {
			let Member::Child(id) = member else {
				return Some(Resolved::Partial(pkg, owner, member, &path[i + 1..]));
			};
			owner = id;
			member = mods.member(owner, *part)?;
		}
		match member {
			Member::Type(_) | Member::Proto(_) | Member::Proc(_) | Member::Var(_) => {
				Some(Resolved::Member(pkg, owner, member))
			}
			Member::Child(id) => Some(Resolved::Module(pkg, id)),
		}
	}

	fn parent(&self, id: ModuleId) -> Option<ModuleId> {
		let module = self.get(id);
		if module.file == Path::new(ROOT_FILE) {
			return None;
		}
		let parent_dir = module.file.parent().unwrap();
		if parent_dir == Path::new(RESERVED_DIR) {
			return None;
		}
		let parent_file = if parent_dir == Path::new("src") {
			Path::new(ROOT_FILE)
		} else {
			&parent_dir.with_extension("ms")
		};
		self.by_file.get(parent_file).copied()
	}
}

// Check all invariants about the module graph, returning as many errors as
// possible.
pub fn check(
	syms: &Interner,
	pkgs: &Packages,
	pkg_id: PackageId,
	sources: &Sources,
	chunks: &Chunks,
) -> Result<Modules, Vec<Error>> {
	let mut errs = check_sibling_files_exist(sources);

	let mut mods = match build(syms, pkgs, pkg_id, sources, chunks) {
		Ok(mods) => mods,
		Err(mut header_errs) => {
			// If there are errors in the module headers, checking them won't
			// result in anything sensible; error early.
			errs.append(&mut header_errs);
			return Err(errs);
		}
	};

	errs.append(&mut check_child_prefixes_match(syms, sources, &mods));
	errs.append(&mut check_members_unique(syms, sources, chunks, &mut mods));

	// Resolution reads every module's members, so it runs after they are
	// filled. Cycles are a property of the resolved edges, so they run after
	// that.
	errs.append(&mut resolve_imports(
		syms, pkgs, pkg_id, sources, chunks, &mut mods,
	));
	errs.append(&mut check_import_cycles(syms, sources, chunks, &mods));

	if !errs.is_empty() {
		return Err(errs);
	}

	// The order walk needs to know that the import graph is acyclic, so it runs
	// after.
	mods.order = compute_order(&mods);

	Ok(mods)
}

// Build the *incomplete* module graph. Later passes will flesh out its data.
fn build(
	syms: &Interner,
	pkgs: &Packages,
	pkg_id: PackageId,
	sources: &Sources,
	chunks: &Chunks,
) -> Result<Modules, Vec<Error>> {
	let mut mods = Vec::new();
	let mut by_file = HashMap::new();
	let mut by_name = HashMap::new();
	let mut by_chunk = HashMap::new();
	let mut errs = Vec::new();

	for chunk_id in chunks.ids() {
		let chunk = chunks.get(chunk_id);
		let file = sources.get(chunk.src).file().to_path_buf();
		let (name, name_span) = match read_header(sources, chunks, chunk_id) {
			Ok(header) => header,
			Err(mut header_errs) => {
				errs.append(&mut header_errs);
				continue;
			}
		};
		let id = ModuleId(mods.len() as u32);
		by_file.insert(file.clone(), id);
		by_chunk.insert(chunk_id, id);
		// Nothing ties a module's last segment to its file name, so two files
		// can declare one path; error only when encountering a duplicate
		// *declared* name.
		if by_name.insert(name.clone(), id).is_some() {
			errs.push(Error::DuplicateModuleName(
				sources.loc(name_span),
				syms.resolve_path(&name),
			));
		}
		for (other_id, other_pkg) in pkgs.iter() {
			if other_id == pkg_id {
				continue;
			}
			if other_pkg.modules.by_name.contains_key(&name) {
				errs.push(Error::ConflictingModuleName(
					sources.loc(name_span),
					syms.resolve_path(&name),
					other_pkg.manifest.name.clone(),
				));
			}
		}
		mods.push(Module {
			chunk: chunk_id,
			file,
			name,
			name_span,
			members: OrderMap::new(),
			exports: read_exports(chunks.get(chunk_id)),
			imports: Vec::new(),
		});
	}

	if !errs.is_empty() {
		return Err(errs);
	}

	Ok(Modules {
		pkg: pkg_id,
		mods,
		by_file,
		by_name,
		by_chunk,
		order: Vec::new(),
	})
}

// Determine the module path that the chunk declares by reading the 'module'
// header. Error if it is missing, duplicated, or otherwise misplaced.
fn read_header(
	sources: &Sources,
	chunks: &Chunks,
	chunk_id: ChunkId,
) -> Result<(Vec<Sym>, Span), Vec<Error>> {
	let chunk = chunks.get(chunk_id);
	let mut header = None;
	let mut errs = Vec::new();

	for (i, item_id) in chunk.top.iter().enumerate() {
		let module = match chunk.get_module_item(*item_id) {
			ModuleItem::Module(module) => module,
			_ => continue,
		};
		let span = chunk.get_module_item_span(*item_id);
		if header.is_some() {
			errs.push(Error::DuplicateModuleHeader(sources.loc(span)));
			continue;
		}
		if i != 0 {
			errs.push(Error::MisplacedModuleHeader(sources.loc(span)));
		}
		header = Some((module.path.clone(), span));
	}

	if header.is_none() {
		let file = sources.get(chunk.src).file();
		errs.push(Error::MissingModuleHeader(Location::file(
			file.to_path_buf(),
		)));
	}

	if !errs.is_empty() {
		return Err(errs);
	}

	Ok(header.unwrap())
}

// Determine the exports that the module declares.
fn read_exports(chunk: &Chunk) -> Option<Vec<Sym>> {
	let mut exports: Option<Vec<Sym>> = None;
	for item_id in &chunk.top {
		if let ModuleItem::Export(export) = chunk.get_module_item(*item_id) {
			if let Some(exports) = &mut exports {
				exports.extend(export.names.iter());
			} else {
				exports = Some(export.names.to_vec());
			}
		}
	}
	exports
}

// For every directory in the package, check that a file exists in the same
// parent with the same name as the directory.
fn check_sibling_files_exist(sources: &Sources) -> Vec<Error> {
	let mut errs = Vec::new();
	for dir in sources.dirs() {
		if dir == Path::new(RESERVED_DIR) {
			errs.push(Error::ReservedDirectory(Location::file(dir.clone())));
			continue;
		} else if dir.file_name().unwrap() == "src" {
			continue;
		} else {
			let sibling = dir.with_extension("ms");
			if !sources.files().any(|file| file == &sibling) {
				errs.push(Error::UnpairedDirectory(Location::file(dir.clone())));
			}
		}
	}
	errs
}

// For every module in a subdirectory, check that the prefix of its module path
// matches the module path declared by the parent file which is the sibling of
// that directory.
fn check_child_prefixes_match(syms: &Interner, sources: &Sources, mods: &Modules) -> Vec<Error> {
	let mut errs = Vec::new();
	for id in mods.ids() {
		let parent = match mods.parent(id) {
			Some(parent) => mods.get(parent),
			None => continue,
		};
		let module = mods.get(id);
		if module.name.len() == parent.name.len() + 1
			&& module.name[..parent.name.len()] == parent.name
		{
			continue;
		}
		errs.push(Error::PrefixMismatch(
			sources.loc(module.name_span),
			syms.resolve_path(&module.name),
			syms.resolve_path(&parent.name),
		));
	}
	errs
}

// For every member in a module—which *includes* direct submodules—check that it
// does not duplicate the name of another member. While checking, store the
// (verified unique) members in the module map for later phases.
fn check_members_unique(
	syms: &Interner,
	sources: &Sources,
	chunks: &Chunks,
	mods: &mut Modules,
) -> Vec<Error> {
	let mut errs = Vec::new();

	for id in mods.ids() {
		let chunk = chunks.get(mods.get(id).chunk);
		let mut members = OrderMap::new();
		for item_id in &chunk.top {
			let span = chunk.get_module_item_span(*item_id);
			let (name, member) = match chunk.get_module_item(*item_id) {
				ModuleItem::Module(_) => continue,
				ModuleItem::Import(_) => continue,
				ModuleItem::Export(_) => continue,
				ModuleItem::Type(type_) => (type_.name, Member::Type(*item_id)),
				ModuleItem::Proto(proto) => (proto.name, Member::Proto(*item_id)),
				ModuleItem::Def(def) => (def.name, Member::Proc(*item_id)),
				// A second 'x := ...' is reassignment, not redeclaration; a
				// binding claims its key only if nothing holds it, and names
				// the declaration that first claimed it.
				ModuleItem::Expr(expr_id) => match bound_name(chunk, *expr_id) {
					Some(name) => {
						members.entry(name).or_insert(Member::Var(*item_id));
						continue;
					}
					None => continue,
				},
			};
			if members.insert(name, member).is_some() {
				errs.push(Error::DuplicateMember(
					sources.loc(span),
					syms.resolve(name).to_string(),
				));
			}
		}
		mods.mods[id.index()].members = members;
	}

	// A child module's name is a member of its parent, so it collides with the
	// parent's declarations even though the two live in different files.
	for id in mods.ids() {
		let parent_id = match mods.parent(id) {
			Some(parent_id) => parent_id,
			None => continue,
		};
		let leaf = *mods.get(id).name.last().unwrap();
		if mods.mods[parent_id.index()]
			.members
			.insert(leaf, Member::Child(id))
			.is_some()
		{
			errs.push(Error::DuplicateMember(
				sources.loc(mods.get(id).name_span),
				syms.resolve(leaf).to_string(),
			));
		}
	}

	errs
}

// For each module, resolve its imports of other modules or members of other
// modules.
fn resolve_imports(
	syms: &Interner,
	pkgs: &Packages,
	pkg_id: PackageId,
	sources: &Sources,
	chunks: &Chunks,
	mods: &mut Modules,
) -> Vec<Error> {
	let mut errs = Vec::new();

	for id in mods.ids() {
		let chunk = chunks.get(mods.get(id).chunk);
		let mut imports = Vec::new();
		for item_id in &chunk.top {
			let ModuleItem::Import(import) = chunk.get_module_item(*item_id) else {
				continue;
			};
			let span = chunk.get_module_item_span(*item_id);
			match resolve_import_path(mods, pkgs, pkg_id, &import.path) {
				Some((pkg, target)) => imports.push(Import { pkg, target, span }),
				None => errs.push(Error::UnknownImport(
					sources.loc(span),
					syms.resolve_path(&import.path),
				)),
			}
		}
		mods.mods[id.index()].imports = imports;
	}

	errs
}

// Resolve a dotted path to an import target. An import target can either be a
// module itself or a member of a module. Tries the local package first, then
// falls through to other packages in the arena.
fn resolve_import_path(
	mods: &Modules,
	pkgs: &Packages,
	pkg_id: PackageId,
	path: &[Sym],
) -> Option<(PackageId, Target)> {
	if let Some(found) = resolve_import_path_in(mods, path) {
		return Some((pkg_id, found));
	}
	for (other_id, other_pkg) in pkgs.iter() {
		if other_id == pkg_id {
			continue;
		}
		if let Some(found) = resolve_import_path_in(&other_pkg.modules, path) {
			return Some((other_id, found));
		}
	}
	None
}

fn resolve_import_path_in(mods: &Modules, path: &[Sym]) -> Option<Target> {
	if let Some(id) = mods.by_name.get(path) {
		return Some(Target::Module(*id));
	}
	let (last, prefix) = path.split_last()?;
	let id = *mods.by_name.get(prefix)?;
	if mods.get(id).members.contains_key(last) {
		return Some(Target::Member(id, *last));
	}
	None
}

// Where a module stands in the cycle-detection walk: never entered, entered and
// still on the path below the current one, or entered and left again.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Visit {
	Unseen,
	OnPath,
	Done,
}

// Walk the entire module graph, checking whether there are any import cycles.
// Every module starts a walk, since imports can leave a module unreachable
// from any other; the marks carry across walks, so a module reached by an
// earlier one costs nothing when its own turn comes.
fn check_import_cycles(
	syms: &Interner,
	sources: &Sources,
	chunks: &Chunks,
	mods: &Modules,
) -> Vec<Error> {
	let mut errs = Vec::new();
	let mut visits = vec![Visit::Unseen; mods.mods.len()];
	// The modules entered and not yet left, in the order they were entered,
	// which is what a cycle is rendered from.
	let mut path = Vec::new();
	for id in mods.ids() {
		walk_imports(
			syms,
			sources,
			chunks,
			mods,
			id,
			&mut visits,
			&mut path,
			&mut errs,
		);
	}
	errs
}

// Walk all imports of a module, checking whether there are any cycles. A DFS
// that marks each module on entry and unmarks it on exit, so an import to a
// module still marked is one that imports back into the path being walked.
fn walk_imports(
	syms: &Interner,
	sources: &Sources,
	chunks: &Chunks,
	mods: &Modules,
	id: ModuleId,
	visits: &mut Vec<Visit>,
	path: &mut Vec<ModuleId>,
	errs: &mut Vec<Error>,
) {
	// Reached only for a module already walked to completion: the loop below
	// never recurses into one still on the path, having reported it instead.
	if visits[id.index()] != Visit::Unseen {
		return;
	}
	visits[id.index()] = Visit::OnPath;
	path.push(id);

	for import in &mods.get(id).imports {
		if import.pkg != mods.pkg {
			continue;
		}
		let next = import.target.module();
		// If this module imports one still being walked, it's a cycle.
		if visits[next.index()] == Visit::OnPath {
			errs.push(Error::ImportCycle(
				sources.loc(import.span),
				render_cycle(syms, mods, path, next),
			));
			continue;
		}
		// Otherwise, continue walking the graph through the imported module.
		walk_imports(syms, sources, chunks, mods, next, visits, path, errs);
	}

	path.pop();
	visits[id.index()] = Visit::Done;
}

// Render a string representation of an import cycle: the modules from the one
// imported back into through to the one importing it, then that first module
// again, so that the cycle reads as a closed loop. The path can hold modules
// walked through before the cycle began, and those are dropped.
fn render_cycle(syms: &Interner, mods: &Modules, path: &[ModuleId], back: ModuleId) -> String {
	let start = path.iter().position(|id| *id == back).unwrap();
	let mut names: Vec<String> = path[start..]
		.iter()
		.map(|id| syms.resolve_path(&mods.get(*id).name))
		.collect();
	names.push(syms.resolve_path(&mods.get(back).name));
	names.join(" → ")
}

// Order every module for its top-level expression pass. Every non-root module
// starts its own walk, in ModuleId order; the root module starts last, unless
// an earlier walk already reached it as somebody's dependency, in which case it
// is already in the order and dependencies still precede dependents.
fn compute_order(mods: &Modules) -> Vec<ModuleId> {
	let root = mods.by_file[Path::new(ROOT_FILE)];
	let mut order = Vec::with_capacity(mods.mods.len());
	let mut visited = vec![false; mods.mods.len()];
	for id in mods.ids() {
		if id != root {
			walk_order(mods, id, &mut visited, &mut order);
		}
	}
	walk_order(mods, root, &mut visited, &mut order);
	order
}

// Post-order DFS over import edges, so a module's dependencies are always
// evaluated before it.
fn walk_order(mods: &Modules, id: ModuleId, visited: &mut [bool], order: &mut Vec<ModuleId>) {
	if visited[id.index()] {
		return;
	}
	visited[id.index()] = true;
	for import in &mods.get(id).imports {
		if import.pkg != mods.pkg {
			continue;
		}
		walk_order(mods, import.target.module(), visited, order);
	}
	order.push(id);
}

// The name that a top-level expression claims, if any.
fn bound_name(chunk: &Chunk, expr_id: ExprId) -> Option<Sym> {
	let assign = match chunk.get_expr(expr_id) {
		Expr::Assign(assign) => assign,
		_ => return None,
	};
	match &assign.place {
		Place::Name(name) => Some(name.sym),
		Place::Member(_) => None,
		Place::Access(_) => None,
	}
}
