use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rustc_hash::{FxBuildHasher, FxHashMap};

use crate::intern::{CORE_TYPE_NAMES, Interner, Sym};
use crate::pkg::{Package, PackageId};
use crate::rt::Error;
use crate::rt::pkg::Packages;
use crate::rt::scope::Scopes;
use crate::rt::val::{Bool, Dict, List, Member, Nil, Num, Obj, Proc, Str, Val};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeTypeId(u32);

impl NativeTypeId {
	pub fn index(&self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug)]
pub struct NativeParam {
	pub name: &'static str,
	pub default: Option<fn() -> Val>,
}

#[derive(Debug, Clone)]
pub struct NativeMember {
	pub params: &'static [NativeParam],
	pub call: fn(&Val, Vec<Val>) -> Result<Val, Error>,
}

impl NativeMember {
	pub fn defaults(&self) -> Option<Vec<Val>> {
		self.params
			.iter()
			.map(|param| param.default.map(|default| default()))
			.collect()
	}
}

#[derive(Debug, Clone)]
pub struct NativeType {
	pub name: Sym,
	pub new: Option<fn() -> Val>,
	pub members: FxHashMap<Sym, NativeMember>,
}

impl NativeType {
	fn has_member(&self, name: Sym) -> bool {
		self.members.contains_key(&name)
	}
}

pub const CORE_TYPES: &[(
	NativeTypeId,
	Option<fn() -> Val>,
	&[(
		&str,
		&[NativeParam],
		fn(&Val, Vec<Val>) -> Result<Val, Error>,
	)],
)] = &[
	(NativeTypeId::NIL, Some(Nil::new), &[]),
	(NativeTypeId::NUM, Some(Num::new), &[]),
	(NativeTypeId::BOOL, Some(Bool::new), &[]),
	(NativeTypeId::CHAR, None, &[]),
	(
		NativeTypeId::STR,
		Some(Str::new),
		&[("size", &[], Str::size), ("chars", &[], Str::chars)],
	),
	(
		NativeTypeId::LIST,
		Some(List::new),
		&[("size", &[], List::size)],
	),
	(
		NativeTypeId::DICT,
		Some(Dict::new),
		&[("size", &[], Dict::size)],
	),
	(NativeTypeId::PROC, None, &[]),
	(NativeTypeId::TYPE, None, &[]),
	(NativeTypeId::PROTO, None, &[]),
	(NativeTypeId::MODULE, None, &[]),
];

impl NativeTypeId {
	pub const NIL: NativeTypeId = NativeTypeId(0);
	pub const NUM: NativeTypeId = NativeTypeId(1);
	pub const BOOL: NativeTypeId = NativeTypeId(2);
	pub const CHAR: NativeTypeId = NativeTypeId(3);
	pub const STR: NativeTypeId = NativeTypeId(4);
	pub const LIST: NativeTypeId = NativeTypeId(5);
	pub const DICT: NativeTypeId = NativeTypeId(6);
	pub const PROC: NativeTypeId = NativeTypeId(7);
	pub const TYPE: NativeTypeId = NativeTypeId(8);
	pub const PROTO: NativeTypeId = NativeTypeId(9);
	pub const MODULE: NativeTypeId = NativeTypeId(10);
}

// Build a table of the core native types.
pub fn build_core_types(syms: &mut Interner) -> Vec<NativeType> {
	let mut core = Vec::with_capacity(CORE_TYPES.len());
	for (id, new, member_pairs) in CORE_TYPES {
		let mut members = HashMap::with_capacity_and_hasher(member_pairs.len(), FxBuildHasher);
		for (name, params, call) in *member_pairs {
			members.insert(
				syms.intern(name),
				NativeMember {
					params,
					call: *call,
				},
			);
		}
		core.push(NativeType {
			name: syms.intern(CORE_TYPE_NAMES[id.0 as usize]),
			new: *new,
			members,
		});
	}
	core
}

// The runtime image of every type that the program has. Includes runtime
// information about both the static type descriptions and the
// dynamically-loaded native types.
pub struct Types<'descs> {
	pub descs: &'descs types::Types,
	// Indexed in step with the description arena, so a type's index there is
	// its index here.
	user: Vec<UserType>,
	native: Vec<NativeType>,
	native_vals: Vec<Rc<RefCell<Obj>>>,
}

