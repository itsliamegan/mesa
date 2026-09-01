use crate::pkg::{self, PackageId};

use super::eval::Prelude;
use super::modules::Modules;
use super::scope::Scopes;
use super::types::{Natives, Types, add_extern_members};

pub struct Package<'descs> {
	pub types: Types<'descs>,
	pub mods: Modules<'descs>,
}

impl<'descs> Package<'descs> {
	pub fn new(
		prelude: &Prelude,
		pkgs: &Packages<'descs>,
		natives: &mut Natives,
		desc: &'descs pkg::Package,
		id: PackageId,
	) -> Self {
		let scopes = Scopes::new(prelude.scope.clone(), desc.modules());
		let types = Types::new(pkgs, id, desc, &scopes);
		add_extern_members(pkgs, natives, id, desc, &scopes);
		let mods = Modules::new(id, desc.modules(), scopes);

		Self { types, mods }
	}
}

pub struct Packages<'descs> {
	descs: &'descs pkg::Packages,
	// Only the packages built so far. A package is built after every package it
	// can import from, so an id is resolvable exactly when the package it names
	// is already here.
	pkgs: Vec<Package<'descs>>,
}

impl<'descs> Packages<'descs> {
	pub fn new(descs: &'descs pkg::Packages) -> Self {
		Self {
			descs,
			pkgs: Vec::with_capacity(descs.len()),
		}
	}

	pub fn descs(&self) -> &'descs pkg::Packages {
		self.descs
	}

	pub fn desc(&self, id: PackageId) -> &'descs pkg::Package {
		self.descs.get(id)
	}

	pub fn get(&self, id: PackageId) -> &Package<'descs> {
		&self.pkgs[id.index()]
	}

	pub fn insert(&mut self, id: PackageId, pkg: Package<'descs>) {
		debug_assert_eq!(id.index(), self.pkgs.len());
		self.pkgs.push(pkg);
	}

	pub fn ids(&self) -> impl Iterator<Item = PackageId> {
		(0..self.descs.len()).map(PackageId::from_index)
	}
}
