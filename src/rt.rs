use std::cell::{Cell, RefCell};
use std::cmp::{Eq, PartialEq};
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use ordermap::OrderMap;
use rustc_hash::{FxBuildHasher, FxHashMap};

use crate::intern::{CORE_TYPE_NAMES, Interner, Sym};
use crate::pkg::Package;
use crate::sem::types::{self, MemberSite};
use crate::src::{Location, Span};
use crate::syn::{
	self, BinaryOp, BlockId, Builtin, Chunk, ChunkId, Expr, ExprId, Lit, ModuleItem, ModuleItemId,
	Param, TypeItem, TypeItemId, UnaryOp,
};

#[derive(Debug)]
pub enum Error {
	ProtocolError(ProtocolError),
	ArgumentError(ArgumentError),
	TypeError(TypeError),
	MemberError(MemberError),
	IndexError(IndexError),
	NameError(String),
	KeyError(String),
}

#[derive(Debug)]
pub enum ProtocolError {
	NotIterable(String),
	NotAccessible(String),
	NotAppendable(String),
	NotOrderable(String),
}

#[derive(Debug)]
pub enum ArgumentError {
	Missing(Vec<String>),
	TooMany(usize, usize),
	Unknown(String),
	Duplicate(String),
}

#[derive(Debug)]
pub enum TypeError {
	IndexNonNum(String),
	ArithNonNum(String),
	ConcatNonStr(String),
	NotCallable(String),
	NotConstructible(String),
	NotInvokable(String),
	CaseNonType(String),
}

#[derive(Debug)]
pub enum MemberError {
	Missing(String, String),
	ReadOnly(String, String),
}

#[derive(Debug)]
pub enum IndexError {
	OutOfRange(f64),
	NonIntegral(f64),
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::ProtocolError(err) => Display::fmt(err, f),
			Self::ArgumentError(err) => Display::fmt(err, f),
			Self::TypeError(err) => Display::fmt(err, f),
			Self::MemberError(err) => Display::fmt(err, f),
			Self::IndexError(err) => Display::fmt(err, f),
			Self::NameError(name) => write!(f, "name '{}' is not defined", name),
			Self::KeyError(key) => write!(f, "key {} not found", key),
		}
	}
}

impl Display for ProtocolError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::NotIterable(type_name) => write!(f, "type {} is not iterable", type_name),
			Self::NotAccessible(type_name) => write!(f, "type {} is not accessible", type_name),
			Self::NotAppendable(type_name) => write!(f, "type {} is not appendable", type_name),
			Self::NotOrderable(type_name) => write!(f, "type {} is not orderable", type_name),
		}
	}
}

impl Display for ArgumentError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::Missing(names) => {
				let noun = if names.len() == 1 { "arg" } else { "args" };
				let names = names
					.iter()
					.map(|name| format!("'{}'", name))
					.collect::<Vec<_>>()
					.join(", ");
				write!(f, "missing {} {}", noun, names)
			}
			Self::TooMany(have, want) => {
				write!(f, "too many args; have {}, want at most {}", have, want)
			}
			Self::Unknown(name) => write!(f, "no param named '{}'", name),
			Self::Duplicate(name) => write!(f, "arg '{}' given twice", name),
		}
	}
}

impl Display for TypeError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::IndexNonNum(val) => write!(f, "index {} is not a number", val),
			Self::ArithNonNum(type_name) => {
				write!(f, "type {} cannot be used in arithmetic", type_name)
			}
			Self::ConcatNonStr(type_name) => write!(f, "type {} cannot be concatenated", type_name),
			Self::NotCallable(type_name) => write!(f, "type {} is not callable", type_name),
			Self::NotConstructible(type_name) => {
				write!(f, "type {} cannot be constructed", type_name)
			}
			Self::NotInvokable(type_name) => write!(f, "type {} is not invokable", type_name),
			Self::CaseNonType(type_name) => {
				write!(f, "type {} cannot be matched against", type_name)
			}
		}
	}
}

impl Display for MemberError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::Missing(type_name, name) => {
				write!(f, "type {} has no such member '{}'", type_name, name)
			}
			Self::ReadOnly(type_name, name) => {
				write!(f, "member '{}' on type {} is read-only", name, type_name)
			}
		}
	}
}

impl Display for IndexError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::OutOfRange(idx) => write!(f, "index {} is out of bounds", idx),
			Self::NonIntegral(idx) => write!(f, "index {} is not a whole number", idx),
		}
	}
}

#[derive(Debug, Clone, Copy)]
struct Num(f64);

impl Num {
	fn new() -> Val {
		Val::Num(Num(0.0))
	}
}

#[derive(Debug, Clone, Copy)]
struct Bool(bool);

impl Bool {
	fn new() -> Val {
		Val::Bool(Bool(false))
	}
}

#[derive(Debug, Clone, Copy)]
struct Char(char);

#[derive(Debug)]
struct Str {
	text: Box<str>,
	size: Cell<Option<f64>>,
}

impl Str {
	fn new() -> Val {
		Val::Str(Rc::new(Str {
			text: Box::from(""),
			size: Cell::new(None),
		}))
	}

	fn size(val: &Val, _args: Vec<Val>) -> Result<Val, Error> {
		let Val::Str(str) = val else { panic!() };
		let size = match str.size.get() {
			Some(size) => size,
			None => {
				let size = str.text.chars().count() as f64;
				str.size.set(Some(size));
				size
			}
		};
		Ok(Val::Num(Num(size)))
	}

	fn chars(val: &Val, _args: Vec<Val>) -> Result<Val, Error> {
		let Val::Str(str) = val else { panic!() };
		let items = str.text.chars().map(|c| Val::Char(Char(c))).collect();
		Ok(Val::Obj(Rc::new(RefCell::new(Obj::List(List { items })))))
	}
}

#[derive(Debug, Clone, Copy)]
struct Nil;

impl Nil {
	fn new() -> Val {
		Val::Nil(Nil)
	}
}

#[derive(Debug, Clone)]
enum Val {
	Num(Num),
	Bool(Bool),
	Char(Char),
	Str(Rc<Str>),
	Obj(Rc<RefCell<Obj>>),
	Nil(Nil),
}

impl Val {
	fn type_id(&self) -> TypeId {
		match self {
			Val::Num(_) => TypeId::Native(NativeTypeId::NUM),
			Val::Bool(_) => TypeId::Native(NativeTypeId::BOOL),
			Val::Char(_) => TypeId::Native(NativeTypeId::CHAR),
			Val::Str(_) => TypeId::Native(NativeTypeId::STR),
			Val::Obj(rf) => rf.borrow().type_id(),
			Self::Nil(_) => TypeId::Native(NativeTypeId::NIL),
		}
	}

	fn namespace_type_id(&self) -> TypeId {
		match self {
			Val::Obj(rf) => match &*rf.borrow() {
				Obj::Type(id) => *id,
				obj => obj.type_id(),
			},
			val => val.type_id(),
		}
	}

	fn is_truthy(&self) -> bool {
		match self {
			Val::Bool(bool) => bool.0,
			Val::Nil(_) => false,
			_ => true,
		}
	}
}

impl PartialEq for Val {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Num(num), Self::Num(other_num)) => num.0 == other_num.0,
			(Self::Bool(bool), Self::Bool(other_bool)) => bool.0 == other_bool.0,
			(Self::Char(char), Self::Char(other_char)) => char.0 == other_char.0,
			(Self::Str(str), Self::Str(other_str)) => str.text == other_str.text,
			(Self::Obj(rf), Self::Obj(other_rf)) => rf.as_ptr() == other_rf.as_ptr(),
			(Self::Nil(_), Self::Nil(_)) => true,
			_ => false,
		}
	}
}

impl Eq for Val {}

impl Hash for Val {
	fn hash<H: Hasher>(&self, state: &mut H) {
		match self {
			Self::Num(num) => num.0.to_bits().hash(state),
			Self::Bool(bool) => bool.0.hash(state),
			Self::Char(char) => char.0.hash(state),
			Self::Str(str) => str.text.hash(state),
			Self::Obj(rf) => rf.as_ptr().hash(state),
			Self::Nil(_) => 0_u8.hash(state),
		}
	}
}

