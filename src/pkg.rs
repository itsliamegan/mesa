use serde::Deserialize;

use crate::src::{Location, Sources, Span};
use crate::syn::{ChunkId, Chunks};

#[derive(Debug)]
pub struct Packages {
	pkgs: Vec<Package>,
}

#[derive(Debug, Clone, Copy)]
pub struct PackageId(u32);

#[derive(Debug)]
pub struct Package {
	pub manifest: Manifest,
	pub sources: Sources,
	pub chunks: Chunks,
}

#[derive(Debug, Deserialize)]
pub struct Manifest {
	pub name: String,
	pub version: String,
}

impl Packages {
	pub fn new() -> Self {
		Self { pkgs: Vec::new() }
	}
}

impl PackageId {
	fn index(&self) -> usize {
		self.0 as usize
	}
}

impl Package {
	pub fn new(manifest: Manifest) -> Self {
		Self {
			manifest,
			sources: Sources::new(),
			chunks: Chunks::new(),
		}
	}

	pub fn loc(&self, span: Span) -> Location {
		self.sources.get(span.src).loc(span.start)
	}

	pub fn file(&self, id: ChunkId) -> &str {
		self.sources.get(self.chunks.get(id).src).file()
	}
}
