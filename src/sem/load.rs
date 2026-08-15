use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use crate::intern::Interner;
use crate::pkg::Package;
use crate::syn::{self, ChunkId, Lexer, Parser};

#[derive(Debug)]
pub enum Error {
	NoManifest,
	NoSrcDir,
	NoRootModule,
	Unreadable(PathBuf),
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		write!(f, "error: ")?;
		match self {
			Self::NoManifest => write!(f, "missing package.toml manifest file"),
			Self::NoSrcDir => write!(f, "missing src/ directory"),
			Self::NoRootModule => write!(f, "missing src/package.ms root module"),
			Self::Unreadable(path) => write!(f, "cannot read file '{}'", path.to_string_lossy()),
		}
	}
}

pub fn find() -> Result<PathBuf, Error> {
	let root_dir = env::current_dir().unwrap();

	let manifest_file = root_dir.join("package.toml");
	if !manifest_file.exists() {
		return Err(Error::NoManifest);
	}

	let src_dir = root_dir.join("src");
	if !src_dir.exists() {
		return Err(Error::NoSrcDir);
	}
	if !src_dir.join("package.ms").exists() {
		return Err(Error::NoRootModule);
	}

	Ok(root_dir)
}

pub fn collect(root_dir: &Path) -> Result<Vec<(String, String)>, Error> {
	let contents = collect_all_in_dir(root_dir, Path::new("src"))?;
	let files = contents.into_iter().collect();
	Ok(files)
}

fn collect_all_in_dir(root_dir: &Path, dir: &Path) -> Result<BTreeMap<String, String>, Error> {
	let mut contents = BTreeMap::new();
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
			let mut subdir_contents = collect_all_in_dir(root_dir, &path)?;
			contents.append(&mut subdir_contents);
		} else {
			if path.extension() != Some(OsStr::new("ms")) {
				continue;
			}
			let text = fs::read_to_string(root_dir.join(&path))
				.map_err(|_| Error::Unreadable(path.clone()))?;
			contents.insert(path.to_string_lossy().to_string(), text);
		}
	}

	Ok(contents)
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
