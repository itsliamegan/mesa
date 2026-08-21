use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rustc_hash::{FxBuildHasher, FxHashMap};

use crate::intern::{Interner, Sym};
use crate::pkg::{Package, PackageId};
use crate::rt::Error;
use crate::rt::pkg::Packages;
use crate::rt::scope::Scopes;
use crate::rt::val::{Bool, Char, Dict, List, Member, Nil, Num, Obj, Proc, Str, Val};
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
	// The slot this type must land in. A core type's ID is a fixed constant
	// that 'Val::type_id' returns directly, so its registration pins the slot
	// rather than trusting insertion order; an extended type leaves it None.
	pub id: Option<NativeTypeId>,
	pub new: Option<fn() -> Val>,
	pub members: &'static [NativeMemberSpec],
	pub statics: &'static [NativeMemberSpec],
}

#[derive(Debug)]
pub struct NativeType {
	pub desc: types::NativeType,
	pub new: Option<fn() -> Val>,
	pub members: FxHashMap<Sym, NativeMember>,
	pub statics: FxHashMap<Sym, NativeMember>,
}

impl NativeType {
	fn has_member(&self, name: Sym) -> bool {
		self.members.contains_key(&name)
	}

	fn has_static(&self, name: Sym) -> bool {
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
		id: Some(NativeTypeId::NIL),
		new: Some(Nil::new),
		members: &[],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Num",
		id: Some(NativeTypeId::NUM),
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
		id: Some(NativeTypeId::BOOL),
		new: Some(Bool::new),
		members: &[],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Char",
		id: Some(NativeTypeId::CHAR),
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
		id: Some(NativeTypeId::STR),
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
		id: Some(NativeTypeId::LIST),
		new: Some(List::new),
		members: &[("size", &[], List::size)],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Dict",
		id: Some(NativeTypeId::DICT),
		new: Some(Dict::new),
		members: &[("size", &[], Dict::size)],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Proc",
		id: Some(NativeTypeId::PROC),
		new: None,
		members: &[],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Type",
		id: Some(NativeTypeId::TYPE),
		new: None,
		members: &[],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Proto",
		id: Some(NativeTypeId::PROTO),
		new: None,
		members: &[],
		statics: &[],
	},
	NativeTypeSpec {
		name: "Module",
		id: Some(NativeTypeId::MODULE),
		new: None,
		members: &[],
		statics: &[],
	},
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

// Every native type in the program and the canonical value for each. There is
// exactly one per runtime; native type identity is global, so every package
// reads the same table and the same values.
pub struct Natives {
	types: Vec<NativeType>,
	// Indexed in step with 'types', so a native type's id indexes both.
	vals: Vec<Rc<RefCell<Obj>>>,
	// Which native type a name means inside a given package. The id is global,
	// but the declaration site is not: 'extern type File' in the stdlib and in
	// a user package name different types.
	names: FxHashMap<(PackageId, Sym), NativeTypeId>,
}

impl Natives {
	pub fn new() -> Self {
		Self {
			types: Vec::new(),
			vals: Vec::new(),
			names: FxHashMap::default(),
		}
	}

	// Register a package's native types, in the order the package provides
	// them.
	pub fn register(&mut self, syms: &mut Interner, pkg: PackageId, specs: &[NativeTypeSpec]) {
		for spec in specs {
			let id = NativeTypeId(self.types.len() as u32);
			if let Some(claimed) = spec.id {
				assert_eq!(claimed, id);
			}
			let name = syms.intern(spec.name);
			self.types.push(NativeType {
				desc: build_native_desc(syms, name, spec.members, spec.statics),
				new: spec.new,
				members: build_native_members(syms, spec.members),
				statics: build_native_members(syms, spec.statics),
			});
			self.vals
				.push(Rc::new(RefCell::new(Obj::Type(TypeId::Native(id)))));
			self.names.insert((pkg, name), id);
		}
	}

