use std::collections::HashMap;

use ordermap::OrderMap;

use crate::intern::{Interner, Sym};
use crate::load::{RESERVED_DIR, ROOT_FILE};
use crate::pkg::Package;
use crate::sem::Error;
use crate::src::{Location, Span};
use crate::syn::nodes::{Chunk, ChunkId, Expr, ExprId, ModuleItem, ModuleItemId, Place};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ModuleId(u32);

impl ModuleId {
	pub fn index(self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug)]
pub struct Modules {
	mods: Vec<Module>,
	by_file: HashMap<String, ModuleId>,
	by_name: HashMap<Vec<Sym>, ModuleId>,
	by_chunk: HashMap<ChunkId, ModuleId>,
	order: Vec<ModuleId>,
}

#[derive(Debug)]
pub struct Module {
	chunk: ChunkId,
	file: String,
	name: Vec<Sym>,
	name_span: Span,
	members: OrderMap<Sym, Member>,
	exports: Option<Vec<Sym>>,
	imports: Vec<Import>,
}

// A member of a module, named by the declaration that gives the module the
// member. Every item id indexes the arena of the module's own chunk, except a
// child's, which names the module that another file declares.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Member {
	Type(ModuleItemId),
	Proto(ModuleItemId),
	Proc(ModuleItemId),
	// A top-level 'x := ...', naming the assignment that first claimed the
	// name, since a later one reassigns rather than redeclares.
	Var(ModuleItemId),
	// A module declared in a subdirectory file, whose leaf segment is a member
	// of this one.
	Child(ModuleId),
}

// What a name means in the file declaring a module: a member of some module, or
// a module itself, which only an import can bind.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Binding {
	Member(ModuleId, Member),
	Module(ModuleId),
}

#[derive(Debug)]
struct Import {
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
		self.get(id).members.get(&name).copied()
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
	pub fn binding(&self, id: ModuleId, name: Sym) -> Option<Binding> {
		if let Some(member) = self.member(id, name) {
			match member {
				Member::Type(_) | Member::Proto(_) | Member::Proc(_) | Member::Var(_) => {
					return Some(Binding::Member(id, member));
				}
				// A child module is a member of this one but is declared by
				// another file, which this one no more sees into than any
				// other. Naming the child takes an import like anything else,
				// and that import binds it as a module rather than a member.
				Member::Child(_) => {}
			}
		}
		for import in &self.get(id).imports {
			match import.target {
				// A module-form import binds the module itself, under the leaf
				// of its declared name.
				Target::Module(owner) => {
					if *self.get(owner).name.last().unwrap() == name {
						return Some(Binding::Module(owner));
					}
				}
				// A member-form import binds that one member and nothing else
				// of the module holding it.
				Target::Member(owner, bound) => {
					if bound == name {
						return Some(Binding::Member(owner, self.member(owner, name)?));
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
		start: ModuleId,
		path: &'path [Sym],
	) -> Option<(ModuleId, Member, &'path [Sym])> {
		// The first part must be resolved relative to the current module's
		// *bindings*, but subsequent parts must be resolved relative to each
		// child module's *members*. Check the first part here, and the
		// subsequent parts separately.
		let (mut owner, mut member) = match self.binding(start, path[0]) {
			// The name is bound to a value in the current module (possibly an import).
			Some(Binding::Member(owner, member)) => (owner, member),
			// The name is bound to a module imported by the current module.
			Some(Binding::Module(id)) => (start, Member::Child(id)),
			None => return None,
		};
		// For the remaining parts, look them up relative to each resolved part
		// in turn.
		for (i, part) in path[1..].iter().enumerate() {
			let Member::Child(id) = member else {
				return Some((owner, member, &path[i + 1..]));
			};
			owner = id;
			member = self.member(owner, *part)?;
		}
		Some((owner, member, &[]))
	}

	fn parent(&self, id: ModuleId) -> Option<ModuleId> {
		let file = parent_file(&self.get(id).file)?;
		self.by_file.get(&file).copied()
	}
}

// Check all invariants about the module graph, returning as many errors as
// possible.
pub fn check(syms: &Interner, pkg: &Package, dirs: &[String]) -> Result<Modules, Vec<Error>> {
	let files: Vec<String> = pkg.chunk_ids().map(|id| pkg.file(id).to_string()).collect();
	let mut errs = check_sibling_files_exist(dirs, &files);

	let mut mods = match build(syms, pkg) {
		Ok(mods) => mods,
		Err(mut header_errs) => {
			// If there are errors in the module headers, checking them won't
			// result in anything sensible; error early.
			errs.append(&mut header_errs);
			return Err(errs);
		}
	};

	errs.append(&mut check_child_prefixes_match(syms, pkg, &mods));
	errs.append(&mut check_members_unique(syms, pkg, &mut mods));

	// Resolution reads every module's members, so it runs after they are
	// filled. Cycles are a property of the resolved edges, so they run after
	// that.
	errs.append(&mut resolve_imports(syms, pkg, &mut mods));
	errs.append(&mut check_import_cycles(syms, pkg, &mods));

	if !errs.is_empty() {
		return Err(errs);
	}

	// The order walk needs to know that the import graph is acyclic, so it runs
	// after.
	mods.order = compute_order(&mods);

	Ok(mods)
}

// Build the *incomplete* module graph. Later passes will flesh out its data.
fn build(syms: &Interner, pkg: &Package) -> Result<Modules, Vec<Error>> {
	let mut mods = Vec::new();
	let mut by_file = HashMap::new();
	let mut by_name = HashMap::new();
	let mut by_chunk = HashMap::new();
	let mut errs = Vec::new();

	for chunk_id in pkg.chunk_ids() {
		let file = pkg.file(chunk_id).to_string();
		let (name, name_span) = match read_header(pkg, chunk_id, &file) {
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
				pkg.loc(name_span),
				syms.resolve_path(&name),
			));
		}
		mods.push(Module {
			chunk: chunk_id,
			file,
			name,
			name_span,
			members: OrderMap::new(),
			exports: read_exports(pkg.get_chunk(chunk_id)),
			imports: Vec::new(),
		});
	}

	if !errs.is_empty() {
		return Err(errs);
	}

	Ok(Modules {
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
	pkg: &Package,
	chunk_id: ChunkId,
	file: &str,
) -> Result<(Vec<Sym>, Span), Vec<Error>> {
	let chunk = pkg.get_chunk(chunk_id);
	let mut header = None;
	let mut errs = Vec::new();

	for (i, item_id) in chunk.top.iter().enumerate() {
		let module = match chunk.get_module_item(*item_id) {
			ModuleItem::Module(module) => module,
			_ => continue,
		};
		let span = chunk.get_module_item_span(*item_id);
		if header.is_some() {
			errs.push(Error::DuplicateModuleHeader(pkg.loc(span)));
			continue;
		}
		if i != 0 {
			errs.push(Error::MisplacedModuleHeader(pkg.loc(span)));
		}
		header = Some((module.path.clone(), span));
	}

	if header.is_none() {
		errs.push(Error::MissingModuleHeader(Location::file(file.to_string())));
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
fn check_sibling_files_exist(dirs: &[String], files: &[String]) -> Vec<Error> {
	let mut errs = Vec::new();
	for dir in dirs {
		if dir == RESERVED_DIR {
			errs.push(Error::ReservedDirectory(Location::file(dir.clone())));
			continue;
		}
		let sibling = format!("{}.ms", dir);
		if !files.iter().any(|file| *file == sibling) {
			errs.push(Error::UnpairedDirectory(Location::file(dir.clone())));
		}
	}
	errs
}

// For every module in a subdirectory, check that the prefix of its module path
// matches the module path declared by the parent file which is the sibling of
// that directory.
fn check_child_prefixes_match(syms: &Interner, pkg: &Package, mods: &Modules) -> Vec<Error> {
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
			pkg.loc(module.name_span),
			syms.resolve_path(&module.name),
			syms.resolve_path(&parent.name),
		));
	}
	errs
}

// For every member in a module—which *includes* direct submodules—check that it
// does not duplicate the name of another member. While checking, store the
// (verified unique) members in the module map for later phases.
fn check_members_unique(syms: &Interner, pkg: &Package, mods: &mut Modules) -> Vec<Error> {
	let mut errs = Vec::new();

	for id in mods.ids() {
		let chunk = pkg.get_chunk(mods.get(id).chunk);
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
					pkg.loc(span),
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
				pkg.loc(mods.get(id).name_span),
				syms.resolve(leaf).to_string(),
			));
		}
	}

