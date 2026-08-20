use std::path::Path;

use serde::Deserialize;

use crate::sem::modules::Modules;
use crate::sem::types::Types;
use crate::src::{Location, Sources, Span};
use crate::syn::{ChunkId, Chunks};

#[derive(Debug)]
pub struct Packages {
	pkgs: Vec<Package>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PackageId(u32);

#[derive(Debug)]
pub struct Package {
	pub manifest: Manifest,
	pub sources: Sources,
	pub chunks: Chunks,
	pub modules: Modules,
	pub types: Types,
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

	pub fn reserve(&mut self) -> PackageId {
		PackageId::from_index(self.pkgs.len())
	}

	pub fn insert(&mut self, id: PackageId, pkg: Package) {
		debug_assert_eq!(id.index(), self.pkgs.len());
		self.pkgs.push(pkg);
	}

	pub fn get(&self, id: PackageId) -> &Package {
		&self.pkgs[id.index()]
	}

	pub fn len(&self) -> usize {
		self.pkgs.len()
	}

	pub fn iter(&self) -> impl Iterator<Item = (PackageId, &Package)> {
		self.pkgs
			.iter()
			.enumerate()
			.map(|(i, pkg)| (PackageId::from_index(i), pkg))
	}
}

impl PackageId {
	pub fn from_index(index: usize) -> Self {
		Self(index as u32)
	}

	pub fn index(&self) -> usize {
		self.0 as usize
	}
}

impl Package {
	pub fn loc(&self, span: Span) -> Location {
		self.sources.get(span.src).loc(span.start)
	}

	pub fn file(&self, id: ChunkId) -> &Path {
		self.sources.get(self.chunks.get(id).src).file()
	}
}
