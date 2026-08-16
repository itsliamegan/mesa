use std::collections::HashMap;

use ordermap::OrderMap;

use crate::intern::{Interner, Sym};
use crate::pkg::Package;
use crate::sem::Error;
use crate::sem::load::{RESERVED_DIR, ROOT_FILE};
use crate::src::{Location, Span};
use crate::syn::{Chunk, ChunkId, Expr, ExprId, ModuleItem, Place};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ModuleId(u32);

impl ModuleId {
	fn index(self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug)]
pub struct Modules {
	mods: Vec<Module>,
	by_file: HashMap<String, ModuleId>,
}

#[derive(Debug)]
pub struct Module {
	chunk: ChunkId,
	file: String,
	name: Vec<Sym>,
	name_span: Span,
	members: OrderMap<Sym, Span>,
	exports: Option<Vec<Sym>>,
}

impl Modules {
	fn get(&self, id: ModuleId) -> &Module {
		&self.mods[id.index()]
	}

	fn ids(&self) -> impl Iterator<Item = ModuleId> + use<> {
		(0..self.mods.len() as u32).map(ModuleId)
	}

	fn parent(&self, id: ModuleId) -> Option<ModuleId> {
		let file = parent_file(&self.get(id).file)?;
		self.by_file.get(&file).copied()
	}
}

pub fn check(syms: &Interner, pkg: &Package, dirs: &[String]) -> Result<Modules, Vec<Error>> {
	let files: Vec<String> = pkg.chunk_ids().map(|id| pkg.file(id).to_string()).collect();
	let mut errs = check_sibling_files_exist(dirs, &files);

	let mut mods = match build(pkg) {
		Ok(mods) => mods,
		Err(mut header_errs) => {
			errs.append(&mut header_errs);
			return Err(errs);
		}
	};

	errs.append(&mut check_child_prefixes_match(syms, pkg, &mods));
	errs.append(&mut check_members_unique(syms, pkg, &mut mods));

	if !errs.is_empty() {
		return Err(errs);
	}

	Ok(mods)
}

fn build(pkg: &Package) -> Result<Modules, Vec<Error>> {
	let mut mods = Vec::new();
	let mut by_file = HashMap::new();
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
		by_file.insert(file.clone(), ModuleId(mods.len() as u32));
		mods.push(Module {
			chunk: chunk_id,
			file,
			name,
			name_span,
			members: OrderMap::new(),
			exports: read_exports(pkg.get_chunk(chunk_id)),
		});
	}

	if !errs.is_empty() {
		return Err(errs);
	}

	Ok(Modules { mods, by_file })
}

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

fn check_members_unique(syms: &Interner, pkg: &Package, mods: &mut Modules) -> Vec<Error> {
	let mut errs = Vec::new();

	for id in mods.ids() {
		let chunk = pkg.get_chunk(mods.get(id).chunk);
		let mut members = OrderMap::new();
		for item_id in &chunk.top {
			let span = chunk.get_module_item_span(*item_id);
			let name = match chunk.get_module_item(*item_id) {
				ModuleItem::Module(_) => continue,
				ModuleItem::Import(_) => continue,
				ModuleItem::Export(_) => continue,
				ModuleItem::Type(type_) => type_.name,
				ModuleItem::Proto(proto) => proto.name,
				ModuleItem::Def(def) => def.name,
				// A second `x := …` is reassignment, not redeclaration, so a
				// binding claims its key only if nothing holds it.
				ModuleItem::Expr(expr_id) => match bound_name(chunk, *expr_id) {
					Some(name) => {
						members.entry(name).or_insert(span);
						continue;
					}
					None => continue,
				},
			};
			if members.insert(name, span).is_some() {
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
		let parent = match mods.parent(id) {
			Some(parent) => mods.get(parent),
			None => continue,
		};
		let module = mods.get(id);
		let leaf = *module.name.last().unwrap();
		if parent.members.contains_key(&leaf) {
			errs.push(Error::DuplicateMember(
				pkg.loc(module.name_span),
				syms.resolve(leaf).to_string(),
			));
		}
	}

	errs
}

// The name a top-level binding claims, per §44.3: the left-hand side of
// `x := …` is a member key as much as a declared name is.
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
	fn test_checks_prefixes_against_the_parent() {
		// A well-formed tree at two depths, and the root checked against
		// nothing even though it declares a dotted path.
		let (syms, pkg) = package(&[
			(ROOT_FILE, "module Test\n"),
			("src/codec.ms", "module Test.Codec\n"),
			("src/codec/json.ms", "module Test.Codec.Json\n"),
		]);
		let mods = build(&pkg).unwrap();
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
		let mods = build(&pkg).unwrap();
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
