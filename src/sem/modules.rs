use std::collections::HashMap;

use ordermap::OrderMap;

use crate::intern::{Interner, Sym};
use crate::pkg::Package;
use crate::sem::Error;
use crate::sem::load::{RESERVED_DIR, ROOT_FILE};
use crate::src::{Location, Span};
use crate::syn::{Chunk, ChunkId, Expr, ExprId, ModuleItem, ModuleItemId, Place};

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
				dotted_path_to_string(syms, &name),
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
			dotted_path_to_string(syms, &module.name),
			dotted_path_to_string(syms, &parent.name),
		));
	}
	errs
}

fn dotted_path_to_string(syms: &Interner, path: &[Sym]) -> String {
	path.iter()
		.map(|seg| syms.resolve(*seg))
		.collect::<Vec<_>>()
		.join(".")
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
			let import = match chunk.get_module_item(*item_id) {
				ModuleItem::Import(import) => import,
				_ => continue,
			};
			let span = chunk.get_module_item_span(*item_id);
			match resolve_path(mods, &import.path) {
				Some(target) => imports.push(Import { target, span }),
				None => errs.push(Error::UnknownImport(
					pkg.loc(span),
					dotted_path_to_string(syms, &import.path),
				)),
			}
		}
		mods.mods[id.index()].imports = imports;
	}

	errs
}

