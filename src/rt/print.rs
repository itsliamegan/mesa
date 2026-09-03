use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::intern::Interner;
use crate::pkg::PackageId;
use crate::rt::Runtime;
use crate::rt::types::{NativeTypeId, TypeId};
use crate::rt::val::{Instance, Method, Obj, Proc, Val};
use crate::sem::modules::ModuleId;
use crate::sem::protos::ProtoId;
use crate::sem::types::Type;

type Active = HashSet<*const RefCell<Obj>>;

pub fn inspect_val(syms: &Interner, rt: &Runtime, val: &Val) -> String {
	print_val(syms, rt, val, &mut Active::new())
}

pub(crate) fn print_nil() -> String {
	String::from("nil")
}

pub(crate) fn print_num(num: f64) -> String {
	format!("{}", num)
}

pub(crate) fn print_bool(bool: bool) -> String {
	format!("{}", bool)
}

pub(crate) fn print_char(char: char) -> String {
	let mut res = String::new();
	res.push('\'');
	push_escaped(&mut res, char, '\'');
	res.push('\'');
	res
}

pub(crate) fn print_digest(digest: u64) -> String {
	format!("digest({:016x})", digest)
}

pub(crate) fn print_str(str: &str) -> String {
	let mut res = String::new();
	res.push('"');
	for char in str.chars() {
		push_escaped(&mut res, char, '"');
	}
	res.push('"');
	res
}

pub(crate) fn print_module(syms: &Interner, rt: &Runtime, pkg: PackageId, id: ModuleId) -> String {
	format!("module {}", rt.pkgs.get(pkg).modules.name(syms, id))
}

pub(crate) fn print_proto(syms: &Interner, rt: &Runtime, pkg: PackageId, id: ProtoId) -> String {
	let proto = rt.pkgs.desc(pkg).protos().get_proto(id);
	format!("proto {}", syms.resolve(proto.name))
}

pub(crate) fn print_native(syms: &Interner, rt: &Runtime, id: NativeTypeId) -> String {
	format!("<native {}>", syms.resolve(rt.natives.get(id).name))
}

pub(crate) fn print_type(syms: &Interner, rt: &Runtime, type_id: TypeId) -> String {
	match type_id {
		TypeId::User(pkg, id) => {
			let Type::User(desc) = rt.pkgs.get(pkg).types.descs.get_type(id) else {
				panic!()
			};
			let mut res = format!("type {}", rt.type_name(syms, TypeId::User(pkg, id)));
			if !desc.ctor_fields.is_empty() {
				res.push('(');
				for (i, field) in desc.ctor_fields.iter().enumerate() {
					res.push_str(syms.resolve(field.name));
					if i + 1 != desc.ctor_fields.len() {
						res.push_str(", ");
					}
				}
				res.push(')');
			}
			res
		}
		TypeId::Native(id) => format!("type {}", syms.resolve(rt.natives.get(id).name)),
	}
}

pub(crate) fn print_proc(syms: &Interner, proc: &Proc) -> String {
	let mut res = String::new();
	res.push_str(&format!("def {}", syms.resolve(proc.name)));
	if !proc.params.is_empty() {
		res.push('(');
	}
	for (i, param) in proc.params.iter().enumerate() {
		res.push_str(syms.resolve(param.name));
		if i + 1 != proc.params.len() {
			res.push_str(", ");
		}
	}
	if !proc.params.is_empty() {
		res.push(')');
	}
	res
}

pub(crate) fn print_method(syms: &Interner, method: &Method) -> String {
	match method {
		Method::User(_, proc) => print_proc(syms, &proc.borrow()),
		Method::Native(_, name, _) => format!("def {}", syms.resolve(*name)),
	}
}