	errs
}

// For each module, resolve its imports of other modules or members of other
// modules.
fn resolve_imports(syms: &Interner, pkg: &Package, mods: &mut Modules) -> Vec<Error> {
	let mut errs = Vec::new();

	for id in mods.ids() {
		let chunk = pkg.get_chunk(mods.get(id).chunk);
		let mut imports = Vec::new();
		for item_id in &chunk.top {
			let ModuleItem::Import(import) = chunk.get_module_item(*item_id) else {
				continue;
			};
			let span = chunk.get_module_item_span(*item_id);
			match resolve_import_path(mods, &import.path) {
				Some(target) => imports.push(Import { target, span }),
				None => errs.push(Error::UnknownImport(
					pkg.loc(span),
					syms.resolve_path(&import.path),
				)),
			}
		}
		mods.mods[id.index()].imports = imports;
	}

	errs
}

// Resolve a dotted path to an import target. An import target can either be a
// module itself or a member of a module.
fn resolve_import_path(mods: &Modules, path: &[Sym]) -> Option<Target> {
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
fn check_import_cycles(syms: &Interner, pkg: &Package, mods: &Modules) -> Vec<Error> {
	let mut errs = Vec::new();
	let mut visits = vec![Visit::Unseen; mods.mods.len()];
	// The modules entered and not yet left, in the order they were entered,
	// which is what a cycle is rendered from.
	let mut path = Vec::new();
	for id in mods.ids() {
		walk_imports(syms, pkg, mods, id, &mut visits, &mut path, &mut errs);
	}
	errs
}

// Walk all imports of a module, checking whether there are any cycles. A DFS
// that marks each module on entry and unmarks it on exit, so an import to a
// module still marked is one that imports back into the path being walked.
fn walk_imports(
	syms: &Interner,
	pkg: &Package,
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
		let next = import.target.module();
		// If this module imports one still being walked, it's a cycle.
		if visits[next.index()] == Visit::OnPath {
			errs.push(Error::ImportCycle(
				pkg.loc(import.span),
				render_cycle(syms, mods, path, next),
			));
			continue;
		}
		// Otherwise, continue walking the graph through the imported module.
		walk_imports(syms, pkg, mods, next, visits, path, errs);
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
	let root = mods.by_file[ROOT_FILE];
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

// The file that the parent module should be in, if any.
fn parent_file(file: &str) -> Option<String> {
	if file == ROOT_FILE {
		return None;
	}
	let stem = file.strip_suffix(".ms")?;
	match stem.rsplit_once('/')? {
		("src", _) => Some(ROOT_FILE.to_string()),
		(dir, _) => Some(format!("{}.ms", dir)),
	}
}
