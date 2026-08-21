use std::cell::RefCell;
use std::rc::Rc;

use crate::intern::Sym;
use crate::pkg::{self, PackageId};

use super::eval::Prelude;
use super::modules::Modules;
use super::scope::Scopes;
use super::types::{ExternMembers, NativeType, NativeTypeId, Natives, Types, build_extern_members};
use super::val::Obj;

pub struct Package<'descs> {
	pub types: Types<'descs>,
	pub mods: Modules<'descs>,
}

impl<'descs> Package<'descs> {
	pub fn new(
		prelude: &Prelude,
		pkgs: &Packages<'descs>,
		desc: &'descs pkg::Package,
		id: PackageId,
	) -> (Self, Vec<(NativeTypeId, ExternMembers)>) {
		let scopes = Scopes::new(prelude.scope.clone(), &desc.modules);
		let types = Types::new(pkgs, id, desc, &scopes);
		let externs = build_extern_members(pkgs, id, desc, &scopes);
		let mods = Modules::new(id, &desc.modules, scopes);

		(Self { types, mods }, externs)
	}
}

pub struct Packages<'descs> {
	descs: &'descs pkg::Packages,
	natives: Natives,
	// Only the packages built so far. A package is built after every package it
	// can import from, so an id is resolvable exactly when the package it names
	// is already here.
	pkgs: Vec<Package<'descs>>,
}

impl<'descs> Packages<'descs> {
	pub fn new(descs: &'descs pkg::Packages, natives: Natives) -> Self {
		Self {
			descs,
			natives,
			pkgs: Vec::with_capacity(descs.len()),
		}
	}

	pub fn native(&self, id: NativeTypeId) -> &NativeType {
		self.natives.get(id)
	}

	pub fn native_val(&self, id: NativeTypeId) -> Rc<RefCell<Obj>> {
		self.natives.val(id)
	}

	pub fn native_id(&self, pkg: PackageId, name: Sym) -> Option<NativeTypeId> {
		self.natives.id(pkg, name)
	}

	pub fn native_ids(&self, pkg: PackageId) -> Vec<NativeTypeId> {
		self.natives.ids(pkg)
	}

	pub fn merge_externs(&mut self, externs: Vec<(NativeTypeId, ExternMembers)>) {
		for (id, extern_) in externs {
			let native = self.natives.get_mut(id);
			native.members.extend(extern_.members);
			native.statics.extend(extern_.statics);
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