// Resolve a dotted path to an import target. An import target can either be a
// module itself or a member of a module.
fn resolve_path(mods: &Modules, path: &[Sym]) -> Option<Target> {
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
		.map(|id| dotted_path_to_string(syms, &mods.get(*id).name))
		.collect();
	names.push(dotted_path_to_string(syms, &mods.get(back).name));
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

#[cfg(test)]
mod tests {
	use super::*;

	use crate::sem::load;

	// A package built from literal source text, which is what keeps every
	// family below testable without a temp dir.
	fn package(files: &[(&str, &str)]) -> (Interner, Package) {
		let mut syms = Interner::new();
		let files = files
			.iter()
			.map(|(file, text)| (file.to_string(), text.to_string()))
			.collect();
		let (pkg, _) = load::parse(&mut syms, files).unwrap();
		(syms, pkg)
	}

	fn rendered(errs: Vec<Error>) -> Vec<String> {
		errs.iter().map(|err| err.to_string()).collect()
	}

	fn layout(dirs: &[&str], files: &[&str]) -> Vec<String> {
		let dirs: Vec<String> = dirs.iter().map(|dir| dir.to_string()).collect();
		let files: Vec<String> = files.iter().map(|file| file.to_string()).collect();
		check_sibling_files_exist(&dirs, &files)
			.iter()
			.map(|err| err.to_string())
			.collect()
	}

	#[test]
	fn test_pairs_directories_with_siblings() {
		// A paired directory at either depth is well-formed, and a file with no
		// directory is an ordinary leaf module.
		assert!(
			layout(
				&["src/codec", "src/codec/json"],
				&[
					ROOT_FILE,
					"src/codec.ms",
					"src/codec/json.ms",
					"src/util.ms",
				],
			)
			.is_empty()
		);

		// Unpaired directories report in path order, and src/package/ is
		// reserved even though src/package.ms pairs with it.
		assert_eq!(
			vec![
				"src/a: semantic error: directory has no sibling module file",
				"src/b: semantic error: directory has no sibling module file",
				"src/codec/json: semantic error: directory has no sibling module file",
				"src/package: semantic error: 'src/package/' is reserved; the root module's children live in src/",
			],
			layout(
				&[
					"src/a",
					"src/b",
					"src/codec",
					"src/codec/json",
					"src/package"
				],
				&[ROOT_FILE, "src/codec.ms"],
			)
		);
	}

	#[test]
	fn test_reports_duplicate_module_names() {
		// Two files declaring one path satisfy the prefix and member checks, so
		// the duplicate reports at the second header.
		let (syms, pkg) = package(&[
			(ROOT_FILE, "module Test\n"),
			("src/a.ms", "module Test.X\ndef one()\nend\n"),
			("src/b.ms", "module Test.X\ndef two()\nend\n"),
		]);
		assert_eq!(
			vec!["src/b.ms:1,1: semantic error: duplicate module 'Test.X'"],
			rendered(build(&syms, &pkg).unwrap_err())
		);
	}

	#[test]
	fn test_checks_prefixes_against_the_parent() {
		// A well-formed tree at two depths, and the root checked against
		// nothing even though it declares a dotted path.
		let (syms, pkg) = package(&[
			(ROOT_FILE, "module Test\n"),
			("src/codec.ms", "module Test.Codec\n"),
			("src/codec/json.ms", "module Test.Codec.Json\n"),
		]);
		let mods = build(&syms, &pkg).unwrap();
		assert!(check_child_prefixes_match(&syms, &pkg, &mods).is_empty());

		// A wrong prefix, a missing one, and a path one segment too long. Each
		// reports at the offending header, in path order.
		let (syms, pkg) = package(&[
			(ROOT_FILE, "module Test\n"),
			("src/codec.ms", "module Test.Codec\n"),
			("src/codec/json.ms", "module Test.Json\n"),
			("src/codec/rpc.ms", "module Test.Codec.Rpc.Inner\n"),
			("src/util.ms", "module Util\n"),
		]);
		let mods = build(&syms, &pkg).unwrap();
		assert_eq!(
			vec![
				"src/codec/json.ms:1,1: semantic error: module 'Test.Json' must be declared under 'Test.Codec'",
				"src/codec/rpc.ms:1,1: semantic error: module 'Test.Codec.Rpc.Inner' must be declared under 'Test.Codec'",
				"src/util.ms:1,1: semantic error: module 'Util' must be declared under 'Test'",
			],
			rendered(check_child_prefixes_match(&syms, &pkg, &mods))
		);
	}

	#[test]
	fn test_describes_every_member() {
		// One of each kind a module can hold, with the child contributed by a
		// second file, and a rebinding that must not claim the key a second
		// time or displace the description of the first.
		let (syms, pkg) = package(&[
			(ROOT_FILE, "module Test\n"),
			(
				"src/codec.ms",
				"module Test.Codec\ntype Frame\nend\nproto Sized\ndef size()\nend\nend\ndef read()\nend\nlimit := 8\nlimit := 9\n",
			),
			("src/codec/json.ms", "module Test.Codec.Json\n"),
		]);
		let mut mods = build(&syms, &pkg).unwrap();
		assert!(check_members_unique(&syms, &pkg, &mut mods).is_empty());

		let json = mods.by_file["src/codec/json.ms"];
		let codec = mods.get(mods.by_file["src/codec.ms"]);
		let chunk = pkg.get_chunk(codec.chunk);
		let named: Vec<(&str, Member)> = codec
			.members
			.iter()
			.map(|(name, member)| (syms.resolve(*name), *member))
			.collect();

		// The four declared in this file name items in its own arena, in source
		// order, and the child names the module a second file declares.
		assert_eq!(
			vec![
				("Frame", Member::Type(chunk.top[1])),
				("Sized", Member::Proto(chunk.top[2])),
				("read", Member::Proc(chunk.top[3])),
				("limit", Member::Var(chunk.top[4])),
				("Json", Member::Child(json)),
			],
			named
		);
	}

	// Import resolution reads members, so a package under test is built through
	// the member pass before its imports are resolved.
	fn resolvable(files: &[(&str, &str)]) -> (Interner, Package, Modules) {
		let (syms, pkg) = package(files);
		let mut mods = build(&syms, &pkg).unwrap();
		assert!(check_members_unique(&syms, &pkg, &mut mods).is_empty());
		(syms, pkg, mods)
	}

	#[test]
	fn test_resolves_both_import_forms() {
		let (syms, pkg, mut mods) = resolvable(&[
			(ROOT_FILE, "module Test\n"),
			("src/protos.ms", "module Test.Protos\ndef helper()\nend\n"),
			(
				"src/ring.ms",
				"module Test.Ring\nimport Test.Protos\nimport Test.Protos.helper\n",
			),
		]);
		assert!(resolve_imports(&syms, &pkg, &mut mods).is_empty());

		let protos = mods.by_file["src/protos.ms"];
		let ring = mods.by_file["src/ring.ms"];
		let targets = &mods.get(ring).imports;
		assert_eq!(2, targets.len());
		match targets[0].target {
			Target::Module(id) => assert_eq!(protos, id),
			Target::Member(..) => panic!(),
		}
		match targets[1].target {
			Target::Module(_) => panic!(),
			Target::Member(id, name) => {
				assert_eq!(protos, id);
				assert_eq!("helper", syms.resolve(name));
			}
		}
	}

	#[test]
	fn test_binds_names_a_module_can_see() {
		let (syms, pkg, mut mods) = resolvable(&[
			(ROOT_FILE, "module Test\n"),
			(
				"src/protos.ms",
				"module Test.Protos\nproto Order\nend\ndef helper()\nend\n",
			),
			(
				"src/ring.ms",
				"module Test.Ring\nimport Test.Protos.Order\ntype Ring\nend\n",
			),
			("src/blind.ms", "module Test.Blind\nimport Test.Protos\n"),
		]);
		assert!(resolve_imports(&syms, &pkg, &mut mods).is_empty());

		let protos = mods.by_file["src/protos.ms"];
		let ring = mods.by_file["src/ring.ms"];
		let blind = mods.by_file["src/blind.ms"];
		// The names to ask about, and the items they should resolve to: the
		// protocol and proc of one file, and the type of another.
		let order = *mods.get(protos).members.get_index(0).unwrap().0;
		let helper = *mods.get(protos).members.get_index(1).unwrap().0;
		let ring_type = *mods.get(ring).members.get_index(0).unwrap().0;
		let proto_item = pkg.get_chunk(mods.get(protos).chunk).top[1];
		let type_item = pkg.get_chunk(mods.get(ring).chunk).top[2];

		// A module sees what it declares, and the declaring module comes back
		// with it even when that is the asking module itself.
		assert_eq!(
			Some(Binding::Member(protos, Member::Proto(proto_item))),
			mods.binding(protos, order)
		);
		assert_eq!(
			Some(Binding::Member(ring, Member::Type(type_item))),
			mods.binding(ring, ring_type)
		);

		// It sees a name a member-form import binds, described by the module
		// that declared it rather than the one importing it.
		assert_eq!(
			Some(Binding::Member(protos, Member::Proto(proto_item))),
			mods.binding(ring, order)
		);

		// It does not see the rest of that module, nor anything it never
		// imported at all.
		assert_eq!(None, mods.binding(ring, helper));
		assert_eq!(None, mods.binding(protos, ring_type));

		// A module-form import binds the module itself, under its leaf name, and
		// the members of that module stay invisible.
		let protos_leaf = *mods.get(protos).name.last().unwrap();
		assert_eq!(
			Some(Binding::Module(protos)),
			mods.binding(blind, protos_leaf)
		);
		assert_eq!(None, mods.binding(blind, order));
		assert_eq!(None, mods.binding(blind, helper));

		// And it binds that name only where it was written: Ring imported a
		// member of Protos, not Protos itself.
		assert_eq!(None, mods.binding(ring, protos_leaf));

		// Binding a name is not holding it out as a member: Ring can write
		// 'Order' but Test.Ring.Order reaches nothing.
		assert_eq!(Some(Member::Proto(proto_item)), mods.member(protos, order));
		assert_eq!(None, mods.member(ring, order));
	}

	#[test]
	fn test_binds_child_modules_only_when_imported() {
		// A parent that imports its own child, a parent that does not, and an
		// unrelated module importing the same child.
		let (syms, pkg, mut mods) = resolvable(&[
			(ROOT_FILE, "module Test\n"),
			("src/json.ms", "module Test.Json\nimport Test.Json.Codec\n"),
			("src/json/codec.ms", "module Test.Json.Codec\n"),
		]);
		assert!(resolve_imports(&syms, &pkg, &mut mods).is_empty());

		let json = mods.by_file["src/json.ms"];
		let codec = mods.by_file["src/json/codec.ms"];
		let leaf = *mods.get(codec).name.last().unwrap();

		// The child is a member of its parent, which is what a dotted path
		// reaches and what makes the two share one namespace.
		assert_eq!(Some(Member::Child(codec)), mods.member(json, leaf));

		// Being a member does not put it in scope: the parent's file declares
		// nothing called Codec, so it takes an import like any other module,
		// and binds as a module rather than as the member it also is.
		assert_eq!(Some(Binding::Module(codec)), mods.binding(json, leaf));
		let (syms, pkg, mut mods) = resolvable(&[
			(ROOT_FILE, "module Test\n"),
			("src/json.ms", "module Test.Json\n"),
			("src/json/codec.ms", "module Test.Json.Codec\n"),
		]);
		assert!(resolve_imports(&syms, &pkg, &mut mods).is_empty());
		let json_alone = mods.by_file["src/json.ms"];
		let codec_alone = mods.by_file["src/json/codec.ms"];
		let leaf_alone = *mods.get(codec_alone).name.last().unwrap();
		assert_eq!(
			Some(Member::Child(codec_alone)),
			mods.member(json_alone, leaf_alone)
		);
		assert_eq!(None, mods.binding(json_alone, leaf_alone));
	}

	#[test]
	fn test_binds_a_childs_module_to_unrelated_importers() {
		// Containment is nobody else's business: a module importing another's
		// child binds it exactly as its parent would have to.
		let (syms, pkg, mut mods) = resolvable(&[
			(ROOT_FILE, "module Test\n"),
			("src/json.ms", "module Test.Json\n"),
			("src/json/codec.ms", "module Test.Json.Codec\n"),
			("src/plain.ms", "module Test.Plain\n"),
			("src/app.ms", "module Test.App\nimport Test.Json.Codec\n"),
		]);
		assert!(resolve_imports(&syms, &pkg, &mut mods).is_empty());

		let codec = mods.by_file["src/json/codec.ms"];
		let app = mods.by_file["src/app.ms"];
		let plain = mods.by_file["src/plain.ms"];
		let leaf = *mods.get(codec).name.last().unwrap();

		assert_eq!(Some(Binding::Module(codec)), mods.binding(app, leaf));
		assert_eq!(None, mods.binding(plain, leaf));
		assert_eq!(None, mods.member(app, leaf));
	}

	#[test]
	fn test_reports_unresolvable_imports() {
		// A path naming no module, and one whose prefix is a module but whose
		// last segment is not one of its members.
		let (syms, pkg, mut mods) = resolvable(&[
			(ROOT_FILE, "module Test\n"),
			("src/protos.ms", "module Test.Protos\ndef helper()\nend\n"),
			(
				"src/ring.ms",
				"module Test.Ring\nimport Test.Missing\nimport Test.Protos.absent\n",
			),
		]);
		assert_eq!(
			vec![
				"src/ring.ms:2,1: semantic error: unknown import 'Test.Missing'",
				"src/ring.ms:3,1: semantic error: unknown import 'Test.Protos.absent'",
			],
			rendered(resolve_imports(&syms, &pkg, &mut mods))
		);
	}

	#[test]
	fn test_reports_import_cycles() {
		let (syms, pkg, mut mods) = resolvable(&[
			(ROOT_FILE, "module Test\n"),
			("src/a.ms", "module Test.A\nimport Test.B\n"),
			("src/b.ms", "module Test.B\nimport Test.A\n"),
		]);
		assert!(resolve_imports(&syms, &pkg, &mut mods).is_empty());
		assert_eq!(
			vec!["src/b.ms:2,1: semantic error: import cycle: Test.A → Test.B → Test.A"],
			rendered(check_import_cycles(&syms, &pkg, &mods))
		);

		let (syms, pkg, mut mods) = resolvable(&[
			(ROOT_FILE, "module Test\n"),
			("src/a.ms", "module Test.A\nimport Test.B\n"),
			("src/b.ms", "module Test.B\nimport Test.C\n"),
			("src/c.ms", "module Test.C\nimport Test.A\n"),
		]);
		assert!(resolve_imports(&syms, &pkg, &mut mods).is_empty());
		assert_eq!(
			vec!["src/c.ms:2,1: semantic error: import cycle: Test.A → Test.B → Test.C → Test.A"],
			rendered(check_import_cycles(&syms, &pkg, &mods))
		);
	}

	#[test]
	fn test_allows_a_diamond() {
		// Two separate paths reaching D is reconvergence, not a cycle: D is
		// walked once and left again before C reaches it, so nothing on the
		// path is ever imported back into.
		let (syms, pkg, mut mods) = resolvable(&[
			(ROOT_FILE, "module Test\n"),
			("src/a.ms", "module Test.A\nimport Test.B\nimport Test.C\n"),
			("src/b.ms", "module Test.B\nimport Test.D\n"),
			("src/c.ms", "module Test.C\nimport Test.D\n"),
			("src/d.ms", "module Test.D\n"),
		]);
		assert!(resolve_imports(&syms, &pkg, &mut mods).is_empty());
		assert!(check_import_cycles(&syms, &pkg, &mods).is_empty());
	}

	#[test]
	fn test_orders_dependencies_before_a_diamonds_reconvergence() {
		// D is a dependency of both B and C, so it precedes both; the root
		// imports nothing, so nothing forces it earlier than last.
		let (syms, pkg) = package(&[
			(ROOT_FILE, "module Test\n"),
			("src/a.ms", "module Test.A\nimport Test.B\nimport Test.C\n"),
			("src/b.ms", "module Test.B\nimport Test.D\n"),
			("src/c.ms", "module Test.C\nimport Test.D\n"),
			("src/d.ms", "module Test.D\n"),
		]);
		let mods = check(&syms, &pkg, &[]).unwrap();
		let names: Vec<String> = mods
			.order()
			.iter()
			.map(|id| dotted_path_to_string(&syms, mods.path(*id)))
			.collect();
		assert_eq!(vec!["Test.D", "Test.B", "Test.C", "Test.A", "Test"], names);
	}

	#[test]
	fn test_orders_a_dependent_of_the_root_before_it() {
		// A module importing the root forces the root earlier, so
		// dependencies-before-dependents holds even for the root.
		let (syms, pkg) = package(&[
			(ROOT_FILE, "module Test\n"),
			("src/a.ms", "module Test.A\nimport Test\n"),
		]);
		let mods = check(&syms, &pkg, &[]).unwrap();
		let names: Vec<String> = mods
			.order()
			.iter()
			.map(|id| dotted_path_to_string(&syms, mods.path(*id)))
			.collect();
		assert_eq!(vec!["Test", "Test.A"], names);
	}

	#[test]
	fn test_derives_the_parent_file() {
		assert_eq!(None, parent_file(ROOT_FILE));
		assert_eq!(Some(ROOT_FILE.to_string()), parent_file("src/codec.ms"));
		assert_eq!(
			Some("src/codec.ms".to_string()),
			parent_file("src/codec/json.ms")
		);
		assert_eq!(
			Some("src/codec/json.ms".to_string()),
			parent_file("src/codec/json/rpc.ms")
		);
	}
}
