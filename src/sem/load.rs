use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use toml::{Table, Value};

use crate::intern::Interner;
use crate::pkg::Package;
use crate::syn;
use crate::syn::lex::Lexer;
use crate::syn::nodes::ChunkId;
use crate::syn::parse::Parser;

pub const ROOT_FILE: &str = "src/package.ms";
pub const RESERVED_DIR: &str = "src/package";
pub const MANIFEST_FILE: &str = "package.toml";

#[derive(Debug)]
pub enum Error {
	NoManifest,
	NoSrcDir,
	NoRootModule,
	BadManifest(String),
	Unreadable(PathBuf),
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		write!(f, "error: ")?;
		match self {
			Self::NoManifest => write!(f, "missing package.toml manifest"),
			Self::NoSrcDir => write!(f, "missing src/ directory"),
			Self::NoRootModule => write!(f, "missing src/package.ms root module"),
			Self::BadManifest(why) => write!(f, "malformed package.toml manifest: {}", why),
			Self::Unreadable(path) => write!(f, "cannot read file '{}'", path.to_string_lossy()),
		}
	}
}

#[derive(Debug)]
pub struct Manifest {
	pub name: Option<String>,
	pub version: Option<String>,
}

#[derive(Debug)]
pub struct Tree {
	pub dirs: Vec<String>,
	pub files: Vec<(String, String)>,
}

pub fn find() -> Result<(PathBuf, Manifest), Error> {
	let root_dir = env::current_dir().unwrap();

	let manifest_file = root_dir.join(MANIFEST_FILE);
	if !manifest_file.exists() {
		return Err(Error::NoManifest);
	}
	let text = fs::read_to_string(&manifest_file)
		.map_err(|_| Error::Unreadable(PathBuf::from(MANIFEST_FILE)))?;
	let manifest = read_manifest(&text)?;

	let src_dir = root_dir.join("src");
	if !src_dir.exists() {
		return Err(Error::NoSrcDir);
	}
	if !root_dir.join(ROOT_FILE).exists() {
		return Err(Error::NoRootModule);
	}

	Ok((root_dir, manifest))
}

fn read_manifest(text: &str) -> Result<Manifest, Error> {
	let table = text
		.parse::<Table>()
		.map_err(|err| Error::BadManifest(err.message().to_string()))?;

	Ok(Manifest {
		name: read_toml_str(&table, "name")?,
		version: read_toml_str(&table, "version")?,
	})
}

fn read_toml_str(table: &Table, key: &str) -> Result<Option<String>, Error> {
	match table.get(key) {
		Some(Value::String(text)) => Ok(Some(text.clone())),
		Some(_) => Err(Error::BadManifest(format!("'{}' is not a string", key))),
		None => Ok(None),
	}
}

pub fn collect(root_dir: &Path) -> Result<Tree, Error> {
	let mut dirs = BTreeSet::new();
	let mut files = BTreeMap::new();
	collect_all_in_dir(root_dir, Path::new("src"), &mut dirs, &mut files)?;
	Ok(Tree {
		dirs: dirs.into_iter().collect(),
		files: files.into_iter().collect(),
	})
}

fn collect_all_in_dir(
	root_dir: &Path,
	dir: &Path,
	dirs: &mut BTreeSet<String>,
	files: &mut BTreeMap<String, String>,
) -> Result<(), Error> {
	let entries =
		fs::read_dir(root_dir.join(dir)).map_err(|_| Error::Unreadable(dir.to_path_buf()))?;

	for entry in entries {
		let entry = entry.map_err(|_| Error::Unreadable(dir.to_path_buf()))?;
		let path = dir.join(entry.file_name());

		if entry.file_name().to_string_lossy().starts_with(".") {
			continue;
		}

		let file_type = entry
			.file_type()
			.map_err(|_| Error::Unreadable(path.clone()))?;

		if file_type.is_dir() {
			dirs.insert(path.to_string_lossy().to_string());
			collect_all_in_dir(root_dir, &path, dirs, files)?;
		} else {
			if path.extension() != Some(OsStr::new("ms")) {
				continue;
			}
			let text = fs::read_to_string(root_dir.join(&path))
				.map_err(|_| Error::Unreadable(path.clone()))?;
			files.insert(path.to_string_lossy().to_string(), text);
		}
	}

	Ok(())
}

pub fn parse(
	syms: &mut Interner,
	files: Vec<(String, String)>,
) -> Result<(Package, ChunkId), Vec<syn::Error>> {
	let mut pkg = Package::new();
	let mut errs = Vec::new();
	let mut root_chunk_id = None;

	for (file, text) in files {
		let is_root_module = file == "src/package.ms";
		let src_id = pkg.add_src(file, text);
		let src = pkg.get_src(src_id);
		let toks = match Lexer::new(syms, src).lex() {
			Ok(toks) => toks,
			Err(err) => {
				errs.push(err);
				continue;
			}
		};
		let chunk = match Parser::new(src, toks).parse() {
			Ok(chunk) => chunk,
			Err(err) => {
				errs.push(err);
				continue;
			}
		};
		let chunk_id = pkg.add_chunk(chunk);
		if is_root_module {
			root_chunk_id = Some(chunk_id);
		}
	}

	if !errs.is_empty() {
		Err(errs)
	} else {
		Ok((pkg, root_chunk_id.unwrap()))
	}
}
