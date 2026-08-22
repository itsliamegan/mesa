use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rustc_hash::{FxBuildHasher, FxHashMap};

use crate::intern::{Interner, Sym};
use crate::pkg::{Package, PackageId};
use crate::rt::Error;
use crate::rt::pkg::Packages;
use crate::rt::scope::Scopes;
use crate::rt::val::{Bool, Char, Dict, List, Nil, Num, Obj, Proc, Str, Val};
use crate::sem::types::{self, MemberSite};
use crate::syn::nodes::{TypeItem, TypeItemId};
use crate::syn::{self, ChunkId};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TypeId {
	// A user type is an index into one package's arena, so it carries the
	// package. Native type identity is global, so a native one does not.
	User(PackageId, types::TypeId),
	Native(NativeTypeId),
}

// Data the runtime attaches to the type descriptor, the latter having already
// computed most of the important information.
#[derive(Debug)]
pub struct UserType {
	// The type's canonical value.
	pub val: Rc<RefCell<Obj>>,
	// Every member reachable on an instance, method precedence already
	// accounted for by the descriptor's method table.
	methods: FxHashMap<Sym, Rc<RefCell<Proc>>>,
	statics: FxHashMap<Sym, Static>,
}

#[derive(Debug, Clone)]
pub enum Static {
	Proc(Rc<RefCell<Proc>>),
	Type(types::TypeId),
}

pub use crate::sem::types::NativeTypeId;

#[derive(Debug)]
pub struct NativeParam {
	pub name: &'static str,
	pub default: Option<fn() -> Val>,
}

#[derive(Debug, Clone)]
pub enum NativeMember {
	Native(NativeMethod),
	User(Rc<RefCell<Proc>>),
}

#[derive(Debug, Clone, Copy)]
pub struct NativeMethod {
	pub params: &'static [NativeParam],
	pub call: fn(&Val, Vec<Val>) -> Result<Val, Error>,
}

impl NativeMethod {
	pub fn defaults(&self) -> Option<Vec<Val>> {
		self.params
			.iter()
			.map(|param| param.default.map(|default| default()))
			.collect()
	}
}

// A native type as its Rust implementation declares it, before registration
// interns the names and assigns identity.
pub struct NativeTypeSpec {
	pub name: &'static str,
	pub new: Option<fn() -> Val>,
	pub members: &'static [NativeMemberSpec],
	pub statics: &'static [NativeMemberSpec],
}

#[derive(Debug)]
pub struct NativeType {
	pub name: Sym,
	pub new: Option<fn() -> Val>,
	// The type's canonical value.
	pub val: Rc<RefCell<Obj>>,
	pub members: FxHashMap<Sym, NativeMember>,
	pub statics: FxHashMap<Sym, NativeMember>,
}

impl NativeType {
	pub(crate) fn has_member(&self, name: Sym) -> bool {
		self.members.contains_key(&name)
	}

	pub(crate) fn has_static(&self, name: Sym) -> bool {
		self.statics.contains_key(&name)
	}
}

// An entry of a native type's member or static table, as written in Rust.
type NativeMemberSpec = (
	&'static str,
	&'static [NativeParam],
	fn(&Val, Vec<Val>) -> Result<Val, Error>,
);

// Every core type, in id order. Each claims its reserved id.
pub const CORE_TYPES: &[NativeTypeSpec] = &[
	NativeTypeSpec {
		name: "Nil",
		new: Some(Nil::new),
		members: &[],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Num",
		new: Some(Num::new),
		members: &[(
			"order",
			&[NativeParam {
				name: "other",
				default: None,
			}],
			Num::order,
		)],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Bool",
		new: Some(Bool::new),
		members: &[],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Char",
		new: None,
		members: &[(
			"order",
			&[NativeParam {
				name: "other",
				default: None,
			}],
			Char::order,
		)],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Str",
		new: Some(Str::new),
		members: &[
			("size", &[], Str::size),
			("chars", &[], Str::chars),
			(
				"order",
				&[NativeParam {
					name: "other",
					default: None,
				}],
				Str::order,
			),
		],
		statics: &[("empty", &[], Str::empty)],
	},
	NativeTypeSpec {
		name: "List",
		new: Some(List::new),
		members: &[("size", &[], List::size)],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Dict",
		new: Some(Dict::new),
		members: &[("size", &[], Dict::size)],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Proc",
		new: None,
		members: &[],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Type",
		new: None,
		members: &[],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Proto",
		new: None,
		members: &[],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Module",
		new: None,
		members: &[],
		statics: &[],
	},
];

impl NativeTypeId {
	pub const NIL: NativeTypeId = NativeTypeId::new(0);
	pub const NUM: NativeTypeId = NativeTypeId::new(1);
	pub const BOOL: NativeTypeId = NativeTypeId::new(2);
	pub const CHAR: NativeTypeId = NativeTypeId::new(3);
	pub const STR: NativeTypeId = NativeTypeId::new(4);
	pub const LIST: NativeTypeId = NativeTypeId::new(5);
	pub const DICT: NativeTypeId = NativeTypeId::new(6);
	pub const PROC: NativeTypeId = NativeTypeId::new(7);
	pub const TYPE: NativeTypeId = NativeTypeId::new(8);
	pub const PROTO: NativeTypeId = NativeTypeId::new(9);
	pub const MODULE: NativeTypeId = NativeTypeId::new(10);
}

