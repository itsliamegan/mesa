mod eval;
mod modules;
mod native;
pub mod pkg;
mod scope;
mod types;
mod val;

use std::fmt::{self, Display, Formatter};

use crate::intern::{Interner, Sym};
use crate::sem::types::Type;
use crate::src::Location;

pub use eval::{Prelude, build_prelude};
pub use native::TYPES as STDLIB_NATIVE_TYPES;
pub use pkg::Packages;
pub use types::{CORE_TYPES, NativeTypeSpec, Natives, TypeId};

use val::{Member, Val};

// The runtime's representation of the world. Contains every package built so
// far and the native implementations any of them may bind an 'extern' to.
pub struct Runtime<'descs> {
	pub pkgs: Packages<'descs>,
	pub natives: Natives,
}

impl<'descs> Runtime<'descs> {
	pub fn new(descs: &'descs crate::pkg::Packages, natives: Natives) -> Self {
		Self {
			pkgs: Packages::new(descs),
			natives,
		}
	}

	// Get a type's *qualified* name (including all lexical nesting).
	pub fn type_name(&self, syms: &Interner, id: TypeId) -> String {
		match id {
			TypeId::User(pkg, id) => {
				let Type::User(desc) = self.pkgs.get(pkg).types.descs.get_type(id) else {
					panic!()
				};
				match desc.enclosing {
					Some(outer) => {
						let outer = self.type_name(syms, TypeId::User(pkg, outer));
						format!("{}.{}", outer, syms.resolve(desc.name))
					}
					None => syms.resolve(desc.name).to_string(),
				}
			}
			TypeId::Native(id) => syms.resolve(self.natives.get(id).name).to_string(),
		}
	}

	pub fn member(&self, val: &Val, name: Sym) -> Option<Member> {
		let type_id = val.type_id();
		let namespace_type_id = val.namespace_type_id();
		if type_id != namespace_type_id {
			// type_id and namespace_type_id only ever differ when the Val is
			// itself a Type.
			let Val::Obj(rf) = val else {
				panic!();
			};
			let has_static = match namespace_type_id {
				TypeId::User(pkg, id) => self.pkgs.get(pkg).types.static_(id, name).is_some(),
				TypeId::Native(id) => self.natives.get(id).has_static(name),
			};
			if has_static {
				Some(Member::Static(rf.clone(), name))
			} else {
				let TypeId::Native(id) = type_id else {
					panic!();
				};
				if self.natives.get(id).has_member(name) {
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
					let types = &self.pkgs.get(pkg).types;
					if types.field(id, name) || types.method(id, name).is_some() {
						Some(Member::User(rf.clone(), name))
					} else {
						None
					}
				}
				TypeId::Native(id) => {
					if self.natives.get(id).has_member(name) {
						Some(Member::Native(val.clone(), name))
					} else {
						None
					}
				}
			}
		}
	}
}

// Evaluate the packages in the runtime, *in dependency order*.
pub fn eval(
	syms: &mut Interner,
	rt: &mut Runtime,
	prelude: &Prelude,
) -> Result<(), (Error, Vec<(String, Location)>)> {
	let ids: Vec<_> = rt.pkgs.ids().collect();
	for pkg_id in ids {
		let pkg = pkg::Package::new(
			prelude,
			&rt.pkgs,
			&mut rt.natives,
			rt.pkgs.desc(pkg_id),
			pkg_id,
		);
		rt.pkgs.insert(pkg_id, pkg);
		eval::Interpreter::new(syms, rt, pkg_id).eval()?;
	}
	Ok(())
}

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
			Self::Missing(namespace, name) => {
				write!(f, "{} has no such member '{}'", namespace, name)
			}
			Self::ReadOnly(namespace, name) => {
				write!(f, "member '{}' on {} is read-only", name, namespace)
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
