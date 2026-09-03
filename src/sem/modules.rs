use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ordermap::OrderMap;

use crate::intern::{Interner, Sym};
use crate::load::{RESERVED_DIR, ROOT_FILE};
use crate::pkg::{PackageId, Packages};
use crate::sem::{Error, Visit};
use crate::src::{Location, Span};
use crate::syn::nodes::{Expr, ExprId, ModuleItem, ModuleItemId, Place};
use crate::syn::{Chunk, ChunkId};

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
	modules: Vec<Module>,
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
		&self.modules[id.index()]
	}

	pub fn pkg(&self) -> PackageId {
		self.pkg
	}

	pub fn ids(&self) -> impl Iterator<Item = ModuleId> + use<> {
		(0..self.modules.len() as u32).map(ModuleId)
	}

	pub fn chunk(&self, id: ModuleId) -> ChunkId {
		self.get(id).chunk
	}

	pub fn by_chunk(&self, chunk: ChunkId) -> ModuleId {
		self.by_chunk[&chunk]
	}

	// A module by its declared dotted path.
	pub fn by_path(&self, path: &[Sym]) -> Option<ModuleId> {
		self.by_name.get(path).copied()
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
					if *pkgs
						.get(import.pkg)
						.modules()
						.get(owner)
						.name
						.last()
						.unwrap() == name
					{
						return Some(Binding::Module(import.pkg, owner));
					}
				}
				// A member-form import binds that one member and nothing else
				// of the module holding it.
				Target::Member(owner, bound) => {
					if bound == name {
						return Some(Binding::Imported(
							import.pkg,
							owner,
							pkgs.get(import.pkg).modules().member(owner, name)?,
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
		// For the remaining parts, look them up relative to each resolved part
		// in turn.
		for (i, part) in path[1..].iter().enumerate() {
			let Member::Child(id) = member else {
				return Some(Resolved::Partial(pkg, owner, member, &path[i + 1..]));
			};
			owner = id;
			member = pkgs.get(pkg).modules().member(owner, *part)?;
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
pub fn check(syms: &Interner, pkgs: &Packages, pkg_id: PackageId) -> Result<Modules, Vec<Error>> {
	let mut errs = check_sibling_files_exist(pkgs, pkg_id);

	let mut modules = match build(syms, pkgs, pkg_id) {
		Ok(modules) => modules,
		Err(mut header_errs) => {
			// If there are errors in the module headers, checking them won't
			// result in anything sensible; error early.
			errs.append(&mut header_errs);
			return Err(errs);
		}
	};

	errs.append(&mut check_child_prefixes_match(
		syms, pkgs, pkg_id, &modules,
	));
	errs.append(&mut check_members_unique(syms, pkgs, pkg_id, &mut modules));

	// Resolution reads every module's members, so it runs after they are
	// filled. Cycles are a property of the resolved edges, so they run after
	// that.
	errs.append(&mut resolve_imports(syms, pkgs, pkg_id, &mut modules));
	errs.append(&mut check_import_cycles(syms, pkgs, pkg_id, &modules));

	if !errs.is_empty() {
		return Err(errs);
	}

	// The order walk needs to know that the import graph is acyclic, so it runs
	// after.
	modules.order = compute_order(&modules);

	Ok(modules)
}

// Build the *incomplete* module graph. Later passes will flesh out its data.
fn build(syms: &Interner, pkgs: &Packages, pkg_id: PackageId) -> Result<Modules, Vec<Error>> {
	let pkg = pkgs.get(pkg_id);
	let mut modules = Vec::new();
	let mut by_file = HashMap::new();
	let mut by_name = HashMap::new();
	let mut by_chunk = HashMap::new();
	let mut errs = Vec::new();

	for chunk_id in pkg.chunks().ids() {
		let chunk = pkg.chunks().get(chunk_id);
		let file = pkg.sources().get(chunk.source).file().to_path_buf();
		let (name, name_span) = match read_header(pkgs, pkg_id, chunk_id) {
			Ok(header) => header,
			Err(mut header_errs) => {
				errs.append(&mut header_errs);
				continue;
			}
		};
		let id = ModuleId(modules.len() as u32);
		by_file.insert(file.clone(), id);
		by_chunk.insert(chunk_id, id);
		// Nothing ties a module's last segment to its file name, so two files
		// can declare one path; error only when encountering a duplicate
		// *declared* name.
		if by_name.insert(name.clone(), id).is_some() {
			errs.push(Error::DuplicateModuleName(
				pkg.sources().loc(name_span),
				syms.resolve_path(&name),
			));
		}
		for (other_id, other_pkg) in pkgs.iter() {
			if other_id == pkg_id {
				continue;
			}
			if other_pkg.modules().by_name.contains_key(&name) {
				errs.push(Error::ConflictingModuleName(
					pkg.sources().loc(name_span),
					syms.resolve_path(&name),
					other_pkg.manifest().name.clone(),
				));
			}
		}
		modules.push(Module {
			chunk: chunk_id,
			file,
			name,
			name_span,
			members: OrderMap::new(),
			exports: read_exports(pkg.chunks().get(chunk_id)),
			imports: Vec::new(),
		});
	}

	if !errs.is_empty() {
		return Err(errs);
	}

	Ok(Modules {
		pkg: pkg_id,
		modules,
		by_file,
		by_name,
		by_chunk,
		order: Vec::new(),
	})
}

// Determine the module path that the chunk declares by reading the 'module'
// header. Error if it is missing, duplicated, or otherwise misplaced.
fn read_header(
	pkgs: &Packages,
	pkg_id: PackageId,
	chunk_id: ChunkId,
) -> Result<(Vec<Sym>, Span), Vec<Error>> {
	let pkg = pkgs.get(pkg_id);
	let chunk = pkg.chunks().get(chunk_id);
	let mut header = None;
	let mut errs = Vec::new();

	for (i, item_id) in chunk.top.iter().enumerate() {
		let module = match chunk.get_module_item(*item_id) {
			ModuleItem::Module(module) => module,
			_ => continue,
		};
		let span = chunk.get_module_item_span(*item_id);
		if header.is_some() {
			errs.push(Error::DuplicateModuleHeader(pkg.sources().loc(span)));
			continue;
		}
		if i != 0 {
			errs.push(Error::MisplacedModuleHeader(pkg.sources().loc(span)));
		}
		header = Some((module.path.clone(), span));
	}

	if header.is_none() {
		let file = pkg.sources().get(chunk.source).file();
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
fn check_sibling_files_exist(pkgs: &Packages, pkg_id: PackageId) -> Vec<Error> {
	let pkg = pkgs.get(pkg_id);
	let mut errs = Vec::new();
	for dir in pkg.sources().dirs() {
		if dir == Path::new(RESERVED_DIR) {
			errs.push(Error::ReservedDirectory(Location::file(dir.clone())));
			continue;
		} else if dir.file_name().unwrap() == "src" {
			continue;
		} else {
			let sibling = dir.with_extension("ms");
			if !pkg.sources().files().any(|file| file == &sibling) {
				errs.push(Error::UnpairedDirectory(Location::file(dir.clone())));
			}
		}
	}
	errs
}

// For every module in a subdirectory, check that the prefix of its module path
// matches the module path declared by the parent file which is the sibling of
// that directory.
fn check_child_prefixes_match(
	syms: &Interner,
	pkgs: &Packages,
	pkg_id: PackageId,
	modules: &Modules,
) -> Vec<Error> {
	let pkg = pkgs.get(pkg_id);
	let mut errs = Vec::new();
	for id in modules.ids() {
		let parent = match modules.parent(id) {
			Some(parent) => modules.get(parent),
			None => continue,
		};
		let module = modules.get(id);
		if module.name.len() == parent.name.len() + 1
			&& module.name[..parent.name.len()] == parent.name
		{
			continue;
		}
		errs.push(Error::PrefixMismatch(
			pkg.sources().loc(module.name_span),
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
	pkgs: &Packages,
	pkg_id: PackageId,
	modules: &mut Modules,
) -> Vec<Error> {
	let pkg = pkgs.get(pkg_id);
	let mut errs = Vec::new();

	for id in modules.ids() {
		let chunk = pkg.chunks().get(modules.get(id).chunk);
		let mut members = OrderMap::new();
		for item_id in &chunk.top {
			let span = chunk.get_module_item_span(*item_id);
			let (name, member) = match chunk.get_module_item(*item_id) {
				ModuleItem::Module(_) => continue,
				ModuleItem::Import(_) => continue,
				ModuleItem::Export(_) => continue,
				ModuleItem::Type(type_) => (type_.name, Member::Type(*item_id)),
				ModuleItem::Extern(extern_) => (extern_.name, Member::Type(*item_id)),
				ModuleItem::Proto(proto) => (proto.name, Member::Proto(*item_id)),
				ModuleItem::Def(def_id) => (chunk.get_def(*def_id).name, Member::Proc(*item_id)),
				ModuleItem::Expr(expr_id) => match bound_name(chunk, *expr_id) {
					Some(name) => {
						match members.get(&name) {
							None => {
								members.insert(name, Member::Var(*item_id));
							}
							// A second 'x := ...' is reassignment, not
							// redeclaration; the member keeps naming the
							// assignment that first claimed the name.
							Some(Member::Var(_)) => {}
							// A 'def', 'type', or 'proto' already claims the
							// name, and an assignment cannot displace it.
							Some(_) => errs.push(Error::DuplicateMember(
								pkg.sources().loc(span),
								syms.resolve(name).to_string(),
							)),
						}
						continue;
					}
					None => continue,
				},
			};
			if members.insert(name, member).is_some() {
				errs.push(Error::DuplicateMember(
					pkg.sources().loc(span),
					syms.resolve(name).to_string(),
				));
			}
		}
		modules.modules[id.index()].members = members;
	}

	// A child module's name is a member of its parent, so it collides with the
	// parent's declarations even though the two live in different files.
	for id in modules.ids() {
		let parent_id = match modules.parent(id) {
			Some(parent_id) => parent_id,
			None => continue,
		};
		let leaf = *modules.get(id).name.last().unwrap();
		if modules.modules[parent_id.index()]
			.members
			.insert(leaf, Member::Child(id))
			.is_some()
		{
			errs.push(Error::DuplicateMember(
				pkg.sources().loc(modules.get(id).name_span),
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
	modules: &mut Modules,
) -> Vec<Error> {
	let pkg = pkgs.get(pkg_id);
	let mut errs = Vec::new();

	for id in modules.ids() {
		let chunk = pkg.chunks().get(modules.get(id).chunk);
		let mut imports = Vec::new();
		for item_id in &chunk.top {
			let ModuleItem::Import(import) = chunk.get_module_item(*item_id) else {
				continue;
			};
			let span = chunk.get_module_item_span(*item_id);
			match resolve_import_path(modules, pkgs, pkg_id, &import.path) {
				Some((pkg, target)) => imports.push(Import { pkg, target, span }),
				None => errs.push(Error::UnknownImport(
					pkg.sources().loc(span),
					syms.resolve_path(&import.path),
				)),
			}
		}
		modules.modules[id.index()].imports = imports;
	}

	errs
}

// Resolve a dotted path to an import target. An import target can either be a
// module itself or a member of a module. Tries the local package first, then
// falls through to other packages in the arena.
fn resolve_import_path(
	modules: &Modules,
	pkgs: &Packages,
	pkg_id: PackageId,
	path: &[Sym],
) -> Option<(PackageId, Target)> {
	if let Some(found) = resolve_import_path_in(modules, path) {
		return Some((pkg_id, found));
	}
	for (other_id, other_pkg) in pkgs.iter() {
		if other_id == pkg_id {
			continue;
		}
		if let Some(found) = resolve_import_path_in(other_pkg.modules(), path) {
			return Some((other_id, found));
		}
	}
	None
}

fn resolve_import_path_in(modules: &Modules, path: &[Sym]) -> Option<Target> {
	if let Some(id) = modules.by_name.get(path) {
		return Some(Target::Module(*id));
	}
	let (last, prefix) = path.split_last()?;
	let id = *modules.by_name.get(prefix)?;
	if modules.get(id).members.contains_key(last) {
		return Some(Target::Member(id, *last));
	}
	None
}

// Walk the entire module graph, checking whether there are any import cycles.
// Every module starts a walk, since imports can leave a module unreachable
// from any other; the marks carry across walks, so a module reached by an
// earlier one costs nothing when its own turn comes.
fn check_import_cycles(
	syms: &Interner,
	pkgs: &Packages,
	pkg_id: PackageId,
	modules: &Modules,
) -> Vec<Error> {
	let mut errs = Vec::new();
	let mut visits = vec![Visit::Unseen; modules.modules.len()];
	// The modules entered and not yet left, in the order they were entered,
	// which is what a cycle is rendered from.
	let mut path = Vec::new();
	for id in modules.ids() {
		walk_imports(
			syms,
			pkgs,
			pkg_id,
			modules,
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
	pkgs: &Packages,
	pkg_id: PackageId,
	modules: &Modules,
	id: ModuleId,
	visits: &mut Vec<Visit>,
	path: &mut Vec<ModuleId>,
	errs: &mut Vec<Error>,
) {
	let pkg = pkgs.get(pkg_id);
	// Reached only for a module already walked to completion: the loop below
	// never recurses into one still on the path, having reported it instead.
	if visits[id.index()] != Visit::Unseen {
		return;
	}
	visits[id.index()] = Visit::OnPath;
	path.push(id);

	for import in &modules.get(id).imports {
		if import.pkg != modules.pkg {
			continue;
		}
		let next = import.target.module();
		// If this module imports one still being walked, it's a cycle.
		if visits[next.index()] == Visit::OnPath {
			errs.push(Error::ImportCycle(
				pkg.sources().loc(import.span),
				render_cycle(syms, modules, path, next),
			));
			continue;
		}
		// Otherwise, continue walking the graph through the imported module.
		walk_imports(syms, pkgs, pkg_id, modules, next, visits, path, errs);
	}

	path.pop();
	visits[id.index()] = Visit::Done;
}

// Render a string representation of an import cycle: the modules from the one
// imported back into through to the one importing it, then that first module
// again, so that the cycle reads as a closed loop. The path can hold modules
// walked through before the cycle began, and those are dropped.
fn render_cycle(syms: &Interner, modules: &Modules, path: &[ModuleId], back: ModuleId) -> String {
	let start = path.iter().position(|id| *id == back).unwrap();
	let mut names: Vec<String> = path[start..]
		.iter()
		.map(|id| syms.resolve_path(&modules.get(*id).name))
		.collect();
	names.push(syms.resolve_path(&modules.get(back).name));
	names.join(" → ")
}

// Order every module for its top-level expression pass. Every non-root module
// starts its own walk, in ModuleId order; the root module starts last, unless
// an earlier walk already reached it as somebody's dependency, in which case it
// is already in the order and dependencies still precede dependents.
fn compute_order(modules: &Modules) -> Vec<ModuleId> {
	let root = modules.by_file[Path::new(ROOT_FILE)];
	let mut order = Vec::with_capacity(modules.modules.len());
	let mut visited = vec![false; modules.modules.len()];
	for id in modules.ids() {
		if id != root {
			walk_order(modules, id, &mut visited, &mut order);
		}
	}
	walk_order(modules, root, &mut visited, &mut order);
	order
}

// Post-order DFS over import edges, so a module's dependencies are always
// evaluated before it.
fn walk_order(modules: &Modules, id: ModuleId, visited: &mut [bool], order: &mut Vec<ModuleId>) {
	if visited[id.index()] {
		return;
	}
	visited[id.index()] = true;
	for import in &modules.get(id).imports {
		if import.pkg != modules.pkg {
			continue;
		}
		walk_order(modules, import.target.module(), visited, order);
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