#[cfg(test)]
mod core_types_tests {
	use super::*;

	// 'Val::type_id' returns these constants directly, so they must match
	// 'CORE_TYPES' order exactly: registration assigns ids by position, with
	// no per-spec pinning to check them against.
	#[test]
	fn core_types_match_reserved_slots() {
		let names: Vec<&str> = CORE_TYPES.iter().map(|spec| spec.name).collect();
		let expected = [
			"Nil", "Num", "Bool", "Char", "Str", "List", "Dict", "Proc", "Type", "Proto", "Module",
		];
		assert_eq!(names, expected);
	}
}

// Every native type in the program. There is exactly one per runtime; native
// type identity is global, so every package reads the same table.
pub struct Natives {
	impls: Vec<NativeType>,
}

impl Natives {
	pub fn new() -> Self {
		Self { impls: Vec::new() }
	}

	// Register a package's native types in the order the package provides them.
	// Determine the minimal shape that the semantic layer needs to check
	// against.
	pub fn register(
		&mut self,
		syms: &mut Interner,
		specs: &[NativeTypeSpec],
	) -> HashMap<Sym, types::NativeTypeShape> {
		let mut shapes = HashMap::with_capacity(specs.len());
		for spec in specs {
			let id = NativeTypeId::new(self.impls.len() as u32);
			let name = syms.intern(spec.name);
			self.impls.push(NativeType {
				name,
				new: spec.new,
				val: Rc::new(RefCell::new(Obj::Type(TypeId::Native(id)))),
				members: build_native_members(syms, spec.members),
				statics: build_native_members(syms, spec.statics),
			});
			shapes.insert(
				name,
				types::NativeTypeShape {
					provider: id,
					members: build_native_params(syms, spec.members),
					statics: build_native_params(syms, spec.statics),
				},
			);
		}
		shapes
	}

	pub fn get(&self, id: NativeTypeId) -> &NativeType {
		&self.impls[id.index()]
	}

	pub fn get_mut(&mut self, id: NativeTypeId) -> &mut NativeType {
		&mut self.impls[id.index()]
	}

	pub fn val(&self, id: NativeTypeId) -> Rc<RefCell<Obj>> {
		self.get(id).val.clone()
	}
}

fn build_native_params(
	syms: &mut Interner,
	specs: &[NativeMemberSpec],
) -> HashMap<Sym, Vec<types::NativeParam>> {
	let mut members = HashMap::with_capacity(specs.len());
	for (name, params, _) in specs {
		let params = params
			.iter()
			.map(|param| types::NativeParam {
				name: syms.intern(param.name),
				has_default: param.default.is_some(),
			})
			.collect();
		members.insert(syms.intern(name), params);
	}
	members
}

fn build_native_members(
	syms: &mut Interner,
	specs: &[NativeMemberSpec],
) -> FxHashMap<Sym, NativeMember> {
	let mut members = HashMap::with_capacity_and_hasher(specs.len(), FxBuildHasher);
	for (name, params, call) in specs {
		members.insert(
			syms.intern(name),
			NativeMember::Native(NativeMethod {
				params,
				call: *call,
			}),
		);
	}
	members
}

// The runtime image of one package's user types. Native types are not here:
// their identity is global, so they live on 'Packages' instead.
pub struct Types<'descs> {
	pub descs: &'descs types::Types,
	// Indexed in step with the description arena, so a type's index there is
	// its index here. A native entry has no runtime representation of its own
	// here — its identity is global and lives on 'Natives' instead — so its
	// slot is 'None'; this keeps every other id's slot aligned with 'descs'.
	user: Vec<Option<UserType>>,
}

impl<'descs> Types<'descs> {
	// Create a runtime representation of every user type, storing a canonical
	// Val for every type and creating an unbound proc for every method.
	pub fn new(
		pkgs: &Packages<'descs>,
		pkg_id: PackageId,
		pkg: &'descs Package,
		scopes: &Scopes,
	) -> Self {
		let descs = &pkg.types;
		let mut user = Vec::with_capacity(descs.ids().len());
		for id in descs.ids() {
			let types::Type::User(desc) = descs.get_type(id) else {
				user.push(None);
				continue;
			};
			let mut methods = FxHashMap::default();
			for (name, site) in &desc.members {
				methods.insert(*name, build_member_proc(pkgs, pkg_id, pkg, scopes, *site));
			}
			let mut statics = FxHashMap::default();
			for (name, static_) in &desc.statics {
				let static_ = match static_ {
					types::Static::Type(id) => Static::Type(*id),
					types::Static::Native => panic!(),
					types::Static::Proc(chunk_id, item_id) => {
						Static::Proc(build_static_proc(pkg_id, pkg, scopes, *chunk_id, *item_id))
					}
				};
				statics.insert(*name, static_);
			}
			user.push(Some(UserType {
				val: Rc::new(RefCell::new(Obj::Type(TypeId::User(pkg_id, id)))),
				methods,
				statics,
			}));
		}

		Self { descs, user }
	}