pub(crate) fn instance_fields(
	syms: &Interner,
	rt: &Runtime,
	instance: &Instance,
) -> (String, Vec<(String, Val)>) {
	let type_id = TypeId::User(instance.pkg, instance.type_);
	let Type::User(desc) = rt
		.pkgs
		.get(instance.pkg)
		.types
		.descs
		.get_type(instance.type_)
	else {
		panic!()
	};
	let names = desc
		.ctor_fields
		.iter()
		.map(|field| field.name)
		.chain(desc.body_fields.iter().map(|(name, _)| *name));
	let fields = names
		.map(|name| {
			(
				syms.resolve(name).to_string(),
				instance.fields.get(&name).unwrap().clone(),
			)
		})
		.collect();
	(rt.type_name(syms, type_id), fields)
}

fn print_val(syms: &Interner, rt: &Runtime, val: &Val, active: &mut Active) -> String {
	match val {
		Val::Num(num) => print_num(num.0),
		Val::Bool(bool) => print_bool(bool.0),
		Val::Char(char) => print_char(char.0),
		Val::Str(str) => print_str(&str.text),
		Val::Digest(digest) => print_digest(digest.0),
		Val::Obj(obj) => {
			let ptr = Rc::as_ptr(obj);
			if !active.insert(ptr) {
				return String::from("...");
			}
			let res = print_obj(syms, rt, &obj.borrow(), active);
			active.remove(&ptr);
			res
		}
		Val::Nil(_) => print_nil(),
	}
}

fn print_obj(syms: &Interner, rt: &Runtime, obj: &Obj, active: &mut Active) -> String {
	match obj {
		Obj::Module(pkg, id) => print_module(syms, rt, *pkg, *id),
		Obj::List(list) => {
			let items = list
				.items
				.iter()
				.map(|item| print_val(syms, rt, item, active))
				.collect::<Vec<_>>();
			print_list(&items)
		}
		Obj::Dict(dict) => {
			let pairs = dict
				.pairs
				.iter()
				.map(|(key, val)| {
					(
						print_val(syms, rt, key, active),
						print_val(syms, rt, val, active),
					)
				})
				.collect::<Vec<_>>();
			print_dict(&pairs)
		}
		Obj::Proc(proc) => print_proc(syms, proc),
		Obj::Type(type_id) => print_type(syms, rt, *type_id),
		Obj::Proto(pkg, proto_id) => print_proto(syms, rt, *pkg, *proto_id),
		Obj::Instance(instance) => {
			let (name, fields) = instance_fields(syms, rt, instance);
			let fields = fields
				.iter()
				.map(|(field, val)| (field.clone(), print_val(syms, rt, val, active)))
				.collect::<Vec<_>>();
			print_instance(&name, &fields)
		}
		Obj::Method(method) => print_method(syms, method),
		Obj::Native(id, _) => print_native(syms, rt, *id),
	}
}

pub(crate) fn print_list(items: &[String]) -> String {
	let mut res = String::new();
	res.push('[');
	for (i, item) in items.iter().enumerate() {
		if i != 0 {
			res.push_str(", ");
		}
		res.push_str(item);
	}
	res.push(']');
	res
}

pub(crate) fn print_dict(pairs: &[(String, String)]) -> String {
	let mut res = String::new();
	res.push('{');
	for (i, (key, val)) in pairs.iter().enumerate() {
		if i != 0 {
			res.push_str(", ");
		}
		res.push_str(key);
		res.push_str(": ");
		res.push_str(val);
	}
	res.push('}');
	res
}

pub(crate) fn print_instance(name: &str, fields: &[(String, String)]) -> String {
	let mut res = format!("{}(", name);
	for (i, (field, val)) in fields.iter().enumerate() {
		if i != 0 {
			res.push_str(", ");
		}
		res.push_str(field);
		res.push_str(": ");
		res.push_str(val);
	}
	res.push(')');
	res
}

fn push_escaped(res: &mut String, char: char, quote: char) {
	match char {
		'\n' => res.push_str("\\n"),
		'\t' => res.push_str("\\t"),
		'\\' => res.push_str("\\\\"),
		char if char == quote => {
			res.push('\\');
			res.push(char);
		}
		char => res.push(char),
	}
}
