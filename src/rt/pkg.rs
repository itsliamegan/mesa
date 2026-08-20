use crate::pkg::{self, PackageId};

use super::modules::Modules;
use super::types::Types;

pub struct Package<'pkg> {
	pub types: Types<'pkg>,
	pub mods: Modules<'pkg>,
}

pub struct Packages<'pkg> {
	descs: &'pkg pkg::Packages,
	pkgs: Vec<Option<Package<'pkg>>>,
}

impl<'pkg> Packages<'pkg> {
	pub fn new(descs: &'pkg pkg::Packages) -> Self {
		let mut pkgs = Vec::with_capacity(descs.len());
		for _ in 0..descs.len() {
			pkgs.push(None);
		}
		Self { descs, pkgs }
	}

	pub fn descs(&self) -> &'pkg pkg::Packages {
		self.descs
	}

	pub fn desc(&self, id: PackageId) -> &'pkg pkg::Package {
		self.descs.get(id)
	}

	pub fn get(&self, id: PackageId) -> &Package<'pkg> {
		self.pkgs[id.index()].as_ref().unwrap()
	}

	pub fn insert(&mut self, id: PackageId, pkg: Package<'pkg>) {
		self.pkgs[id.index()] = Some(pkg);
	}

	pub fn ids(&self) -> impl Iterator<Item = PackageId> {
		(0..self.pkgs.len()).map(PackageId::from_index)
	}
}
