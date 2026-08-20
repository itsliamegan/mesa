use std::cell::{Cell, RefCell};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use ordermap::OrderMap;
use rustc_hash::FxHashMap;

use crate::intern::{Interner, Sym};
use crate::pkg::PackageId;
use crate::rt::Error;
use crate::rt::pkg::Packages;
use crate::rt::scope::Scope;
use crate::rt::types::{NativeMember, NativeTypeId, TypeId, Types};
use crate::sem::modules::ModuleId;
use crate::sem::types;
use crate::syn::ChunkId;
use crate::syn::nodes::{BlockId, Param};

#[derive(Debug, Clone, Copy)]
pub struct Num(pub f64);

impl Num {
	pub fn new() -> Val {
		Val::Num(Num(0.0))
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Bool(pub bool);

impl Bool {
	pub fn new() -> Val {
		Val::Bool(Bool(false))
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Char(pub char);

#[derive(Debug)]
pub struct Str {
	pub text: Box<str>,
	pub size: Cell<Option<f64>>,
}

impl Str {
	pub fn new() -> Val {
		Val::Str(Rc::new(Str {
			text: Box::from(""),
			size: Cell::new(None),
		}))
	}

	pub fn size(val: &Val, _args: Vec<Val>) -> Result<Val, Error> {
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

	pub fn chars(val: &Val, _args: Vec<Val>) -> Result<Val, Error> {
		let Val::Str(str) = val else { panic!() };
		let items = str.text.chars().map(|c| Val::Char(Char(c))).collect();
		Ok(Val::Obj(Rc::new(RefCell::new(Obj::List(List { items })))))
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Nil;

impl Nil {
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
			Val::Obj(rf) => rf.borrow().type_id(),
			Self::Nil(_) => TypeId::Native(NativeTypeId::NIL),
		}
	}

	pub fn namespace_type_id(&self) -> TypeId {
		match self {
			Val::Obj(rf) => match &*rf.borrow() {
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
pub enum Obj {
	Module(PackageId, ModuleId),
	List(List),
	Dict(Dict),
	Proc(Proc),
	Type(TypeId),
	Proto(PackageId, types::ProtoId),
	Instance(Instance),
	Method(Method),
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
			Self::Instance(inst) => TypeId::User(inst.pkg, inst.typ),
			Self::Method(_) => TypeId::Native(NativeTypeId::PROC),
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

	pub fn size(val: &Val, _args: Vec<Val>) -> Result<Val, Error> {
		let Val::Obj(rf) = val else { panic!() };
		let Obj::List(list) = &*rf.borrow() else {
			panic!()
		};
		Ok(Val::Num(Num(list.items.len() as f64)))
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

	pub fn size(val: &Val, _args: Vec<Val>) -> Result<Val, Error> {
		let Val::Obj(rf) = val else { panic!() };
		let Obj::Dict(dict) = &*rf.borrow() else {
			panic!()
		};
		Ok(Val::Num(Num(dict.pairs.len() as f64)))
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
	pub typ: types::TypeId,
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
	User(Rc<RefCell<Obj>>, Rc<RefCell<Proc>>),
	Native(Val, Sym, NativeMember),
}

pub fn rt_print_proc(syms: &Interner, proc: &Proc) -> String {
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

pub fn rt_print_val(syms: &Interner, types: &Types, pkgs: &Packages, val: &Val) -> String {
	match val {
		Val::Num(num) => format!("{}", num.0),
		Val::Bool(bool) => format!("{}", bool.0),
		Val::Char(char) => format!("{}", char.0),
		Val::Str(str) => format!("{}", str.text),
		Val::Obj(rf) => rt_print_obj(syms, types, pkgs, &rf.borrow()),
		Val::Nil(_) => String::from("nil"),
	}
}

pub fn rt_print_obj(syms: &Interner, types: &Types, pkgs: &Packages, obj: &Obj) -> String {
	match obj {
		Obj::Module(pkg, id) => format!("module {}", pkgs.get(*pkg).mods.name(syms, *id)),
		Obj::List(list) => {
			let mut res = String::new();
			res.push('[');
			for (i, item) in list.items.iter().enumerate() {
				res.push_str(&rt_debug_val(syms, types, pkgs, item));
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
				res.push_str(&rt_debug_val(syms, types, pkgs, key));
				res.push_str(": ");
				res.push_str(&rt_debug_val(syms, types, pkgs, val));
				if i + 1 != dict.pairs.len() {
					res.push_str(", ");
				}
			}
			res.push('}');
			res
		}
		Obj::Proc(proc) => rt_print_proc(syms, proc),
		Obj::Type(type_id) => match type_id {
			TypeId::User(type_pkg, type_id) => {
				let desc = pkgs.get(*type_pkg).types.descs.get_type(*type_id);
				let name = types.name(syms, pkgs, TypeId::User(*type_pkg, *type_id));
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
		Obj::Proto(pkg, proto_id) => {
			let proto = pkgs.get(*pkg).types.descs.get_proto(*proto_id);
			format!("proto {}", syms.resolve(proto.name))
		}
		Obj::Instance(inst) => {
			let desc = pkgs.get(inst.pkg).types.descs.get_type(inst.typ);
			let name = types.name(syms, pkgs, TypeId::User(inst.pkg, inst.typ));
			let mut res = String::new();
			res.push_str(&format!("{}(", name));
			for (i, field) in desc.ctor_fields.iter().enumerate() {
				let val = inst.fields.get(&field.name).unwrap();
				res.push_str(&rt_print_val(syms, types, pkgs, val));
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

pub fn rt_debug_val(syms: &Interner, types: &Types, pkgs: &Packages, val: &Val) -> String {
	match val {
		Val::Char(char) => format!("'{}'", char.0),
		Val::Str(str) => format!("\"{}\"", str.text),
		Val::Obj(rf) => rt_print_obj(syms, types, pkgs, &rf.borrow()),
		val => rt_print_val(syms, types, pkgs, val),
	}
}
