use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use crate::intern::Interner;
use crate::rt::Runtime;
use crate::rt::eval::{Interpreter, Raise};
use crate::rt::types::TypeId;
use crate::rt::val::{Dict, Instance, List, Method, Obj, Proc, Str, Val};
use crate::sem::types::Type;

#[derive(Clone, Copy, Eq, PartialEq)]
enum Form {
	Display,
	Inspect,
}

type Active = HashSet<*const RefCell<Obj>>;

pub fn display_val(syms: &Interner, rt: &Runtime, val: &Val) -> String {
	render_val(syms, rt, val, Form::Display, &mut Active::new())
}

pub fn inspect_val(syms: &Interner, rt: &Runtime, val: &Val) -> String {
	render_val(syms, rt, val, Form::Inspect, &mut Active::new())
}

// The 'inspect' a type gets when it declares none. The whole walk happens here,
// so the cycle guard above still sees all of it.
pub fn inspect_member(interp: &mut Interpreter, val: &Val, _args: Vec<Val>) -> Result<Val, Raise> {
	let text = inspect_val(interp.syms(), interp.rt(), val);
	Ok(Val::Str(Rc::new(Str {
		text: Box::from(text.as_str()),
		size: Cell::new(None),
	})))
}

fn render_val(syms: &Interner, rt: &Runtime, val: &Val, form: Form, active: &mut Active) -> String {
	match val {
		Val::Num(num) => format!("{}", num.0),
		Val::Bool(bool) => format!("{}", bool.0),
		Val::Char(char) => render_char(char.0, form),
		Val::Str(str) => render_str(&str.text, form),
		Val::Obj(obj) => {
			let ptr = Rc::as_ptr(obj);
			if !active.insert(ptr) {
				return String::from("...");
			}
			let res = render_obj(syms, rt, &obj.borrow(), form, active);
			active.remove(&ptr);
			res
		}
		Val::Nil(_) => String::from("nil"),
	}
}

fn render_char(char: char, form: Form) -> String {
	match form {
		Form::Display => char.to_string(),
		Form::Inspect => {
			let mut res = String::new();
			res.push('\'');
			push_escaped(&mut res, char, '\'');
			res.push('\'');
			res
		}
	}
}

fn render_str(str: &str, form: Form) -> String {
	match form {
		Form::Display => str.to_string(),
		Form::Inspect => {
			let mut res = String::new();
			res.push('"');
			for char in str.chars() {
				push_escaped(&mut res, char, '"');
			}
			res.push('"');
			res
		}
	}
}

fn render_obj(syms: &Interner, rt: &Runtime, obj: &Obj, form: Form, active: &mut Active) -> String {
	match obj {
		Obj::Module(pkg, id) => format!("module {}", rt.pkgs.get(*pkg).modules.name(syms, *id)),
		Obj::List(list) => render_list(syms, rt, list, active),
		Obj::Dict(dict) => render_dict(syms, rt, dict, active),
		Obj::Proc(proc) => render_proc(syms, proc),
		Obj::Type(type_id) => match type_id {
			TypeId::User(type_pkg, type_id) => {
				let Type::User(desc) = rt.pkgs.get(*type_pkg).types.descs.get_type(*type_id) else {
					panic!()
				};
				let name = rt.type_name(syms, TypeId::User(*type_pkg, *type_id));
				let mut res = format!("type {}", name);
				if !desc.ctor_fields.is_empty() {
					res.push('(');
					for (i, field) in desc.ctor_fields.iter().enumerate() {
						let field = syms.resolve(field.name);
						res.push_str(field);
						if i + 1 != desc.ctor_fields.len() {
							res.push_str(", ");
						}
					}
					res.push(')');
				}
				res
			}
			TypeId::Native(type_id) => {
				let type_ = rt.natives.get(*type_id);
				let name = syms.resolve(type_.name);
				format!("type {}", name)
			}
		},
		Obj::Proto(pkg, proto_id) => {
			let proto = rt.pkgs.desc(*pkg).protos().get_proto(*proto_id);
			format!("proto {}", syms.resolve(proto.name))
		}
		Obj::Instance(instance) => render_instance(syms, rt, instance, form, active),
		Obj::Method(method) => match method {
			Method::User(_, proc) => render_proc(syms, &proc.borrow()),
			Method::Native(_, name, _) => format!("def {}", syms.resolve(*name)),
		},
		Obj::Native(id, _) => format!("<native {}>", syms.resolve(rt.natives.get(*id).name)),
	}
}

fn render_list(syms: &Interner, rt: &Runtime, list: &List, active: &mut Active) -> String {
	let mut res = String::new();
	res.push('[');
	for (i, item) in list.items.iter().enumerate() {
		res.push_str(&render_val(syms, rt, item, Form::Inspect, active));
		if i + 1 != list.items.len() {
			res.push_str(", ");
		}
	}
	res.push(']');
	res
}

fn render_dict(syms: &Interner, rt: &Runtime, dict: &Dict, active: &mut Active) -> String {
	let mut res = String::new();
	res.push('{');
	for (i, (key, val)) in dict.pairs.iter().enumerate() {
		res.push_str(&render_val(syms, rt, key, Form::Inspect, active));
		res.push_str(": ");
		res.push_str(&render_val(syms, rt, val, Form::Inspect, active));
		if i + 1 != dict.pairs.len() {
			res.push_str(", ");
		}
	}
	res.push('}');
	res
}

fn render_proc(syms: &Interner, proc: &Proc) -> String {
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

fn render_instance(
	syms: &Interner,
	rt: &Runtime,
	instance: &Instance,
	form: Form,
	active: &mut Active,
) -> String {
	let Type::User(desc) = rt
		.pkgs
		.get(instance.pkg)
		.types
		.descs
		.get_type(instance.type_)
	else {
		panic!()
	};
	let name = rt.type_name(syms, TypeId::User(instance.pkg, instance.type_));
	let mut res = String::new();
	res.push_str(&format!("{}(", name));
	let mut first = true;
	for field in &desc.ctor_fields {
		if !first {
			res.push_str(", ");
		}
		res.push_str(syms.resolve(field.name));
		res.push_str(": ");
		let val = instance.fields.get(&field.name).unwrap();
		res.push_str(&render_val(syms, rt, val, Form::Inspect, active));
		first = false;
	}
	if form == Form::Inspect {
		for (name, _) in &desc.body_fields {
			if !first {
				res.push_str(", ");
			}
			res.push_str(syms.resolve(*name));
			res.push_str(": ");
			let val = instance.fields.get(name).unwrap();
			res.push_str(&render_val(syms, rt, val, Form::Inspect, active));
			first = false;
		}
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
