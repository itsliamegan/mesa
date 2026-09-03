use std::cell::{Cell, RefCell};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHasher};

use crate::intern::{Interner, Sym};
use crate::pkg::PackageId;
use crate::rt::Runtime;
use crate::rt::eval::{Interpreter, Raise};
use crate::rt::print;
use crate::rt::scope::Scope;
use crate::rt::types::{NativeMethod, NativeTypeId, TypeId};
use crate::rt::{Error, ProtocolError};
use crate::sem::modules::ModuleId;
use crate::sem::protos::ProtoId;
use crate::sem::types;
use crate::syn::ChunkId;
use crate::syn::nodes::{BlockId, Param};

#[derive(Debug, Clone, Copy)]
pub struct Num(pub f64);

impl Num {
	pub fn new() -> Val {
		Val::Num(Num(0.0))
	}

	pub fn order(_interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Num(Num(self_)) = self_ else {
			panic!()
		};
		let Val::Num(Num(other)) = &args[0] else {
			panic!()
		};
		Ok(Val::Num(Num(self_ - other)))
	}

	pub fn hash(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Num(num) = self_ else { panic!() };
		let canonical = if num.0 == 0.0 { 0.0 } else { num.0 };
		Ok(Val::Digest(
			hash_type_id(self_.type_id()).mix(Digest(canonical.to_bits())),
		))
	}

	pub fn equal(_interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Num(num) = self_ else { panic!() };
		let Val::Num(other) = &args[0] else {
			return Ok(Val::Bool(Bool(false)));
		};
		Ok(Val::Bool(Bool(num.0 == other.0)))
	}