#[derive(Debug)]
enum Obj {
	List(List),
	Dict(Dict),
	Proc(Proc),
	Type(TypeId),
	Proto(types::ProtoId),
	Instance(Instance),
	Method(Method),
}

impl Obj {
	fn type_id(&self) -> TypeId {
		match self {
			Self::List(_) => TypeId::Native(NativeTypeId::LIST),
			Self::Dict(_) => TypeId::Native(NativeTypeId::DICT),
			Self::Proc(_) => TypeId::Native(NativeTypeId::PROC),
			Self::Type(_) => TypeId::Native(NativeTypeId::TYPE),
			Self::Proto(_) => TypeId::Native(NativeTypeId::PROTO),
			Self::Instance(inst) => TypeId::User(inst.typ),
			Self::Method(_) => TypeId::Native(NativeTypeId::PROC),
		}
	}
}

#[derive(Debug)]
struct List {
	items: Vec<Val>,
}

impl List {
	fn new() -> Val {
		Val::Obj(Rc::new(RefCell::new(Obj::List(List { items: Vec::new() }))))
	}

	fn size(val: &Val, _args: Vec<Val>) -> Result<Val, Error> {
		let Val::Obj(rf) = val else { panic!() };
		let Obj::List(list) = &*rf.borrow() else {
			panic!()
		};
		Ok(Val::Num(Num(list.items.len() as f64)))
	}
}

#[derive(Debug)]
struct Dict {
	pairs: OrderMap<Val, Val>,
}

impl Dict {
	fn new() -> Val {
		Val::Obj(Rc::new(RefCell::new(Obj::Dict(Dict {
			pairs: OrderMap::new(),
		}))))
	}

	fn size(val: &Val, _args: Vec<Val>) -> Result<Val, Error> {
		let Val::Obj(rf) = val else { panic!() };
		let Obj::Dict(dict) = &*rf.borrow() else {
			panic!()
		};
		Ok(Val::Num(Num(dict.pairs.len() as f64)))
	}
}

#[derive(Debug)]
struct Proc {
	name: Sym,
	params: Vec<Param>,
	body: BlockId,
	chunk: ChunkId,
	scope: Rc<RefCell<Scope>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeId {
	User(types::TypeId),
	Native(NativeTypeId),
}

// Data the runtime attaches to the type descriptor, the latter having already
// computed most of the important information.
#[derive(Debug)]
struct UserType {
	// The type's canonical value.
	val: Rc<RefCell<Obj>>,
	// Every member reachable on an instance, method precedence already
	// accounted for by the descriptor's method table.
	methods: FxHashMap<Sym, Rc<RefCell<Proc>>>,
	statics: FxHashMap<Sym, Static>,
}

#[derive(Debug, Clone)]
enum Static {
	Proc(Rc<RefCell<Proc>>),
	Type(types::TypeId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeTypeId(u32);

#[derive(Debug)]
struct NativeParam {
	name: &'static str,
	default: Option<fn() -> Val>,
}

#[derive(Debug, Clone)]
struct NativeMember {
	params: &'static [NativeParam],
	call: fn(&Val, Vec<Val>) -> Result<Val, Error>,
}

impl NativeMember {
	fn defaults(&self) -> Option<Vec<Val>> {
		self.params
			.iter()
			.map(|param| param.default.map(|default| default()))
			.collect()
	}
}

#[derive(Debug)]
struct NativeType {
	name: Sym,
	new: Option<fn() -> Val>,
	members: FxHashMap<Sym, NativeMember>,
}

impl NativeType {
	fn has_member(&self, name: Sym) -> bool {
		self.members.contains_key(&name)
	}
}

const CORE_TYPES: &[(
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
];

impl NativeTypeId {
	const NIL: NativeTypeId = NativeTypeId(0);
	const NUM: NativeTypeId = NativeTypeId(1);
	const BOOL: NativeTypeId = NativeTypeId(2);
	const CHAR: NativeTypeId = NativeTypeId(3);
	const STR: NativeTypeId = NativeTypeId(4);
	const LIST: NativeTypeId = NativeTypeId(5);
	const DICT: NativeTypeId = NativeTypeId(6);
	const PROC: NativeTypeId = NativeTypeId(7);
	const TYPE: NativeTypeId = NativeTypeId(8);
	const PROTO: NativeTypeId = NativeTypeId(9);
}

// The built-in types, interned once at startup. Natives have no declaration
// and so no description; this table is all there is of them.
fn native_types(syms: &mut Interner) -> Vec<NativeType> {
	let mut native = Vec::with_capacity(CORE_TYPES.len());
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
		native.push(NativeType {
			name: syms.intern(CORE_TYPE_NAMES[id.0 as usize]),
			new: *new,
			members,
		});
	}
	native
}

// The runtime image of every type the program has: what the package described,
// what running those descriptions takes, and the built-ins no description
// covers.
struct Types<'descs> {
	descs: &'descs types::Types,
	// Indexed in step with the description arena, so a type's index there is
	// its index here.
	user: Vec<UserType>,
	native: Vec<NativeType>,
}

impl<'descs> Types<'descs> {
	// Create a runtime representation of every type, storing a canonical Val
	// for every type and creating an unbound proc for every method.
	fn new(
		native: Vec<NativeType>,
		pkg: &Package,
		descs: &'descs types::Types,
		scopes: &Scopes,
	) -> Self {
		let mut user = Vec::with_capacity(descs.ids().len());
		for id in descs.ids() {
			let desc = descs.get_type(id);
			let mut methods = FxHashMap::default();
			for (name, site) in &desc.members {
				methods.insert(*name, member_proc(pkg, scopes, *site));
			}
			let mut statics = FxHashMap::default();
			for (name, static_) in &desc.statics {
				let static_ = match static_ {
					types::Static::Type(id) => Static::Type(*id),
					types::Static::Proc(chunk_id, item_id) => {
						Static::Proc(static_proc(pkg, scopes, *chunk_id, *item_id))
					}
				};
				statics.insert(*name, static_);
			}
			user.push(UserType {
				val: Rc::new(RefCell::new(Obj::Type(TypeId::User(id)))),
				methods,
				statics,
			});
		}

		Self {
			descs,
			user,
			native,
		}
	}

	// Get a type's *qualified* name (including all lexical nesting).
	fn name(&self, syms: &Interner, id: TypeId) -> String {
		match id {
			TypeId::User(id) => {
				let desc = self.descs.get_type(id);
				match desc.enclosing {
					Some(outer) => {
						let outer = self.name(syms, TypeId::User(outer));
						format!("{}.{}", outer, syms.resolve(desc.name))
					}
					None => syms.resolve(desc.name).to_string(),
				}
			}
			TypeId::Native(id) => syms.resolve(self.native(id).name).to_string(),
		}
	}

	fn user(&self, id: types::TypeId) -> &UserType {
		&self.user[id.index()]
	}

	fn method(&self, id: types::TypeId, name: Sym) -> Option<Rc<RefCell<Proc>>> {
		self.user(id).methods.get(&name).cloned()
	}

	fn static_(&self, id: types::TypeId, name: Sym) -> Option<Static> {
		self.user(id).statics.get(&name).cloned()
	}

	fn field(&self, id: types::TypeId, name: Sym) -> bool {
		let desc = self.descs.get_type(id);
		desc.ctor_fields.iter().any(|param| param.name == name)
			|| desc.body_fields.contains_key(&name)
	}

	fn native(&self, id: NativeTypeId) -> &NativeType {
		&self.native[id.0 as usize]
	}

