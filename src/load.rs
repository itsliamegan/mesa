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
pub fn collect(root_dir: &Path) -> Result<Sources, Error> {
	let mut sources = Sources::new();

	let mut files = BTreeMap::new();
	let mut dirs = BTreeSet::from([PathBuf::from("src")]);
	while let Some(dir) = dirs.pop_first() {
		let entries =
			fs::read_dir(root_dir.join(&dir)).map_err(|_| Error::Unreadable(dir.to_path_buf()))?;

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
				dirs.insert(path.to_path_buf());
			} else {
				if path.extension() != Some(OsStr::new("ms")) {
					continue;
				}
				let text = fs::read_to_string(root_dir.join(&path))
					.map_err(|_| Error::Unreadable(path.clone()))?;
				files.insert(path, text);
			}
		}
	}

	for (file, text) in files {
		sources.add(file, text);
	}

	Ok(sources)
}