	pub fn inspect(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Num(num) = self_ else { panic!() };
		Ok(Str::of(&print::print_num(num.0)))
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Bool(pub bool);

impl Bool {
	pub fn new() -> Val {
		Val::Bool(Bool(false))
	}

	pub fn hash(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Bool(bool) = self_ else { panic!() };
		Ok(Val::Digest(
			hash_type_id(self_.type_id()).mix(Digest(u64::from(bool.0))),
		))
	}

	pub fn equal(_interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Bool(bool) = self_ else { panic!() };
		let Val::Bool(other) = &args[0] else {
			return Ok(Val::Bool(Bool(false)));
		};
		Ok(Val::Bool(Bool(bool.0 == other.0)))
	}

	pub fn inspect(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Bool(self_) = self_ else { panic!() };
		Ok(Str::of(&print::print_bool(self_.0)))
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Char(pub char);

impl Char {
	pub fn equal(_interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Char(char) = self_ else { panic!() };
		let Val::Char(other) = &args[0] else {
			return Ok(Val::Bool(Bool(false)));
		};
		Ok(Val::Bool(Bool(char.0 == other.0)))
	}

	pub fn order(_interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Char(Char(self_)) = self_ else {
			panic!()
		};
		let Val::Char(Char(other)) = &args[0] else {
			panic!()
		};
		Ok(Val::Num(Num(*self_ as u32 as f64 - *other as u32 as f64)))
	}

	pub fn hash(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Char(char) = self_ else { panic!() };
		Ok(Val::Digest(
			hash_type_id(self_.type_id()).mix(Digest(char.0 as u64)),
		))
	}

	pub fn display(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Char(char) = self_ else { panic!() };
		Ok(Str::of(&char.0.to_string()))
	}

	pub fn inspect(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Char(char) = self_ else { panic!() };
		Ok(Str::of(&print::print_char(char.0)))
	}
}

#[derive(Debug)]
pub struct Str {
	pub text: Box<str>,
	pub size: Cell<Option<f64>>,
}

impl Str {
	pub fn of(text: &str) -> Val {
		Val::Str(Rc::new(Str {
			text: Box::from(text),
			size: Cell::new(None),
		}))
	}

	pub fn new() -> Val {
		Val::Str(Rc::new(Str {
			text: Box::from(""),
			size: Cell::new(None),
		}))
	}

	pub fn size(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Str(str) = self_ else { panic!() };
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

	pub fn empty(_interp: &mut Interpreter, _self: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		Ok(Str::new())
	}

	pub fn chars(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Str(str) = self_ else { panic!() };
		let items = str.text.chars().map(|c| Val::Char(Char(c))).collect();
		Ok(Val::Obj(Rc::new(RefCell::new(Obj::List(List { items })))))
	}

	pub fn order(_interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Str(self_) = self_ else { panic!() };
		let Val::Str(other) = &args[0] else { panic!() };
		let order = match self_.text.cmp(&other.text) {
			std::cmp::Ordering::Less => -1.0,
			std::cmp::Ordering::Equal => 0.0,
			std::cmp::Ordering::Greater => 1.0,
		};
		Ok(Val::Num(Num(order)))
	}

	pub fn hash(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Str(str) = self_ else { panic!() };
		let mut hasher = FxHasher::default();
		str.text.hash(&mut hasher);
		Ok(Val::Digest(
			hash_type_id(self_.type_id()).mix(Digest(hasher.finish())),
		))
	}

	pub fn equal(_interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Str(self_) = self_ else { panic!() };
		let Val::Str(other) = &args[0] else {
			return Ok(Val::Bool(Bool(false)));
		};
		Ok(Val::Bool(Bool(self_.text == other.text)))
	}

	pub fn display(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Str(self_) = self_ else { panic!() };
		Ok(Str::of(&self_.text))
	}

	pub fn inspect(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Str(self_) = self_ else { panic!() };
		Ok(Str::of(&print::print_str(&self_.text)))
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Nil;

impl Nil {
	pub fn new() -> Val {
		Val::Nil(Nil)
	}

	pub fn hash(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		Ok(Val::Digest(hash_type_id(self_.type_id())))
	}

	pub fn equal(_interp: &mut Interpreter, _self: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Nil(_) = &args[0] else {
			return Ok(Val::Bool(Bool(false)));
		};
		Ok(Val::Bool(Bool(true)))
	}

	pub fn inspect(_interp: &mut Interpreter, _self: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		Ok(Str::of(&print::print_nil()))
	}
}

#[derive(Debug, Clone)]
pub enum Val {
	Num(Num),
	Bool(Bool),
	Char(Char),
	Str(Rc<Str>),
	Digest(Digest),
	Obj(Rc<RefCell<Obj>>),
	Nil(Nil),
}

impl Val {
	pub fn type_id(&self) -> TypeId {
		match self {
			Val::Num(_) => TypeId::Native(NativeTypeId::NUM),
			Val::Bool(_) => TypeId::Native(NativeTypeId::BOOL),
			Val::Char(_) => TypeId::Native(NativeTypeId::CHAR),
			Val::Str(_) => TypeId::Native(NativeTypeId::STR),
			Val::Digest(_) => TypeId::Native(NativeTypeId::DIGEST),
			Val::Obj(obj) => obj.borrow().type_id(),
			Self::Nil(_) => TypeId::Native(NativeTypeId::NIL),
		}
	}

	pub fn namespace_type_id(&self) -> TypeId {
		match self {
			Val::Obj(obj) => match &*obj.borrow() {
				Obj::Type(id) => *id,
				obj => obj.type_id(),
			},
			val => val.type_id(),
		}
	}

	pub fn is_truthy(&self) -> bool {
		match self {
			Val::Bool(bool) => bool.0,
			Val::Nil(_) => false,
			_ => true,
		}
	}
}

#[derive(Debug)]
pub enum Obj {
	Module(PackageId, ModuleId),
	List(List),
	Dict(Dict),
	Proc(Proc),
	Type(TypeId),
	Proto(PackageId, ProtoId),
	Instance(Instance),
	Method(Method),
	Native(NativeTypeId, NativeData),
}

pub struct NativeData(pub Box<dyn std::any::Any>);

impl Debug for NativeData {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		f.write_str("<native>")
	}
}

impl Obj {
	pub fn type_id(&self) -> TypeId {
		match self {
			Self::Module(_, _) => TypeId::Native(NativeTypeId::MODULE),
			Self::List(_) => TypeId::Native(NativeTypeId::LIST),
			Self::Dict(_) => TypeId::Native(NativeTypeId::DICT),
			Self::Proc(_) => TypeId::Native(NativeTypeId::PROC),
			Self::Type(_) => TypeId::Native(NativeTypeId::TYPE),
			Self::Proto(_, _) => TypeId::Native(NativeTypeId::PROTO),
			Self::Instance(instance) => TypeId::User(instance.pkg, instance.type_),
			Self::Method(_) => TypeId::Native(NativeTypeId::PROC),
			Self::Native(id, _) => TypeId::Native(*id),
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Digest(pub u64);

impl Digest {
	pub(crate) fn mix(self, other: Digest) -> Digest {
		let mut mixed = self.0.rotate_left(27) ^ other.0.wrapping_mul(0xff51afd7ed558ccd);
		mixed = mixed.wrapping_mul(0x9e3779b97f4a7c15);
		Digest(mixed ^ (mixed >> 31))
	}

	pub fn hash(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Digest(digest) = self_ else { panic!() };
		Ok(Val::Digest(*digest))
	}

	pub fn equal(_interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Digest(digest) = self_ else { panic!() };
		let Val::Digest(other) = &args[0] else {
			return Ok(Val::Bool(Bool(false)));
		};
		Ok(Val::Bool(Bool(digest.0 == other.0)))
	}

	pub fn inspect(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Digest(digest) = self_ else { panic!() };
		Ok(Str::of(&print::print_digest(digest.0)))
	}
}

#[derive(Debug)]
pub struct List {
	pub items: Vec<Val>,
}

impl List {
	pub fn new() -> Val {
		Val::Obj(Rc::new(RefCell::new(Obj::List(List { items: Vec::new() }))))
	}

	pub fn equal(interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Obj(obj) = self_ else { panic!() };
		let Val::Obj(other) = &args[0] else {
			return Ok(Val::Bool(Bool(false)));
		};
		if Rc::ptr_eq(obj, other) {
			return Ok(Val::Bool(Bool(true)));
		}
		let items = {
			let obj_ref = obj.borrow();
			let other_ref = other.borrow();
			let Obj::List(list) = &*obj_ref else { panic!() };
			let Obj::List(other_list) = &*other_ref else {
				return Ok(Val::Bool(Bool(false)));
			};
			if list.items.len() != other_list.items.len() {
				return Ok(Val::Bool(Bool(false)));
			}
			list.items
				.iter()
				.cloned()
				.zip(other_list.items.iter().cloned())
				.collect::<Vec<_>>()
		};
		for (item, other_item) in items {
			if !interp.equal_vals(&item, &other_item)? {
				return Ok(Val::Bool(Bool(false)));
			}
		}
		Ok(Val::Bool(Bool(true)))
	}

	pub fn size(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Obj(obj) = self_ else { panic!() };
		let Obj::List(list) = &*obj.borrow() else {
			panic!()
		};
		Ok(Val::Num(Num(list.items.len() as f64)))
	}

	pub fn append(_interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Obj(obj) = self_ else { panic!() };
		let Obj::List(list) = &mut *obj.borrow_mut() else {
			panic!()
		};
		list.items.push(args[0].clone());
		Ok(self_.clone())
	}

	pub fn access(interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Obj(obj) = self_ else { panic!() };
		let len = {
			let Obj::List(list) = &*obj.borrow() else {
				panic!()
			};
			list.items.len()
		};
		let index = interp
			.index_of(&args[0], len)
			.map_err(|err| interp.native_error(err))?;
		let Obj::List(list) = &*obj.borrow() else {
			panic!()
		};
		Ok(list.items[index].clone())
	}

	pub fn store(interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Obj(obj) = self_ else { panic!() };
		let len = {
			let Obj::List(list) = &*obj.borrow() else {
				panic!()
			};
			list.items.len()
		};
		let index = interp
			.index_of(&args[0], len)
			.map_err(|err| interp.native_error(err))?;
		let Obj::List(list) = &mut *obj.borrow_mut() else {
			panic!()
		};
		list.items[index] = args[1].clone();
		Ok(args[1].clone())
	}

	pub fn inspect(interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Obj(obj) = self_ else { panic!() };
		let items = {
			let Obj::List(list) = &*obj.borrow() else {
				panic!()
			};
			list.items.clone()
		};
		let mut printed = Vec::new();
		for item in &items {
			printed.push(interp.inspect(item)?);
		}
		Ok(Str::of(&print::print_list(&printed)))
	}
}

#[derive(Debug)]
pub struct Dict {
	pub pairs: Vec<(Val, Val)>,
	index: FxHashMap<u64, Vec<usize>>,
}

impl Dict {
	pub fn new() -> Val {
		Val::Obj(Rc::new(RefCell::new(Obj::Dict(Dict {
			pairs: Vec::new(),
			index: FxHashMap::default(),
		}))))
	}

	pub(crate) fn with_capacity(capacity: usize) -> Self {
		Self {
			pairs: Vec::with_capacity(capacity),
			index: FxHashMap::default(),
		}
	}

	pub(crate) fn candidate(&self, hash: u64, position: usize) -> Option<(usize, Val)> {
		let index = *self.index.get(&hash)?.get(position)?;
		Some((index, self.pairs[index].0.clone()))
	}

	pub(crate) fn insert(&mut self, hash: u64, key: Val, val: Val) {
		let index = self.pairs.len();
		self.pairs.push((key, val));
		self.index.entry(hash).or_default().push(index);
	}

	pub fn equal(interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Obj(obj) = self_ else { panic!() };
		let Val::Obj(other) = &args[0] else {
			return Ok(Val::Bool(Bool(false)));
		};
		if Rc::ptr_eq(obj, other) {
			return Ok(Val::Bool(Bool(true)));
		}
		let pairs = {
			let obj_ref = obj.borrow();
			let other_ref = other.borrow();
			let Obj::Dict(dict) = &*obj_ref else { panic!() };
			let Obj::Dict(other_dict) = &*other_ref else {
				return Ok(Val::Bool(Bool(false)));
			};
			if dict.pairs.len() != other_dict.pairs.len() {
				return Ok(Val::Bool(Bool(false)));
			}
			dict.pairs.clone()
		};
		for (key, val) in pairs {
			let (_, found) = interp.dict_probe_val(&args[0], &key)?;
			let Some(index) = found else {
				return Ok(Val::Bool(Bool(false)));
			};
			let other_val = {
				let Obj::Dict(other_dict) = &*other.borrow() else {
					panic!()
				};
				other_dict.pairs[index].1.clone()
			};
			if !interp.equal_vals(&val, &other_val)? {
				return Ok(Val::Bool(Bool(false)));
			}
		}
		Ok(Val::Bool(Bool(true)))
	}

	pub fn size(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Obj(obj) = self_ else { panic!() };
		let Obj::Dict(dict) = &*obj.borrow() else {
			panic!()
		};
		Ok(Val::Num(Num(dict.pairs.len() as f64)))
	}

	pub fn access(interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let (_, found) = interp.dict_probe_val(self_, &args[0])?;
		if let Some(index) = found {
			let Val::Obj(obj) = self_ else { panic!() };
			let Obj::Dict(dict) = &*obj.borrow() else {
				panic!()
			};
			return Ok(dict.pairs[index].1.clone());
		}
		Err(interp.native_error(Error::KeyError(print::inspect_val(
			interp.syms(),
			interp.rt(),
			&args[0],
		))))
	}

	pub fn store(interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let (hash, found) = interp.dict_probe_val(self_, &args[0])?;
		let Val::Obj(obj) = self_ else { panic!() };
		let Obj::Dict(dict) = &mut *obj.borrow_mut() else {
			panic!()
		};
		match found {
			Some(index) => dict.pairs[index].1 = args[1].clone(),
			None => dict.insert(hash, args[0].clone(), args[1].clone()),
		}
		Ok(args[1].clone())
	}

	pub fn inspect(interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Obj(obj) = self_ else { panic!() };
		let pairs = {
			let Obj::Dict(dict) = &*obj.borrow() else {
				panic!()
			};
			dict.pairs
				.iter()
				.map(|(key, val)| (key.clone(), val.clone()))
				.collect::<Vec<_>>()
		};
		let mut printed = Vec::new();
		for (key, val) in &pairs {
			printed.push((interp.inspect(key)?, interp.inspect(val)?));
		}
		Ok(Str::of(&print::print_dict(&printed)))
	}
}

#[derive(Debug)]
pub struct Proc {
	pub name: Sym,
	pub params: Vec<Param>,
	pub body: BlockId,
	pub pkg: PackageId,
	pub chunk: ChunkId,
	pub scope: Rc<RefCell<Scope>>,
}

#[derive(Debug)]
pub struct Instance {
	pub pkg: PackageId,
	pub type_: types::TypeId,
	pub fields: FxHashMap<Sym, Val>,
}

#[derive(Debug)]
pub enum Member {
	Module(PackageId, ModuleId, Sym),
	Static(Rc<RefCell<Obj>>, Sym),
	User(Rc<RefCell<Obj>>, Sym),
	Native(Val, Sym),
}

#[derive(Debug)]
pub enum Method {
	User(Val, Rc<RefCell<Proc>>),
	Native(Val, Sym, NativeMethod),
}

// Name the namespace a member lookup was made in. For a module, the name of the
// module; otherwise, the name of the type.
pub fn namespace_name(syms: &Interner, rt: &Runtime, val: &Val) -> String {
	if let Val::Obj(obj) = val
		&& let Obj::Module(pkg, id) = &*obj.borrow()
	{
		let pkg_modules = &rt.pkgs.get(*pkg).modules;
		return format!("module {}", pkg_modules.name(syms, *id));
	}
	format!("type {}", rt.type_name(syms, val.namespace_type_id()))
}

fn hash_type_id(type_id: TypeId) -> Digest {
	match type_id {
		TypeId::User(pkg, id) => Digest(1)
			.mix(Digest(pkg.index() as u64))
			.mix(Digest(id.index() as u64)),
		TypeId::Native(id) => Digest(2).mix(Digest(id.index() as u64)),
	}
}

pub fn derived_hash(interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
	let Val::Obj(obj) = self_ else { panic!() };
	let obj_ref = obj.borrow();
	let digest = match &*obj_ref {
		Obj::Instance(instance) => {
			let type_id = TypeId::User(instance.pkg, instance.type_);
			let types::Type::User(desc) = interp
				.rt()
				.pkgs
				.get(instance.pkg)
				.types
				.descs
				.get_type(instance.type_)
			else {
				panic!()
			};
			let fields = desc
				.ctor_fields
				.iter()
				.map(|field| field.name)
				.chain(desc.body_fields.iter().map(|(name, _)| *name))
				.map(|name| instance.fields.get(&name).unwrap().clone())
				.collect::<Vec<_>>();
			drop(obj_ref);
			let mut digest = hash_type_id(type_id);
			for field in fields {
				digest = digest.mix(Digest(interp.hash_val(&field)?));
			}
			digest
		}
		Obj::Native(_, _) => {
			hash_type_id(self_.type_id()).mix(Digest(Rc::as_ptr(obj) as usize as u64))
		}
		_ => panic!(),
	};
	Ok(Val::Digest(digest))
}

pub fn derived_equal(interp: &mut Interpreter, self_: &Val, args: Vec<Val>) -> Result<Val, Raise> {
	if self_.type_id() != args[0].type_id() {
		return Ok(Val::Bool(Bool(false)));
	}
	let Val::Obj(obj) = self_ else { panic!() };
	let Val::Obj(other) = &args[0] else { panic!() };
	if Rc::ptr_eq(obj, other) {
		return Ok(Val::Bool(Bool(true)));
	}
	let obj_ref = obj.borrow();
	let other_ref = other.borrow();
	let values = match (&*obj_ref, &*other_ref) {
		(Obj::Instance(instance), Obj::Instance(other)) => {
			let types::Type::User(desc) = interp
				.rt()
				.pkgs
				.get(instance.pkg)
				.types
				.descs
				.get_type(instance.type_)
			else {
				panic!()
			};
			desc.ctor_fields
				.iter()
				.map(|field| field.name)
				.chain(desc.body_fields.iter().map(|(name, _)| *name))
				.map(|name| {
					(
						instance.fields.get(&name).unwrap().clone(),
						other.fields.get(&name).unwrap().clone(),
					)
				})
				.collect::<Vec<_>>()
		}
		(Obj::Native(_, _), Obj::Native(_, _)) => return Ok(Val::Bool(Bool(false))),
		_ => panic!(),
	};
	drop(other_ref);
	drop(obj_ref);
	for (val, other) in values {
		if !interp.equal_vals(&val, &other)? {
			return Ok(Val::Bool(Bool(false)));
		}
	}
	Ok(Val::Bool(Bool(true)))
}

pub fn derived_inspect(
	interp: &mut Interpreter,
	self_: &Val,
	_args: Vec<Val>,
) -> Result<Val, Raise> {
	let instance = match self_ {
		Val::Obj(obj) => match &*obj.borrow() {
			Obj::Instance(instance) => {
				Some(print::instance_fields(interp.syms(), interp.rt(), instance))
			}
			_ => None,
		},
		_ => None,
	};
	let Some((name, fields)) = instance else {
		return Ok(Str::of(&print::inspect_val(
			interp.syms(),
			interp.rt(),
			self_,
		)));
	};
	let mut printed = Vec::new();
	for (field, val) in &fields {
		printed.push((field.clone(), interp.inspect(val)?));
	}
	Ok(Str::of(&print::print_instance(&name, &printed)))
}

pub fn derived_store(interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
	let member = interp.rt().behaviors.access.store;
	Err(
		interp.native_error(Error::ProtocolError(ProtocolError::NotImplemented(
			self_.type_id(),
			member,
		))),
	)
}

pub fn identity_equal(
	_interp: &mut Interpreter,
	self_: &Val,
	args: Vec<Val>,
) -> Result<Val, Raise> {
	let Val::Obj(obj) = self_ else { panic!() };
	let Val::Obj(other) = &args[0] else {
		return Ok(Val::Bool(Bool(false)));
	};
	Ok(Val::Bool(Bool(Rc::ptr_eq(obj, other))))
}

pub fn proc_hash(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
	let Val::Obj(obj) = self_ else { panic!() };
	Ok(Val::Digest(
		hash_type_id(self_.type_id()).mix(Digest(Rc::as_ptr(obj) as usize as u64)),
	))
}

pub fn proc_inspect(interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
	let Val::Obj(obj) = self_ else { panic!() };
	let text = match &*obj.borrow() {
		Obj::Proc(proc) => print::print_proc(interp.syms(), proc),
		Obj::Method(method) => print::print_method(interp.syms(), method),
		_ => panic!(),
	};
	Ok(Str::of(&text))
}

pub fn type_hash(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
	let Val::Obj(obj) = self_ else { panic!() };
	let Obj::Type(type_id) = &*obj.borrow() else {
		panic!()
	};
	Ok(Val::Digest(
		hash_type_id(self_.type_id()).mix(hash_type_id(*type_id)),
	))
}

pub fn type_inspect(interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
	let Val::Obj(obj) = self_ else { panic!() };
	let Obj::Type(type_id) = &*obj.borrow() else {
		panic!()
	};
	Ok(Str::of(&print::print_type(
		interp.syms(),
		interp.rt(),
		*type_id,
	)))
}

pub fn proto_inspect(interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
	let Val::Obj(obj) = self_ else { panic!() };
	let Obj::Proto(pkg, proto_id) = &*obj.borrow() else {
		panic!()
	};
	Ok(Str::of(&print::print_proto(
		interp.syms(),
		interp.rt(),
		*pkg,
		*proto_id,
	)))
}

pub fn module_hash(_interp: &mut Interpreter, self_: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
	let Val::Obj(obj) = self_ else { panic!() };
	let Obj::Module(pkg, id) = &*obj.borrow() else {
		panic!()
	};
	let mut hasher = FxHasher::default();
	pkg.hash(&mut hasher);
	id.index().hash(&mut hasher);
	Ok(Val::Digest(
		hash_type_id(self_.type_id()).mix(Digest(hasher.finish())),
	))
}

pub fn module_inspect(
	interp: &mut Interpreter,
	self_: &Val,
	_args: Vec<Val>,
) -> Result<Val, Raise> {
	let Val::Obj(obj) = self_ else { panic!() };
	let Obj::Module(pkg, id) = &*obj.borrow() else {
		panic!()
	};
	Ok(Str::of(&print::print_module(
		interp.syms(),
		interp.rt(),
		*pkg,
		*id,
	)))
}