	pub fn user(&self, id: types::TypeId) -> &UserType {
		self.user[id.index()].as_ref().unwrap()
	}

	pub fn method(&self, id: types::TypeId, name: Sym) -> Option<Rc<RefCell<Proc>>> {
		self.user(id).methods.get(&name).cloned()
	}

	pub fn static_(&self, id: types::TypeId, name: Sym) -> Option<Static> {
		self.user(id).statics.get(&name).cloned()
	}

	pub fn field(&self, id: types::TypeId, name: Sym) -> bool {
		let types::Type::User(desc) = self.descs.get_type(id) else {
			panic!()
		};
		desc.ctor_fields.iter().any(|param| param.name == name)
			|| desc.body_fields.contains_key(&name)
	}
}

// Add the runtime half of every 'extern type' in a package to the native type
// it implements. The native half is already in each native type's maps, put
// there by registration; this walks the package's own type descriptions,
// already fully resolved by 'sem::check', for the rest.
pub fn add_extern_members<'descs>(
	pkgs: &Packages<'descs>,
	natives: &mut Natives,
	pkg_id: PackageId,
	pkg: &'descs Package,
	scopes: &Scopes,
) {
	for id in pkg.types.ids() {
		let types::Type::Native(desc) = pkg.types.get_type(id) else {
			continue;
		};

		let mut members = FxHashMap::default();
		for (name, site) in &desc.members {
			let proc_rf = match site {
				types::MemberSite::Declared(..) | types::MemberSite::Provided(..) => {
					build_member_proc(pkgs, pkg_id, pkg, scopes, *site)
				}
				// Already in the native type's map.
				types::MemberSite::Native => continue,
			};
			members.insert(*name, NativeMember::User(proc_rf));
		}

		let mut statics = FxHashMap::default();
		for (name, static_) in &desc.statics {
			let proc_rf = match static_ {
				// An extern body admits no case or inner type declarations.
				types::Static::Type(_) => panic!(),
				types::Static::Native => continue,
				types::Static::Proc(chunk_id, item_id) => {
					build_static_proc(pkg_id, pkg, scopes, *chunk_id, *item_id)
				}
			};
			statics.insert(*name, NativeMember::User(proc_rf));
		}

		let native = natives.get_mut(desc.provider);
		native.members.extend(members);
		native.statics.extend(statics);
	}
}

// Build the proc a member resolves to. A member declared by the type reads
// off its own item; one acquired from a protocol reads off the protocol's,
// in whichever chunk that protocol was written.
fn build_member_proc<'descs>(
	pkgs: &Packages<'descs>,
	pkg_id: PackageId,
	pkg: &'descs Package,
	scopes: &Scopes,
	site: MemberSite,
) -> Rc<RefCell<Proc>> {
	match site {
		MemberSite::Declared(chunk_id, item_id) => {
			// Protocols don't involve static methods; assert that this is an
			// instance method.
			let TypeItem::Method(syn::nodes::Method::Instance(def)) =
				pkg.chunks.get(chunk_id).get_type_item(item_id)
			else {
				panic!()
			};
			Rc::new(RefCell::new(Proc {
				name: def.name,
				params: def.params.to_vec(),
				body: def.body,
				pkg: pkg_id,
				chunk: chunk_id,
				scope: scopes.module(chunk_id),
			}))
		}
		MemberSite::Provided(site_pkg, chunk_id, item_id) => {
			// The protocol may belong to another package, whose arenas are the
			// only place its body can be read from; the package being built is
			// not in 'pkgs' yet, so it supplies its own. The body is written in
			// the protocol's file and may reference names bound there, so it
			// closes over that module's scope, not the implementing type's.
			let (chunks, scope) = match site_pkg == pkg_id {
				true => (&pkg.chunks, scopes.module(chunk_id)),
				false => (
					&pkgs.desc(site_pkg).chunks,
					pkgs.get(site_pkg).mods.scope(chunk_id),
				),
			};
			let item = chunks.get(chunk_id).get_proto_item(item_id);
			Rc::new(RefCell::new(Proc {
				name: item.def.name,
				params: item.def.params.to_vec(),
				body: item.def.body,
				pkg: site_pkg,
				chunk: chunk_id,
				scope,
			}))
		}
		MemberSite::Native => panic!(),
	}
}

fn build_static_proc(
	pkg_id: PackageId,
	pkg: &Package,
	scopes: &Scopes,
	chunk_id: ChunkId,
	item_id: TypeItemId,
) -> Rc<RefCell<Proc>> {
	let TypeItem::Method(syn::nodes::Method::Static(def)) =
		pkg.chunks.get(chunk_id).get_type_item(item_id)
	else {
		panic!()
	};
	Rc::new(RefCell::new(Proc {
		name: def.name,
		params: def.params.to_vec(),
		body: def.body,
		pkg: pkg_id,
		chunk: chunk_id,
		scope: scopes.module(chunk_id),
	}))
}