impl<'descs> Types<'descs> {
	// Create a runtime representation of every type, storing a canonical Val
	// for every type and creating an unbound proc for every method.
	pub fn new(
		native: Vec<NativeType>,
		native_vals: Vec<Rc<RefCell<Obj>>>,
		pkgs: &Packages<'descs>,
		pkg_id: PackageId,
		pkg: &'descs Package,
		scopes: &Scopes,
	) -> Self {
		let descs = &pkg.types;
		let mut user = Vec::with_capacity(descs.ids().len());
		for id in descs.ids() {
			let desc = descs.get_type(id);
			let mut methods = FxHashMap::default();
			for (name, site) in &desc.members {
				methods.insert(*name, build_member_proc(pkgs, pkg_id, pkg, scopes, *site));
			}
			let mut statics = FxHashMap::default();
			for (name, static_) in &desc.statics {
				let static_ = match static_ {
					types::Static::Type(id) => Static::Type(*id),
					types::Static::Proc(chunk_id, item_id) => {
						Static::Proc(build_static_proc(pkg_id, pkg, scopes, *chunk_id, *item_id))
					}
				};
				statics.insert(*name, static_);
			}
			user.push(UserType {
				val: Rc::new(RefCell::new(Obj::Type(TypeId::User(pkg_id, id)))),
				methods,
				statics,
			});
		}

		Self {
			descs,
			user,
			native,
			native_vals,
		}
	}

	// Get a type's *qualified* name (including all lexical nesting).
	pub fn name(&self, syms: &Interner, pkgs: &Packages, id: TypeId) -> String {
		match id {
			TypeId::User(pkg, id) => {
				let desc = pkgs.get(pkg).types.descs.get_type(id);
				match desc.enclosing {
					Some(outer) => {
						let outer = self.name(syms, pkgs, TypeId::User(pkg, outer));
						format!("{}.{}", outer, syms.resolve(desc.name))
					}
					None => syms.resolve(desc.name).to_string(),
				}
			}
			TypeId::Native(id) => syms.resolve(self.native(id).name).to_string(),
		}
	}

	pub fn user(&self, id: types::TypeId) -> &UserType {
		&self.user[id.index()]
	}

	pub fn method(&self, id: types::TypeId, name: Sym) -> Option<Rc<RefCell<Proc>>> {
		self.user(id).methods.get(&name).cloned()
	}

	pub fn static_(&self, id: types::TypeId, name: Sym) -> Option<Static> {
		self.user(id).statics.get(&name).cloned()
	}

	pub fn field(&self, id: types::TypeId, name: Sym) -> bool {
		let desc = self.descs.get_type(id);
		desc.ctor_fields.iter().any(|param| param.name == name)
			|| desc.body_fields.contains_key(&name)
	}

	pub fn native(&self, id: NativeTypeId) -> &NativeType {
		&self.native[id.0 as usize]
	}

	pub fn native_val(&self, id: NativeTypeId) -> Rc<RefCell<Obj>> {
		self.native_vals[id.0 as usize].clone()
	}

	pub fn member(&self, pkgs: &Packages, val: &Val, name: Sym) -> Option<Member> {
		let type_id = val.type_id();
		let namespace_type_id = val.namespace_type_id();
		if type_id != namespace_type_id {
			// type_id and namespace_type_id only ever differ when the Val is
			// itself a Type.
			let Val::Obj(rf) = val else {
				panic!();
			};
			// Only a user type has statics; a native one reaches its members
			// through the seam instead.
			let has_static = match namespace_type_id {
				TypeId::User(pkg, id) => pkgs.get(pkg).types.static_(id, name).is_some(),
				TypeId::Native(_) => false,
			};
			if has_static {
				Some(Member::Static(rf.clone(), name))
			} else {
				let TypeId::Native(id) = type_id else {
					panic!();
				};
				if self.native(id).has_member(name) {
					Some(Member::Native(val.clone(), name))
				} else {
					None
				}
			}
		} else {
			match namespace_type_id {
				TypeId::User(pkg, id) => {
					let Val::Obj(rf) = val else {
						panic!();
					};
					let types = &pkgs.get(pkg).types;
					if types.field(id, name) || types.method(id, name).is_some() {
						Some(Member::User(rf.clone(), name))
					} else {
						None
					}
				}
				TypeId::Native(id) => {
					if self.native(id).has_member(name) {
						Some(Member::Native(val.clone(), name))
					} else {
						None
					}
				}
			}
		}
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