	pub fn descs_mut(&mut self, pkg: PackageId) -> HashMap<Sym, &mut types::NativeType> {
		let names: FxHashMap<usize, Sym> = self
			.names
			.iter()
			.filter(|((name_pkg, _), _)| *name_pkg == pkg)
			.map(|((_, name), id)| (id.index(), *name))
			.collect();
		self.types
			.iter_mut()
			.enumerate()
			.filter_map(|(index, typ)| names.get(&index).map(|name| (*name, &mut typ.desc)))
			.collect()
	}

	// Which native type 'name' declares in 'pkg', if any.
	pub fn id(&self, pkg: PackageId, name: Sym) -> Option<NativeTypeId> {
		self.names.get(&(pkg, name)).copied()
	}

	// Every native type registered to 'pkg', in registration order.
	pub fn ids(&self, pkg: PackageId) -> Vec<NativeTypeId> {
		let mut ids: Vec<_> = self
			.names
			.iter()
			.filter(|((name_pkg, _), _)| *name_pkg == pkg)
			.map(|(_, id)| *id)
			.collect();
		ids.sort_by_key(|id| id.index());
		ids
	}

	pub fn get(&self, id: NativeTypeId) -> &NativeType {
		&self.types[id.index()]
	}

	pub fn get_mut(&mut self, id: NativeTypeId) -> &mut NativeType {
		&mut self.types[id.index()]
	}

	pub fn val(&self, id: NativeTypeId) -> Rc<RefCell<Obj>> {
		self.vals[id.index()].clone()
	}
}

fn build_native_desc(
	syms: &mut Interner,
	name: Sym,
	members: &[NativeMemberSpec],
	statics: &[NativeMemberSpec],
) -> types::NativeType {
	types::NativeType {
		name,
		native_members: build_native_params(syms, members),
		native_statics: build_native_params(syms, statics),
		members: HashMap::new(),
		statics: HashMap::new(),
		impls: Vec::new(),
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
	// its index here.
	user: Vec<UserType>,
}

impl<'descs> Types<'descs> {
	// Create a runtime representation of every type, storing a canonical Val
	// for every type and creating an unbound proc for every method.
	pub fn new(
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
					types::Static::Native => panic!(),
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

		Self { descs, user }
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
			TypeId::Native(id) => syms.resolve(pkgs.native(id).desc.name).to_string(),
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

	pub fn member(&self, pkgs: &Packages, val: &Val, name: Sym) -> Option<Member> {
		let type_id = val.type_id();
		let namespace_type_id = val.namespace_type_id();
		if type_id != namespace_type_id {
			// type_id and namespace_type_id only ever differ when the Val is
			// itself a Type.
			let Val::Obj(rf) = val else {
				panic!();
			};
			let has_static = match namespace_type_id {
				TypeId::User(pkg, id) => pkgs.get(pkg).types.static_(id, name).is_some(),
				TypeId::Native(id) => pkgs.native(id).has_static(name),
			};
			if has_static {
				Some(Member::Static(rf.clone(), name))
			} else {
				let TypeId::Native(id) = type_id else {
					panic!();
				};
				if pkgs.native(id).has_member(name) {
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
					if pkgs.native(id).has_member(name) {
						Some(Member::Native(val.clone(), name))
					} else {
						None
					}
				}
			}
		}
	}
}

// The methods an 'extern' declaration adds to the native type it names.
pub struct ExternMembers {
	pub members: FxHashMap<Sym, NativeMember>,
	pub statics: FxHashMap<Sym, NativeMember>,
}

// Build the runtime half of every 'extern type' in a package. The native half
// is already in each native type's maps, put there by registration; this
// produces only what the declaration adds, for the caller to merge in.
pub fn build_extern_members<'descs>(
	pkgs: &Packages<'descs>,
	pkg_id: PackageId,
	pkg: &'descs Package,
	scopes: &Scopes,
) -> Vec<(NativeTypeId, ExternMembers)> {
	let mut externs = Vec::new();
	for native_id in pkgs.native_ids(pkg_id) {
		let desc = &pkgs.native(native_id).desc;

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

		externs.push((native_id, ExternMembers { members, statics }));
	}
	externs
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
