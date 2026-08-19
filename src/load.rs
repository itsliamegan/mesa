use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use toml;

use crate::intern::Interner;
use crate::pkg::{Manifest, Package};
use crate::src::Sources;
use crate::syn::lex::Lexer;
use crate::syn::parse::Parser;
use crate::syn::{self, Chunks};

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
			Self::BadManifest(detail) => write!(f, "malformed package.toml manifest: {}", detail),
			Self::Unreadable(path) => write!(f, "cannot read file '{}'", path.to_string_lossy()),
		}
	}
}

#[derive(Debug)]
pub struct Tree {
	pub dirs: Vec<String>,
	pub files: Vec<(String, String)>,
}

// Find the root directory and manifest of the package. Start in the given
// directory and check for a manifest, walking parents up to $HOME.
pub fn find(mut root_dir: &Path) -> Result<(PathBuf, Manifest), Error> {
	let home_dir = env::home_dir().unwrap();
	let mut manifest_file = root_dir.join(MANIFEST_FILE);
	while !manifest_file.exists() && root_dir != home_dir {
		root_dir = root_dir.parent().unwrap();
		manifest_file = root_dir.join(MANIFEST_FILE);
	}
	if !manifest_file.exists() || root_dir == home_dir {
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

	Ok((root_dir.to_path_buf(), manifest))
}

fn read_manifest(text: &str) -> Result<Manifest, Error> {
	toml::from_str(text).map_err(|err| Error::BadManifest(err.message().to_string()))
}

// Walk the package directory tree and collect all source files.
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

// Parse all source files in a package.
pub fn parse(
	syms: &mut Interner,
	files: Vec<(String, String)>,
	manifest: Manifest,
) -> Result<Package, Vec<syn::Error>> {
	let mut pkg = Package::new(manifest);
	let mut errs = Vec::new();

	for (file, text) in files {
		let source_id = pkg.sources.add(file, text);
		let source = pkg.sources.get(source_id);
		let toks = match Lexer::new(syms, source).lex() {
			Ok(toks) => toks,
			Err(err) => {
				errs.push(err);
				continue;
			}
		};
		let chunk = match Parser::new(source, toks).parse() {
			Ok(chunk) => chunk,
			Err(err) => {
				errs.push(err);
				continue;
			}
		};
		pkg.chunks.add(chunk);
	}

	if !errs.is_empty() {
		return Err(errs);
	}

	Ok(pkg)
}
