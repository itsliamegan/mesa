use std::path::Path;

use serde::Deserialize;

use crate::sem::modules::Modules;
use crate::sem::protos::Protos;
use crate::sem::types::Types;
use crate::src::{Location, Sources, Span};
use crate::syn::{ChunkId, Chunks};

#[derive(Debug)]
pub struct Packages {
	pkgs: Vec<Package>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct PackageId(u32);

#[derive(Debug)]
pub struct Package {
	manifest: Manifest,
	sources: Sources,
	chunks: Chunks,
	modules: Option<Modules>,
	protos: Option<Protos>,
	types: Option<Types>,
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

	pub fn open(&mut self, manifest: Manifest, sources: Sources, chunks: Chunks) -> PackageId {
		let id = PackageId::from_index(self.pkgs.len());
		self.pkgs.push(Package {
			manifest,
			sources,
			chunks,
			modules: None,
			protos: None,
			types: None,
		});
		id
	}

	pub fn publish_modules(&mut self, id: PackageId, modules: Modules) {
		let slot = &mut self.pkgs[id.index()].modules;
		assert!(slot.is_none());
		*slot = Some(modules);
	}

	pub fn publish_protos(&mut self, id: PackageId, protos: Protos) {
		let slot = &mut self.pkgs[id.index()].protos;
		assert!(slot.is_none());
		*slot = Some(protos);
	}

	pub fn publish_types(&mut self, id: PackageId, types: Types) {
		let slot = &mut self.pkgs[id.index()].types;
		assert!(slot.is_none());
		*slot = Some(types);
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
	pub fn manifest(&self) -> &Manifest {
		&self.manifest
	}

	pub fn sources(&self) -> &Sources {
		&self.sources
	}

	pub fn chunks(&self) -> &Chunks {
		&self.chunks
	}

	pub fn modules(&self) -> &Modules {
		self.modules.as_ref().unwrap()
	}

	pub fn protos(&self) -> &Protos {
		self.protos.as_ref().unwrap()
	}

	pub fn types(&self) -> &Types {
		self.types.as_ref().unwrap()
	}

	pub fn loc(&self, span: Span) -> Location {
		self.sources().get(span.source).loc(span.start)
	}

	pub fn file(&self, id: ChunkId) -> &Path {
		self.sources().get(self.chunks().get(id).source).file()
	}
}
