use std::cell::{Cell, RefCell};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use ordermap::OrderMap;
use rustc_hash::FxHashMap;

use crate::intern::{Interner, Sym};
use crate::pkg::PackageId;
use crate::rt::Runtime;
use crate::rt::eval::{Interpreter, Raise};
use crate::rt::print;
use crate::rt::scope::Scope;
use crate::rt::types::{NativeMethod, NativeTypeId, TypeId};
use crate::sem::modules::ModuleId;
use crate::sem::protos::ProtoId;
use crate::sem::types;
use crate::syn::ChunkId;
use crate::syn::nodes::{BlockId, Param};

#[derive(Debug, Clone, Copy)]
pub struct Num(pub f64);

impl Num {
	pub fn inspect(_interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Num(num) = val else { panic!() };
		Ok(Str::of(&print::print_num(num.0)))
	}

	pub fn new() -> Val {
		Val::Num(Num(0.0))
	}

	pub fn order(_interp: &mut Interpreter, val: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Num(Num(this)) = val else { panic!() };
		let Val::Num(Num(other)) = &args[0] else {
			panic!()
		};
		Ok(Val::Num(Num(this - other)))
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Bool(pub bool);

impl Bool {
	pub fn inspect(_interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Bool(bool) = val else { panic!() };
		Ok(Str::of(&print::print_bool(bool.0)))
	}

	pub fn new() -> Val {
		Val::Bool(Bool(false))
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Char(pub char);

impl Char {
	pub fn inspect(_interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Char(char) = val else { panic!() };
		Ok(Str::of(&print::print_char(char.0)))
	}

	pub fn display(_interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Char(char) = val else { panic!() };
		Ok(Str::of(&char.0.to_string()))
	}

	pub fn order(_interp: &mut Interpreter, val: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Char(Char(this)) = val else { panic!() };
		let Val::Char(Char(other)) = &args[0] else {
			panic!()
		};
		Ok(Val::Num(Num(*this as u32 as f64 - *other as u32 as f64)))
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

	pub fn inspect(_interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Str(str) = val else { panic!() };
		Ok(Str::of(&print::print_str(&str.text)))
	}

	pub fn display(_interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Str(str) = val else { panic!() };
		Ok(Str::of(&str.text))
	}

	pub fn size(_interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
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

	pub fn empty(_interp: &mut Interpreter, _val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		Ok(Str::new())
	}

	pub fn chars(_interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Str(str) = val else { panic!() };
		let items = str.text.chars().map(|c| Val::Char(Char(c))).collect();
		Ok(Val::Obj(Rc::new(RefCell::new(Obj::List(List { items })))))
	}

	pub fn order(_interp: &mut Interpreter, val: &Val, args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Str(this) = val else { panic!() };
		let Val::Str(other) = &args[0] else { panic!() };
		let order = match this.text.cmp(&other.text) {
			std::cmp::Ordering::Less => -1.0,
			std::cmp::Ordering::Equal => 0.0,
			std::cmp::Ordering::Greater => 1.0,
		};
		Ok(Val::Num(Num(order)))
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Nil;

impl Nil {
	pub fn inspect(_interp: &mut Interpreter, _val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		Ok(Str::of(&print::print_nil()))
	}

	pub fn new() -> Val {
		Val::Nil(Nil)
	}
}

#[derive(Debug, Clone)]
pub enum Val {
	Num(Num),
	Bool(Bool),
	Char(Char),
	Str(Rc<Str>),
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

impl PartialEq for Val {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Num(num), Self::Num(other_num)) => num.0 == other_num.0,
			(Self::Bool(bool), Self::Bool(other_bool)) => bool.0 == other_bool.0,
			(Self::Char(char), Self::Char(other_char)) => char.0 == other_char.0,
			(Self::Str(str), Self::Str(other_str)) => str.text == other_str.text,
			(Self::Obj(obj), Self::Obj(other)) => obj.as_ptr() == other.as_ptr(),
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
			Self::Obj(obj) => obj.as_ptr().hash(state),
			Self::Nil(_) => 0_u8.hash(state),
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

#[derive(Debug)]
pub struct List {
	pub items: Vec<Val>,
}

impl List {
	pub fn new() -> Val {
		Val::Obj(Rc::new(RefCell::new(Obj::List(List { items: Vec::new() }))))
	}

	pub fn size(_interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Obj(obj) = val else { panic!() };
		let Obj::List(list) = &*obj.borrow() else {
			panic!()
		};
		Ok(Val::Num(Num(list.items.len() as f64)))
	}

	pub fn inspect(interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Obj(obj) = val else { panic!() };
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
	pub pairs: OrderMap<Val, Val>,
}

impl Dict {
	pub fn new() -> Val {
		Val::Obj(Rc::new(RefCell::new(Obj::Dict(Dict {
			pairs: OrderMap::new(),
		}))))
	}

	pub fn size(_interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Obj(obj) = val else { panic!() };
		let Obj::Dict(dict) = &*obj.borrow() else {
			panic!()
		};
		Ok(Val::Num(Num(dict.pairs.len() as f64)))
	}

	pub fn inspect(interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
		let Val::Obj(obj) = val else { panic!() };
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

pub fn derived_inspect(interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
	let instance = match val {
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
			val,
		)));
	};
	let mut printed = Vec::new();
	for (field, val) in &fields {
		printed.push((field.clone(), interp.inspect(val)?));
	}
	Ok(Str::of(&print::print_instance(&name, &printed)))
}

pub fn proc_inspect(interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
	let Val::Obj(obj) = val else { panic!() };
	let text = match &*obj.borrow() {
		Obj::Proc(proc) => print::print_proc(interp.syms(), proc),
		Obj::Method(method) => print::print_method(interp.syms(), method),
		_ => panic!(),
	};
	Ok(Str::of(&text))
}

pub fn type_inspect(interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
	let Val::Obj(obj) = val else { panic!() };
	let Obj::Type(type_id) = &*obj.borrow() else {
		panic!()
	};
	Ok(Str::of(&print::print_type(
		interp.syms(),
		interp.rt(),
		*type_id,
	)))
}

pub fn proto_inspect(interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
	let Val::Obj(obj) = val else { panic!() };
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

pub fn module_inspect(interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
	let Val::Obj(obj) = val else { panic!() };
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