	fn member(&self, val: &Val, name: Sym) -> Option<Member> {
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
				TypeId::User(id) => self.static_(id, name).is_some(),
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
				TypeId::User(id) => {
					let Val::Obj(rf) = val else {
						panic!();
					};
					if self.field(id, name) || self.method(id, name).is_some() {
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
fn member_proc(pkg: &Package, scopes: &Scopes, site: MemberSite) -> Rc<RefCell<Proc>> {
	match site {
		MemberSite::Declared(chunk_id, item_id) => {
			// Protocols don't involve static methods; assert that this is an
			// instance method.
			let TypeItem::Method(syn::Method::Instance(def)) =
				pkg.get_chunk(chunk_id).get_type_item(item_id)
			else {
				panic!()
			};
			Rc::new(RefCell::new(Proc {
				name: def.name,
				params: def.params.to_vec(),
				body: def.body,
				chunk: chunk_id,
				scope: scopes.module(chunk_id),
			}))
		}
		MemberSite::Provided(chunk_id, item_id) => {
			let item = pkg.get_chunk(chunk_id).get_proto_item(item_id);
			Rc::new(RefCell::new(Proc {
				name: item.def.name,
				params: item.def.params.to_vec(),
				body: item.def.body,
				chunk: chunk_id,
				scope: scopes.module(chunk_id),
			}))
		}
	}
}

fn static_proc(
	pkg: &Package,
	scopes: &Scopes,
	chunk_id: ChunkId,
	item_id: TypeItemId,
) -> Rc<RefCell<Proc>> {
	let TypeItem::Method(syn::Method::Static(def)) = pkg.get_chunk(chunk_id).get_type_item(item_id)
	else {
		panic!()
	};
	Rc::new(RefCell::new(Proc {
		name: def.name,
		params: def.params.to_vec(),
		body: def.body,
		chunk: chunk_id,
		scope: scopes.module(chunk_id),
	}))
}

#[derive(Debug)]
struct Instance {
	typ: types::TypeId,
	fields: FxHashMap<Sym, Val>,
}

#[derive(Debug)]
enum Member {
	Static(Rc<RefCell<Obj>>, Sym),
	User(Rc<RefCell<Obj>>, Sym),
	Native(Val, Sym),
}

#[derive(Debug)]
enum Method {
	User(Rc<RefCell<Obj>>, Rc<RefCell<Proc>>),
	Native(Val, Sym, NativeMember),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tier {
	Local,
	Module,
	Prelude,
}

#[derive(Debug)]
struct Scope {
	locals: FxHashMap<Sym, Val>,
	outer: Option<Rc<RefCell<Scope>>>,
	tier: Tier,
}

struct Local {
	name: Sym,
	scope: Rc<RefCell<Scope>>,
	tier: Tier,
}

impl Local {
	fn is_bound(&self) -> bool {
		self.scope.borrow().locals.contains_key(&self.name)
	}

	fn get(&self) -> Val {
		self.scope.borrow().locals.get(&self.name).cloned().unwrap()
	}

	fn set(&self, val: Val) {
		self.scope.borrow_mut().locals.insert(self.name, val);
	}
}

// Somewhere a value can be read from and written to. Either a name in a scope
// or a member of a receiver.
enum Place {
	Local(Local),
	Member(Member),
}

impl Place {
	// Whether this place holds a value yet. A member always does, because it
	// would not have resolved otherwise, but a local may have not been written
	// yet.
	fn is_bound(&self) -> bool {
		match self {
			Place::Local(local) => local.is_bound(),
			Place::Member(_) => true,
		}
	}

	fn get(&self, types: &Types) -> Result<Val, Error> {
		match self {
			Place::Local(local) => Ok(local.get()),
			Place::Member(Member::Static(type_rf, name)) => {
				// A static is only ever found on a user type.
				let Obj::Type(TypeId::User(type_id)) = *type_rf.borrow() else {
					panic!();
				};
				match types.static_(type_id, *name).unwrap() {
					Static::Proc(proc_rf) => Ok(Val::Obj(Rc::new(RefCell::new(Obj::Method(
						Method::User(type_rf.clone(), proc_rf),
					))))),
					Static::Type(id) => Ok(Val::Obj(types.user(id).val.clone())),
				}
			}
			Place::Member(Member::User(inst_rf, name)) => {
				let Obj::Instance(inst) = &*inst_rf.borrow() else {
					panic!()
				};
				if let Some(val) = inst.fields.get(name) {
					Ok(val.clone())
				} else if let Some(proc_rf) = types.method(inst.typ, *name) {
					Ok(Val::Obj(Rc::new(RefCell::new(Obj::Method(Method::User(
						inst_rf.clone(),
						proc_rf,
					))))))
				} else {
					panic!();
				}
			}
			Place::Member(Member::Native(recv, name)) => {
				let TypeId::Native(type_id) = recv.type_id() else {
					panic!();
				};
				match types.native(type_id).members.get(name) {
					Some(member) => Ok(Val::Obj(Rc::new(RefCell::new(Obj::Method(
						Method::Native(recv.clone(), *name, member.clone()),
					))))),
					None => panic!(),
				}
			}
		}
	}

	fn set(&self, types: &Types, val: Val) -> Result<(), ()> {
		match self {
			Place::Local(local) => {
				local.set(val);
				Ok(())
			}
			Place::Member(Member::Static(_, _)) => Err(()),
			Place::Member(Member::User(inst_rf, name)) => {
				let Obj::Instance(inst) = &mut *inst_rf.borrow_mut() else {
					panic!()
				};
				// A method cannot be replaced by a field of the same name.
				if !inst.fields.contains_key(name) && types.method(inst.typ, *name).is_some() {
					return Err(());
				}
				inst.fields.insert(*name, val);
				Ok(())
			}
			Place::Member(Member::Native(_, _)) => Err(()),
		}
	}
}

impl Scope {
	fn local(origin: &Rc<RefCell<Scope>>, name: Sym) -> Local {
		let mut scope = origin.clone();
		loop {
			if scope.borrow().locals.contains_key(&name) {
				let tier = scope.borrow().tier;
				return Local { name, scope, tier };
			}
			let outer = scope.borrow().outer.clone();
			if let Some(outer) = outer {
				scope = outer;
			} else {
				return Local {
					name,
					scope: origin.clone(),
					tier: origin.borrow().tier,
				};
			}
		}
	}
}

// The scope for every module, keyed by the module's ChunkId.
struct Scopes {
	modules: Vec<Rc<RefCell<Scope>>>,
}

impl Scopes {
	fn new(prelude: Rc<RefCell<Scope>>, chunks: usize) -> Self {
		let mut modules = Vec::with_capacity(chunks);
		for _ in 0..chunks {
			let scope = Rc::new(RefCell::new(Scope {
				locals: FxHashMap::default(),
				outer: Some(prelude.clone()),
				tier: Tier::Module,
			}));
			modules.push(scope);
		}
		Self { modules }
	}

	fn module(&self, chunk: ChunkId) -> Rc<RefCell<Scope>> {
		self.modules[chunk.index()].clone()
	}
}

pub struct Interpreter<'syms, 'pkg, 'descs> {
	syms: &'syms Interner,
	pkg: &'pkg Package,
	types: Types<'descs>,
	// The chunk that evaluation starts from.
	root: ChunkId,
	scopes: Scopes,
	// The scope the walk is currently in.
	scope: Rc<RefCell<Scope>>,
	receiver: Option<Val>,
}

enum Signal {
	Return(Val),
	Break(Val),
	Error(Error, Vec<(String, Location)>),
}

impl<'syms, 'pkg, 'descs> Interpreter<'syms, 'pkg, 'descs> {
	pub fn new(
		syms: &'syms mut Interner,
		pkg: &'pkg Package,
		descs: &'descs types::Types,
		root: ChunkId,
	) -> Self {
		let native = native_types(syms);

		let prelude = {
			let mut locals = HashMap::with_capacity_and_hasher(CORE_TYPES.len(), FxBuildHasher);
			for (id, _, _) in CORE_TYPES {
				let typ = &native[id.0 as usize];
				let obj = Obj::Type(TypeId::Native(*id));
				locals.insert(typ.name, Val::Obj(Rc::new(RefCell::new(obj))));
			}
			Rc::new(RefCell::new(Scope {
				locals,
				outer: None,
				tier: Tier::Prelude,
			}))
		};

		let scopes = Scopes::new(prelude, pkg.chunk_ids().len());
		let types = Types::new(native, pkg, descs, &scopes);

		Self {
			syms,
			pkg,
			types,
			root,
			scope: scopes.module(root),
			scopes,
			receiver: None,
		}
	}

	pub fn eval(mut self) -> Result<(), (Error, Vec<(String, Location)>)> {
		let chunk_id = self.root;
		let chunk = self.pkg.get_chunk(chunk_id);
		for item_id in &chunk.top {
			match self.eval_module_item(chunk, chunk_id, *item_id) {
				Ok(()) => {}
				Err(Signal::Return(_)) => break,
				Err(Signal::Break(_)) => panic!(),
				Err(Signal::Error(err, trace)) => return Err((err, trace)),
			}
		}
		Ok(())
	}

	fn eval_module_item(
		&mut self,
		chunk: &Chunk,
		chunk_id: ChunkId,
		item_id: ModuleItemId,
	) -> Result<(), Signal> {
		match chunk.get_module_item(item_id) {
			ModuleItem::Module(_) => Ok(()),
			ModuleItem::Import(_) => Ok(()),
			ModuleItem::Export(_) => Ok(()),
			ModuleItem::Type(typ) => {
				let id = self.types.descs.get_type_by_item(chunk_id, item_id);
				let val = Val::Obj(self.types.user(id).val.clone());
				self.scope.borrow_mut().locals.insert(typ.name, val);
				Ok(())
			}
			ModuleItem::Proto(proto) => {
				let id = self.types.descs.get_proto_by_item(chunk_id, item_id);
				let val = Val::Obj(Rc::new(RefCell::new(Obj::Proto(id))));
				self.scope.borrow_mut().locals.insert(proto.name, val);
				Ok(())
			}
			ModuleItem::Def(def) => {
				let obj = Obj::Proc(Proc {
					name: def.name,
					params: def.params.to_vec(),
					body: def.body,
					chunk: chunk_id,
					scope: self.scope.clone(),
				});
				let val = Val::Obj(Rc::new(RefCell::new(obj)));
				self.scope.borrow_mut().locals.insert(def.name, val);
				Ok(())
			}
			ModuleItem::Expr(expr_id) => {
				self.eval_expr(chunk, *expr_id)?;
				Ok(())
			}
		}
	}

	fn eval_expr(&mut self, chunk: &Chunk, expr_id: ExprId) -> Result<Val, Signal> {
		match chunk.get_expr(expr_id) {
			Expr::Each(each) => match self.eval_expr(chunk, each.iter)? {
				Val::Str(str) => {
					for char in str.text.chars() {
						let scope = Rc::new(RefCell::new(Scope {
							locals: FxHashMap::default(),
							outer: Some(self.scope.clone()),
							tier: Tier::Local,
						}));
						scope
							.borrow_mut()
							.locals
							.insert(each.item, Val::Char(Char(char)));
						match self.eval_block(chunk, scope, each.body) {
							Ok(_) => {}
							Err(Signal::Break(val)) => return Ok(val),
							Err(signal) => return Err(signal),
						}
					}
					Ok(Val::Nil(Nil))
				}
				Val::Obj(rf) => match &*rf.borrow() {
					Obj::List(list) => {
						for item in &list.items {
							let scope = Rc::new(RefCell::new(Scope {
								locals: FxHashMap::default(),
								outer: Some(self.scope.clone()),
								tier: Tier::Local,
							}));
							scope.borrow_mut().locals.insert(each.item, item.clone());
							match self.eval_block(chunk, scope, each.body) {
								Ok(_) => {}
								Err(Signal::Break(val)) => return Ok(val),
								Err(signal) => return Err(signal),
							}
						}
						Ok(Val::Nil(Nil))
					}
					Obj::Dict(dict) => {
						for key in dict.pairs.keys() {
							let scope = Rc::new(RefCell::new(Scope {
								locals: FxHashMap::default(),
								outer: Some(self.scope.clone()),
								tier: Tier::Local,
							}));
							scope.borrow_mut().locals.insert(each.item, key.clone());
							match self.eval_block(chunk, scope, each.body) {
								Ok(_) => {}
								Err(Signal::Break(val)) => return Ok(val),
								Err(signal) => return Err(signal),
							}
						}
						Ok(Val::Nil(Nil))
					}
					obj => {
						let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
						let type_name = self.types.name(self.syms, obj.type_id());
						Err(Signal::Error(
							Error::ProtocolError(ProtocolError::NotIterable(type_name)),
							vec![(String::new(), loc)],
						))
					}
				},
				val => {
					let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
					let type_name = self.types.name(self.syms, val.type_id());
					Err(Signal::Error(
						Error::ProtocolError(ProtocolError::NotIterable(type_name)),
						vec![(String::new(), loc)],
					))
				}
			},
			Expr::Loop(loop_) => loop {
				let scope = Rc::new(RefCell::new(Scope {
					locals: FxHashMap::default(),
					outer: Some(self.scope.clone()),
					tier: Tier::Local,
				}));
				match self.eval_block(chunk, scope, loop_.body) {
					Ok(_) => {}
					Err(Signal::Break(val)) => return Ok(val),
					Err(signal) => return Err(signal),
				}
			},
			Expr::When(when) => {
				let cond = self.eval_expr(chunk, when.cond)?.is_truthy();
				let scope = Rc::new(RefCell::new(Scope {
					locals: FxHashMap::default(),
					outer: Some(self.scope.clone()),
					tier: Tier::Local,
				}));
				if cond {
					self.eval_block(chunk, scope, when.then_branch)
				} else if let Some(else_branch) = when.else_branch {
					self.eval_block(chunk, scope, else_branch)
				} else {
					Ok(Val::Nil(Nil))
				}
			}
			Expr::Match(match_) => {
				let scrutinee_type_id = self.eval_expr(chunk, match_.scrutinee)?.type_id();
				for arm in &match_.arms {
					let path_val = self.eval_expr(chunk, arm.path)?;
					let arm_type_id = match &path_val {
						Val::Obj(rf) => match &*rf.borrow() {
							Obj::Type(id) => Some(*id),
							_ => None,
						},
						_ => None,
					};
					let Some(arm_type_id) = arm_type_id else {
						let loc = self.pkg.loc(chunk.get_expr_span(arm.path));
						let type_name = self.types.name(self.syms, path_val.type_id());
						return Err(Signal::Error(
							Error::TypeError(TypeError::CaseNonType(type_name)),
							vec![(String::new(), loc)],
						));
					};
					if arm_type_id == scrutinee_type_id {
						let scope = Rc::new(RefCell::new(Scope {
							locals: FxHashMap::default(),
							outer: Some(self.scope.clone()),
							tier: Tier::Local,
						}));
						return self.eval_block(chunk, scope, arm.body);
					}
				}
				if let Some(else_branch) = match_.else_branch {
					let scope = Rc::new(RefCell::new(Scope {
						locals: FxHashMap::default(),
						outer: Some(self.scope.clone()),
						tier: Tier::Local,
					}));
					self.eval_block(chunk, scope, else_branch)
				} else {
					Ok(Val::Nil(Nil))
				}
			}
			Expr::Return(return_) => {
				let val = if let Some(val_expr_id) = return_.val {
					self.eval_expr(chunk, val_expr_id)?
				} else {
					Val::Nil(Nil)
				};
				Err(Signal::Return(val))
			}
			Expr::Break(break_) => {
				let val = if let Some(val_expr_id) = break_.val {
					self.eval_expr(chunk, val_expr_id)?
				} else {
					Val::Nil(Nil)
				};
				Err(Signal::Break(val))
			}
			Expr::Call(call) => {
				let span = chunk.get_expr_span(expr_id);
				let mut args = Vec::with_capacity(call.args.len());
				for arg in &call.args {
					let val = self.eval_expr(chunk, arg.val)?;
					args.push((arg.name, val));
				}
				let callee = self.eval_expr_raw(chunk, call.callee)?;
				match callee {
					Val::Obj(rf) => match &*rf.borrow() {
						Obj::Proc(proc) => self.eval_proc_call(span, proc, None, args),
						Obj::Type(type_id) => match type_id {
							TypeId::User(type_id) => {
								let type_id = *type_id;
								// The description outlives the interpreter, so a type's
								// fields are read where they were written rather than
								// copied onto every construction.
								let desc = self.types.descs.get_type(type_id);
								// A type with variants cannot itself be constructed.
								if let Some(variants) = &desc.variants
									&& !variants.is_empty()
								{
									let loc = self.pkg.loc(span);
									let type_name =
										self.types.name(self.syms, TypeId::User(type_id));
									return Err(Signal::Error(
										Error::TypeError(TypeError::NotConstructible(type_name)),
										vec![(String::new(), loc)],
									));
								}
								let ctor_fields = &desc.ctor_fields;
								let body_fields = &desc.body_fields;
								let type_chunk_id = desc.chunk;
								let type_scope = self.scopes.module(type_chunk_id);

								let slots = self.slot_args(span, ctor_fields, args)?;
								let scope = Rc::new(RefCell::new(Scope {
									locals: FxHashMap::default(),
									outer: Some(type_scope.clone()),
									tier: Tier::Local,
								}));
								// The instance does not exist yet, so a ctor default sees the
								// fields to its left and the module, never self.
								let outer_receiver = self.receiver.take();
								let bound =
									self.bind_args(type_chunk_id, ctor_fields, slots, &scope);
								self.receiver = outer_receiver;
								bound?;
								// The scope was just used for initialization, nothing else holds
								// a reference to it.
								let fields = Rc::try_unwrap(scope).unwrap().into_inner().locals;

								let type_chunk = self.pkg.get_chunk(type_chunk_id);
								let inst_rf = Rc::new(RefCell::new(Obj::Instance(Instance {
									typ: type_id,
									fields,
								})));

								let saved_scope = self.scope.clone();
								let saved_receiver = self.receiver.clone();
								self.receiver = Some(Val::Obj(inst_rf.clone()));
								for (name, init_id) in body_fields {
									self.scope = Rc::new(RefCell::new(Scope {
										locals: FxHashMap::default(),
										outer: Some(type_scope.clone()),
										tier: Tier::Local,
									}));
									let val = match self.eval_expr(type_chunk, *init_id) {
										Ok(val) => val,
										Err(signal) => {
											self.scope = saved_scope;
											self.receiver = saved_receiver;
											return Err(signal);
										}
									};
									Place::Member(Member::User(inst_rf.clone(), *name))
										.set(&self.types, val)
										.unwrap();
								}
								self.scope = saved_scope;
								self.receiver = saved_receiver;

								Ok(Val::Obj(inst_rf))
							}
							TypeId::Native(type_id) => {
								let typ = self.types.native(*type_id);
								match typ.new {
									Some(new) => {
										// A native constructor defines no
										// params by definition, so route
										// through the usual machinery.
										self.bind_native_args(span, &[], args)?;
										Ok(new())
									}
									None => {
										let loc = self.pkg.loc(span);
										let type_name = self.syms.resolve(typ.name);
										Err(Signal::Error(
											Error::TypeError(TypeError::NotConstructible(
												type_name.to_string(),
											)),
											vec![(String::new(), loc)],
										))
									}
								}
							}
						},
						Obj::Method(meth) => match meth {
							Method::User(inst, proc) => self.eval_proc_call(
								span,
								&proc.borrow(),
								Some(Val::Obj(inst.clone())),
								args,
							),
							Method::Native(recv, _name, meth) => {
								// Native params carry &'static str names and
								// fn() -> Val defaults, so they are matched and
								// filled without an Interner or a scope.
								let args = self.bind_native_args(span, meth.params, args)?;
								match (meth.call)(recv, args) {
									Ok(val) => Ok(val),
									Err(err) => {
										let loc = self.pkg.loc(span);
										Err(Signal::Error(err, vec![(String::new(), loc)]))
									}
								}
							}
						},
						obj => {
							let loc = self.pkg.loc(span);
							let type_name = self.types.name(self.syms, obj.type_id());
							Err(Signal::Error(
								Error::TypeError(TypeError::NotCallable(type_name)),
								vec![(String::new(), loc)],
							))
						}
					},
					val => {
						let loc = self.pkg.loc(span);
						let type_name = self.types.name(self.syms, val.type_id());
						Err(Signal::Error(
							Error::TypeError(TypeError::NotCallable(type_name)),
							vec![(String::new(), loc)],
						))
					}
				}
			}
			Expr::Member(member) => {
				let val = self.eval_member_raw(chunk, expr_id, member.receiver, member.name)?;
				self.invoke_or_return(chunk, expr_id, val)
			}
			Expr::Access(access) => match self.eval_expr(chunk, access.receiver)? {
				Val::Obj(rf) => match &*rf.borrow() {
					Obj::List(list) => {
						let idx = self.eval_index(chunk, expr_id, access.key, list.items.len())?;
						Ok(list.items[idx].clone())
					}
					Obj::Dict(dict) => {
						let key = self.eval_expr(chunk, access.key)?;
						match dict.pairs.get(&key) {
							Some(val) => Ok(val.clone()),
							None => {
								let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
								Err(Signal::Error(
									Error::KeyError(rt_debug_val(self.syms, &self.types, &key)),
									vec![(String::new(), loc)],
								))
							}
						}
					}
					obj => {
						let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
						let type_name = self.types.name(self.syms, obj.type_id());
						Err(Signal::Error(
							Error::ProtocolError(ProtocolError::NotAccessible(type_name)),
							vec![(String::new(), loc)],
						))
					}
				},
				val => {
					let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
					let type_name = self.types.name(self.syms, val.type_id());
					Err(Signal::Error(
						Error::ProtocolError(ProtocolError::NotAccessible(type_name)),
						vec![(String::new(), loc)],
					))
				}
			},
			Expr::Mention(mention) => {
				let val = self.eval_expr_raw(chunk, mention.val)?;
				self.expect_invocable(chunk, expr_id, val)
			}
			Expr::Assign(assign) => {
				let val = self.eval_expr(chunk, assign.val)?;
				match &assign.place {
					syn::Place::Name(name) => {
						let sym = name.sym;
						if let Err(()) = self.resolve_name(sym).set(&self.types, val.clone()) {
							let type_name = self.types.name(
								self.syms,
								self.receiver.as_ref().unwrap().namespace_type_id(),
							);
							let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
							return Err(Signal::Error(
								Error::MemberError(MemberError::ReadOnly(
									type_name,
									self.syms.resolve(sym).to_string(),
								)),
								vec![(String::new(), loc)],
							));
						}
						Ok(val)
					}
					syn::Place::Member(member) => {
						let receiver = self.eval_expr(chunk, member.receiver)?;
						if let Some(m) = self.types.member(&receiver, member.name) {
							if let Err(()) = Place::Member(m).set(&self.types, val.clone()) {
								let type_name =
									self.types.name(self.syms, receiver.namespace_type_id());
								let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
								return Err(Signal::Error(
									Error::MemberError(MemberError::ReadOnly(
										type_name.to_string(),
										self.syms.resolve(member.name).to_string(),
									)),
									vec![(String::new(), loc)],
								));
							}
							Ok(val)
						} else {
							let type_name =
								self.types.name(self.syms, receiver.namespace_type_id());
							let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
							Err(Signal::Error(
								Error::MemberError(MemberError::Missing(
									type_name.to_string(),
									self.syms.resolve(member.name).to_string(),
								)),
								vec![(String::new(), loc)],
							))
						}
					}
					syn::Place::Access(access) => {
						let receiver = self.eval_expr(chunk, access.receiver)?;
						match &receiver {
							Val::Obj(rf) => match &mut *rf.borrow_mut() {
								Obj::List(list) => {
									let idx = self.eval_index(
										chunk,
										expr_id,
										access.key,
										list.items.len(),
									)?;
									list.items[idx] = val.clone();
									Ok(val)
								}
								Obj::Dict(dict) => {
									let key = self.eval_expr(chunk, access.key)?;
									dict.pairs.insert(key, val.clone());
									Ok(val)
								}
								obj => {
									let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
									let type_name = self.types.name(self.syms, obj.type_id());
									Err(Signal::Error(
										Error::ProtocolError(ProtocolError::NotAccessible(
											type_name,
										)),
										vec![(String::new(), loc)],
									))
								}
							},
							val => {
								let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
								let type_name = self.types.name(self.syms, val.type_id());
								Err(Signal::Error(
									Error::ProtocolError(ProtocolError::NotAccessible(type_name)),
									vec![(String::new(), loc)],
								))
							}
						}
					}
				}
			}
			Expr::Binary(binary) => match &binary.op {
				BinaryOp::Or => {
					let lhs = self.eval_expr(chunk, binary.lhs)?;
					if lhs.is_truthy() {
						Ok(lhs)
					} else {
						self.eval_expr(chunk, binary.rhs)
					}
				}
				BinaryOp::And => {
					let lhs = self.eval_expr(chunk, binary.lhs)?;
					if !lhs.is_truthy() {
						Ok(lhs)
					} else {
						self.eval_expr(chunk, binary.rhs)
					}
				}
				BinaryOp::Eq
				| BinaryOp::NotEq
				| BinaryOp::Lt
				| BinaryOp::Gt
				| BinaryOp::LtEq
				| BinaryOp::GtEq
				| BinaryOp::Append
				| BinaryOp::Add
				| BinaryOp::Sub
				| BinaryOp::Mul
				| BinaryOp::Div => {
					let lhs = self.eval_expr(chunk, binary.lhs)?;
					let rhs = self.eval_expr(chunk, binary.rhs)?;
					match &binary.op {
						BinaryOp::Eq => Ok(Val::Bool(Bool(lhs == rhs))),
						BinaryOp::NotEq => Ok(Val::Bool(Bool(lhs != rhs))),
						BinaryOp::Lt | BinaryOp::Gt | BinaryOp::LtEq | BinaryOp::GtEq => {
							match (lhs, rhs) {
								(Val::Num(lhs), Val::Num(rhs)) => {
									Ok(Val::Bool(Bool(match &binary.op {
										BinaryOp::Lt => lhs.0 < rhs.0,
										BinaryOp::Gt => lhs.0 > rhs.0,
										BinaryOp::LtEq => lhs.0 <= rhs.0,
										BinaryOp::GtEq => lhs.0 >= rhs.0,
										_ => panic!(),
									})))
								}
								(Val::Str(lhs), Val::Str(rhs)) => {
									Ok(Val::Bool(Bool(match &binary.op {
										BinaryOp::Lt => lhs.text < rhs.text,
										BinaryOp::Gt => lhs.text > rhs.text,
										BinaryOp::LtEq => lhs.text <= rhs.text,
										BinaryOp::GtEq => lhs.text >= rhs.text,
										_ => panic!(),
									})))
								}
								(Val::Char(lhs), Val::Char(rhs)) => {
									Ok(Val::Bool(Bool(match &binary.op {
										BinaryOp::Lt => lhs.0 < rhs.0,
										BinaryOp::Gt => lhs.0 > rhs.0,
										BinaryOp::LtEq => lhs.0 <= rhs.0,
										BinaryOp::GtEq => lhs.0 >= rhs.0,
										_ => panic!(),
									})))
								}
								(lhs, rhs) => {
									let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
									let val = match (&lhs, &rhs) {
										(Val::Num(_), _) => rhs,
										(_, Val::Num(_)) => lhs,
										_ => lhs,
									};
									let type_name = self.types.name(self.syms, val.type_id());
									Err(Signal::Error(
										Error::ProtocolError(ProtocolError::NotOrderable(
											type_name,
										)),
										vec![(String::new(), loc)],
									))
								}
							}
						}
						BinaryOp::Append => {
							let Val::Obj(rf) = lhs.clone() else {
								let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
								let type_name = self.types.name(self.syms, lhs.type_id());
								return Err(Signal::Error(
									Error::ProtocolError(ProtocolError::NotAppendable(type_name)),
									vec![(String::new(), loc)],
								));
							};
							let Obj::List(list) = &mut *rf.borrow_mut() else {
								let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
								let type_name = self.types.name(self.syms, rf.borrow().type_id());
								return Err(Signal::Error(
									Error::ProtocolError(ProtocolError::NotAppendable(type_name)),
									vec![(String::new(), loc)],
								));
							};
							list.items.push(rhs);
							Ok(lhs)
						}
						BinaryOp::Add => match (lhs, rhs) {
							(Val::Num(lhs), Val::Num(rhs)) => Ok(Val::Num(Num(lhs.0 + rhs.0))),
							(Val::Str(lhs), Val::Str(rhs)) => {
								let mut text =
									String::with_capacity(lhs.text.len() + rhs.text.len());
								text.push_str(&lhs.text);
								text.push_str(&rhs.text);
								Ok(Val::Str(Rc::new(Str {
									text: Box::from(text.as_str()),
									size: Cell::new(None),
								})))
							}
							(Val::Str(_), rhs) => {
								let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
								let type_name = self.types.name(self.syms, rhs.type_id());
								Err(Signal::Error(
									Error::TypeError(TypeError::ConcatNonStr(type_name)),
									vec![(String::new(), loc)],
								))
							}
							(_, rhs) => {
								let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
								let type_name = self.types.name(self.syms, rhs.type_id());
								Err(Signal::Error(
									Error::TypeError(TypeError::ArithNonNum(type_name)),
									vec![(String::new(), loc)],
								))
							}
						},
						BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => match (lhs, rhs) {
							(Val::Num(lhs), Val::Num(rhs)) => Ok(Val::Num(Num(match &binary.op {
								BinaryOp::Sub => lhs.0 - rhs.0,
								BinaryOp::Mul => lhs.0 * rhs.0,
								BinaryOp::Div => lhs.0 / rhs.0,
								_ => panic!(),
							}))),
							(lhs, rhs) => {
								let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
								let val = match (&lhs, &rhs) {
									(Val::Num(_), _) => rhs,
									(_, Val::Num(_)) => lhs,
									_ => lhs,
								};
								let type_name = self.types.name(self.syms, val.type_id());
								Err(Signal::Error(
									Error::TypeError(TypeError::ArithNonNum(type_name)),
									vec![(String::new(), loc)],
								))
							}
						},
						BinaryOp::Or | BinaryOp::And => panic!(),
					}
				}
			},
			Expr::Unary(unary) => match &unary.op {
				UnaryOp::Not => {
					let val = self.eval_expr(chunk, unary.val)?;
					Ok(Val::Bool(Bool(!val.is_truthy())))
				}
				UnaryOp::Neg => match self.eval_expr(chunk, unary.val)? {
					Val::Num(num) => Ok(Val::Num(Num(-num.0))),
					val => {
						let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
						let type_name = self.types.name(self.syms, val.type_id());
						Err(Signal::Error(
							Error::TypeError(TypeError::ArithNonNum(type_name)),
							vec![(String::new(), loc)],
						))
					}
				},
			},
			Expr::Self_ => match &self.receiver {
				Some(receiver) => Ok(receiver.clone()),
				None => todo!(),
			},
			Expr::Name(name) => {
				let val = self.eval_name_raw(chunk, expr_id, name.sym)?;
				self.invoke_or_return(chunk, expr_id, val)
			}
			Expr::Builtin(builtin) => match builtin {
				Builtin::Print { val: val_id } => {
					let val = self.eval_expr(chunk, *val_id)?;
					println!("{}", rt_print_val(self.syms, &self.types, &val));
					Ok(val)
				}
			},
			Expr::Lit(lit) => Ok(match lit {
				Lit::Str(str) => Val::Str(Rc::new(Str {
					text: Box::from(str.as_str()),
					size: Cell::new(None),
				})),
				Lit::Char(char) => Val::Char(Char(*char)),
				Lit::Num(num) => Val::Num(Num(*num)),
				Lit::Bool(bool) => Val::Bool(Bool(*bool)),
				Lit::List(item_ids) => {
					let mut items = Vec::with_capacity(item_ids.len());
					for item_id in item_ids {
						let item = self.eval_expr(chunk, *item_id)?;
						items.push(item);
					}
					Val::Obj(Rc::new(RefCell::new(Obj::List(List { items }))))
				}
				Lit::Dict(pair_ids) => {
					let mut pairs = OrderMap::with_capacity(pair_ids.len());
					for (key_id, val_id) in pair_ids {
						let key = self.eval_expr(chunk, *key_id)?;
						let val = self.eval_expr(chunk, *val_id)?;
						pairs.insert(key, val);
					}
					Val::Obj(Rc::new(RefCell::new(Obj::Dict(Dict { pairs }))))
				}
				Lit::Nil => Val::Nil(Nil),
			}),
		}
	}

	fn eval_expr_raw(&mut self, chunk: &Chunk, expr_id: ExprId) -> Result<Val, Signal> {
		match chunk.get_expr(expr_id) {
			Expr::Name(name) => self.eval_name_raw(chunk, expr_id, name.sym),
			Expr::Member(member) => {
				self.eval_member_raw(chunk, expr_id, member.receiver, member.name)
			}
			_ => self.eval_expr(chunk, expr_id),
		}
	}

	// Resolve a name to a place, following the canonical lookup chain:
	//
	//   locals > self's members > module's bindings > prelude
	//
	// The latter two cases are handled by checking the local's tier.
	fn resolve_name(&self, name: Sym) -> Place {
		let local = Scope::local(&self.scope, name);
		if local.is_bound() && local.tier == Tier::Local {
			return Place::Local(local);
		}
		if let Some(receiver) = &self.receiver
			&& let Some(member) = self.types.member(receiver, name)
		{
			return Place::Member(member);
		}
		Place::Local(local)
	}

	fn eval_name_raw(&mut self, chunk: &Chunk, expr_id: ExprId, name: Sym) -> Result<Val, Signal> {
		let place = self.resolve_name(name);
		if !place.is_bound() {
			let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
			let name = self.syms.resolve(name);
			return Err(Signal::Error(
				Error::NameError(name.to_string()),
				vec![(String::new(), loc)],
			));
		}
		match place.get(&self.types) {
			Ok(val) => Ok(val),
			Err(err) => {
				let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
				Err(Signal::Error(err, vec![(String::new(), loc)]))
			}
		}
	}

	fn eval_member_raw(
		&mut self,
		chunk: &Chunk,
		expr_id: ExprId,
		val_id: ExprId,
		name: Sym,
	) -> Result<Val, Signal> {
		let val = self.eval_expr(chunk, val_id)?;

		if let Some(member) = self.types.member(&val, name) {
			match Place::Member(member).get(&self.types) {
				Ok(val) => Ok(val),
				Err(err) => {
					let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
					Err(Signal::Error(err, vec![(String::new(), loc)]))
				}
			}
		} else {
			let type_name = self.types.name(self.syms, val.namespace_type_id());
			let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
			Err(Signal::Error(
				Error::MemberError(MemberError::Missing(
					type_name.to_string(),
					self.syms.resolve(name).to_string(),
				)),
				vec![(String::new(), loc)],
			))
		}
	}

	fn eval_index(
		&mut self,
		chunk: &Chunk,
		expr_id: ExprId,
		key_id: ExprId,
		len: usize,
	) -> Result<usize, Signal> {
		let num = match self.eval_expr(chunk, key_id)? {
			Val::Num(num) => num.0,
			val => {
				let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
				return Err(Signal::Error(
					Error::TypeError(TypeError::IndexNonNum(rt_print_val(
						self.syms,
						&self.types,
						&val,
					))),
					vec![(String::new(), loc)],
				));
			}
		};

		if num != num.trunc() {
			let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
			return Err(Signal::Error(
				Error::IndexError(IndexError::NonIntegral(num)),
				vec![(String::new(), loc)],
			));
		}

		if num < 0.0 || num >= len as f64 {
			let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
			return Err(Signal::Error(
				Error::IndexError(IndexError::OutOfRange(num)),
				vec![(String::new(), loc)],
			));
		}

		Ok(num as usize)
	}

	fn expect_invocable(
		&mut self,
		chunk: &Chunk,
		expr_id: ExprId,
		val: Val,
	) -> Result<Val, Signal> {
		if let Val::Obj(rf) = &val {
			match &*rf.borrow() {
				Obj::Proc(_) | Obj::Method(_) => return Ok(val.clone()),
				_ => {}
			}
		}

		let loc = self.pkg.loc(chunk.get_expr_span(expr_id));
		let type_name = self.types.name(self.syms, val.type_id());
		Err(Signal::Error(
			Error::TypeError(TypeError::NotInvokable(type_name)),
			vec![(String::new(), loc)],
		))
	}

	fn invoke_or_return(
		&mut self,
		chunk: &Chunk,
		expr_id: ExprId,
		val: Val,
	) -> Result<Val, Signal> {
		let Val::Obj(rf) = &val else { return Ok(val) };
		let rf = rf.clone();
		let span = chunk.get_expr_span(expr_id);

		// Required params precede defaulted ones, so the first param alone says
		// whether this needs any arguments.
		let missing = match &*rf.borrow() {
			Obj::Proc(proc) => proc
				.params
				.first()
				.filter(|param| param.default.is_none())
				.map(|param| self.syms.resolve(param.name).to_string()),
			Obj::Method(Method::User(_, proc)) => proc
				.borrow()
				.params
				.first()
				.filter(|param| param.default.is_none())
				.map(|param| self.syms.resolve(param.name).to_string()),
			// Native types are defined in Rust, skipping the
			// required-before-default ordering check. Use find instead of
			// filter to ensure we find any required params.
			Obj::Method(Method::Native(_, _, meth)) => meth
				.params
				.iter()
				.find(|param| param.default.is_none())
				.map(|param| param.name.to_string()),
			_ => return Ok(val),
		};

		// Used in an invoking position but requires arguments; error.
		if let Some(name) = missing {
			let loc = self.pkg.loc(span);
			return Err(Signal::Error(
				Error::ArgumentError(ArgumentError::Missing(vec![name])),
				vec![(String::new(), loc)],
			));
		}

		match &*rf.borrow() {
			Obj::Proc(proc) => self.eval_proc_call(span, proc, None, Vec::new()),
			Obj::Method(Method::User(inst, proc)) => self.eval_proc_call(
				span,
				&proc.borrow(),
				Some(Val::Obj(inst.clone())),
				Vec::new(),
			),
			Obj::Method(Method::Native(recv, _name, meth)) => {
				match (meth.call)(recv, meth.defaults().unwrap()) {
					Ok(val) => Ok(val),
					Err(err) => {
						let loc = self.pkg.loc(span);
						Err(Signal::Error(err, vec![(String::new(), loc)]))
					}
				}
			}
			_ => panic!(),
		}
	}

	fn bind_native_args(
		&self,
		span: Span,
		params: &'static [NativeParam],
		args: Vec<(Option<Sym>, Val)>,
	) -> Result<Vec<Val>, Signal> {
		let arg_count = args.len();
		let mut slots = vec![None; params.len()];
		let mut next = 0;
		for (name, val) in args {
			let index = match name {
				Some(name) => {
					let name = self.syms.resolve(name);
					match params.iter().position(|param| param.name == name) {
						Some(index) => index,
						None => {
							let loc = self.pkg.loc(span);
							return Err(Signal::Error(
								Error::ArgumentError(ArgumentError::Unknown(name.to_string())),
								vec![(String::new(), loc)],
							));
						}
					}
				}
				None => {
					if next == params.len() {
						let loc = self.pkg.loc(span);
						return Err(Signal::Error(
							Error::ArgumentError(ArgumentError::TooMany(arg_count, params.len())),
							vec![(String::new(), loc)],
						));
					}
					let index = next;
					next += 1;
					index
				}
			};
			if slots[index].is_some() {
				let loc = self.pkg.loc(span);
				let name = params[index].name.to_string();
				return Err(Signal::Error(
					Error::ArgumentError(ArgumentError::Duplicate(name)),
					vec![(String::new(), loc)],
				));
			}
			slots[index] = Some(val);
		}

		let missing = params
			.iter()
			.zip(&slots)
			.filter(|(param, slot)| slot.is_none() && param.default.is_none())
			.map(|(param, _)| param.name.to_string())
			.collect::<Vec<_>>();
		if !missing.is_empty() {
			let loc = self.pkg.loc(span);
			return Err(Signal::Error(
				Error::ArgumentError(ArgumentError::Missing(missing)),
				vec![(String::new(), loc)],
			));
		}

		Ok(params
			.iter()
			.zip(slots)
			.map(|(param, slot)| match slot {
				Some(val) => val,
				None => (param.default.unwrap())(),
			})
			.collect())
	}

	// Every failure here is the caller's, so the errors carry the call site and
	// no callee frame.
	fn slot_args(
		&self,
		span: Span,
		params: &[Param],
		args: Vec<(Option<Sym>, Val)>,
	) -> Result<Vec<Option<Val>>, Signal> {
		let arg_count = args.len();
		let mut slots = vec![None; params.len()];
		let mut next = 0;
		for (name, val) in args {
			let index = match name {
				Some(name) => match params.iter().position(|param| param.name == name) {
					Some(index) => index,
					None => {
						let loc = self.pkg.loc(span);
						let name = self.syms.resolve(name).to_string();
						return Err(Signal::Error(
							Error::ArgumentError(ArgumentError::Unknown(name)),
							vec![(String::new(), loc)],
						));
					}
				},
				None => {
					if next == params.len() {
						let loc = self.pkg.loc(span);
						return Err(Signal::Error(
							Error::ArgumentError(ArgumentError::TooMany(arg_count, params.len())),
							vec![(String::new(), loc)],
						));
					}
					let index = next;
					next += 1;
					index
				}
			};
			if slots[index].is_some() {
				let loc = self.pkg.loc(span);
				let name = self.syms.resolve(params[index].name).to_string();
				return Err(Signal::Error(
					Error::ArgumentError(ArgumentError::Duplicate(name)),
					vec![(String::new(), loc)],
				));
			}
			slots[index] = Some(val);
		}

		// Every unfilled parameter is found here, before bind_args runs any
		// default, so a call that cannot succeed evaluates none of them.
		let missing = params
			.iter()
			.zip(&slots)
			.filter(|(param, slot)| slot.is_none() && param.default.is_none())
			.map(|(param, _)| self.syms.resolve(param.name).to_string())
			.collect::<Vec<_>>();
		if !missing.is_empty() {
			let loc = self.pkg.loc(span);
			return Err(Signal::Error(
				Error::ArgumentError(ArgumentError::Missing(missing)),
				vec![(String::new(), loc)],
			));
		}
		Ok(slots)
	}

	// Fills the frame in declaration order, so a default sees every parameter
	// to its left. A default that raises does so inside the callee.
	fn bind_args(
		&mut self,
		chunk_id: ChunkId,
		params: &[Param],
		slots: Vec<Option<Val>>,
		scope: &Rc<RefCell<Scope>>,
	) -> Result<(), Signal> {
		let saved_scope = self.scope.clone();
		self.scope = scope.clone();
		for (param, slot) in params.iter().zip(slots) {
			let val = match slot {
				Some(val) => val,
				None => {
					let chunk = self.pkg.get_chunk(chunk_id);
					match self.eval_expr(chunk, param.default.unwrap()) {
						Ok(val) => val,
						Err(signal) => {
							self.scope = saved_scope;
							return Err(signal);
						}
					}
				}
			};
			scope.borrow_mut().locals.insert(param.name, val);
		}
		self.scope = saved_scope;
		Ok(())
	}

	fn eval_proc_call(
		&mut self,
		span: Span,
		proc: &Proc,
		receiver: Option<Val>,
		args: Vec<(Option<Sym>, Val)>,
	) -> Result<Val, Signal> {
		let slots = self.slot_args(span, &proc.params, args)?;
		let scope = Rc::new(RefCell::new(Scope {
			locals: FxHashMap::default(),
			outer: Some(proc.scope.clone()),
			tier: Tier::Local,
		}));
		let saved_receiver = self.receiver.clone();
		self.receiver = receiver;
		let body_chunk = self.pkg.get_chunk(proc.chunk);
		let result = match self.bind_args(proc.chunk, &proc.params, slots, &scope) {
			Ok(()) => self.eval_block(body_chunk, scope, proc.body),
			Err(signal) => Err(signal),
		};
		self.receiver = saved_receiver;
		match result {
			Ok(val) => Ok(val),
			Err(Signal::Return(val)) => Ok(val),
			Err(Signal::Break(_)) => panic!(),
			Err(Signal::Error(err, mut trace)) => {
				let proc_name = self.syms.resolve(proc.name);
				trace.last_mut().unwrap().0.push_str(proc_name);

				let loc = self.pkg.loc(span);
				trace.push((String::new(), loc));
				Err(Signal::Error(err, trace))
			}
		}
	}

	fn eval_block(
		&mut self,
		chunk: &Chunk,
		scope: Rc<RefCell<Scope>>,
		block_id: BlockId,
	) -> Result<Val, Signal> {
		let block = chunk.get_block(block_id);
		let saved = self.scope.clone();
		self.scope = scope;
		let mut res = Val::Nil(Nil);
		for expr_id in &block.exprs {
			match self.eval_expr(chunk, *expr_id) {
				Ok(val) => {
					res = val;
				}
				Err(Signal::Return(val)) => {
					self.scope = saved;
					return Err(Signal::Return(val));
				}
				Err(Signal::Break(val)) => {
					self.scope = saved;
					return Err(Signal::Break(val));
				}
				Err(Signal::Error(err, loc)) => {
					self.scope = saved;
					return Err(Signal::Error(err, loc));
				}
			}
		}
		self.scope = saved;
		Ok(res)
	}
}

fn rt_print_proc(syms: &Interner, proc: &Proc) -> String {
	let mut res = String::new();
	let name = syms.resolve(proc.name);
	res.push_str(&format!("def {}", name));
	if !proc.params.is_empty() {
		res.push('(');
	}
	for (i, param) in proc.params.iter().enumerate() {
		let param = syms.resolve(param.name);
		res.push_str(param);
		if i + 1 != proc.params.len() {
			res.push_str(", ");
		}
	}
	if !proc.params.is_empty() {
		res.push(')');
	}
	res
}

fn rt_print_val(syms: &Interner, types: &Types, val: &Val) -> String {
	match val {
		Val::Num(num) => format!("{}", num.0),
		Val::Bool(bool) => format!("{}", bool.0),
		Val::Char(char) => format!("{}", char.0),
		Val::Str(str) => format!("{}", str.text),
		Val::Obj(rf) => rt_print_obj(syms, types, &rf.borrow()),
		Val::Nil(_) => String::from("nil"),
	}
}

fn rt_print_obj(syms: &Interner, types: &Types, obj: &Obj) -> String {
	match obj {
		Obj::List(list) => {
			let mut res = String::new();
			res.push('[');
			for (i, item) in list.items.iter().enumerate() {
				res.push_str(&rt_debug_val(syms, types, item));
				if i + 1 != list.items.len() {
					res.push_str(", ");
				}
			}
			res.push(']');
			res
		}
		Obj::Dict(dict) => {
			let mut res = String::new();
			res.push('{');
			for (i, (key, val)) in dict.pairs.iter().enumerate() {
				res.push_str(&rt_debug_val(syms, types, key));
				res.push_str(": ");
				res.push_str(&rt_debug_val(syms, types, val));
				if i + 1 != dict.pairs.len() {
					res.push_str(", ");
				}
			}
			res.push('}');
			res
		}
		Obj::Proc(proc) => rt_print_proc(syms, proc),
		Obj::Type(type_id) => match type_id {
			TypeId::User(type_id) => {
				let desc = types.descs.get_type(*type_id);
				let name = types.name(syms, TypeId::User(*type_id));
				let mut res = String::new();
				res.push_str(&format!("type {}(", name));
				for (i, field) in desc.ctor_fields.iter().enumerate() {
					let field = syms.resolve(field.name);
					res.push_str(field);
					if i + 1 != desc.ctor_fields.len() {
						res.push_str(", ");
					}
				}
				res.push(')');
				res
			}
			TypeId::Native(type_id) => {
				let typ = types.native(*type_id);
				let name = syms.resolve(typ.name);
				format!("type {}", name)
			}
		},
		Obj::Proto(proto_id) => {
			let proto = types.descs.get_proto(*proto_id);
			format!("proto {}", syms.resolve(proto.name))
		}
		Obj::Instance(inst) => {
			let desc = types.descs.get_type(inst.typ);
			let name = types.name(syms, TypeId::User(inst.typ));
			let mut res = String::new();
			res.push_str(&format!("{}(", name));
			for (i, field) in desc.ctor_fields.iter().enumerate() {
				let val = inst.fields.get(&field.name).unwrap();
				res.push_str(&rt_print_val(syms, types, val));
				if i + 1 != desc.ctor_fields.len() {
					res.push_str(", ");
				}
			}
			res.push(')');
			res
		}
		Obj::Method(meth) => match meth {
			Method::User(_, proc) => rt_print_proc(syms, &proc.borrow()),
			Method::Native(_, name, _) => {
				format!("def {}", syms.resolve(*name))
			}
		},
	}
}

fn rt_debug_val(syms: &Interner, types: &Types, val: &Val) -> String {
	match val {
		Val::Char(char) => format!("'{}'", char.0),
		Val::Str(str) => format!("\"{}\"", str.text),
		Val::Obj(rf) => rt_print_obj(syms, types, &rf.borrow()),
		val => rt_print_val(syms, types, val),
	}
}
