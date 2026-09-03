mod err;
mod eval;
mod modules;
mod native;
pub mod pkg;
mod print;
mod protos;
mod scope;
mod types;
mod val;

use crate::intern::{Interner, Sym};
use crate::pkg::PackageId;
use crate::sem::protos::{Behaviors, ProtoId};
use crate::sem::types::Type;

pub use err::{Errors, build_errors};
pub use eval::{Prelude, Raise, Raised, build_prelude};
pub use native::TYPES as STDLIB_NATIVE_TYPES;
pub use pkg::Packages;
pub use print::inspect_val;
pub use types::{CORE_TYPES, NativeTypeSpec, Natives, TypeId};

use types::MethodImpl;
use val::{Member, Val};

// The runtime's representation of the world. Contains every package built so
// far and the native implementations any of them may bind an 'extern' to.
pub struct Runtime<'descs> {
	pub pkgs: Packages<'descs>,
	pub natives: Natives,
	pub behaviors: Behaviors,
	pub errors: Errors,
}

impl<'descs> Runtime<'descs> {
	pub fn new(
		descs: &'descs crate::pkg::Packages,
		natives: Natives,
		behaviors: Behaviors,
		errors: Errors,
	) -> Self {
		Self {
			pkgs: Packages::new(descs),
			natives,
			behaviors,
			errors,
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

	pub fn method(&self, type_id: TypeId, name: Sym) -> Option<MethodImpl> {
		match type_id {
			TypeId::User(pkg, id) => self.pkgs.get(pkg).types.method(id, name),
			TypeId::Native(id) => self.natives.get(id).methods.get(&name).cloned(),
		}
	}

	// Whether a type conforms to a protocol.
	pub fn conforms(&self, type_id: TypeId, proto: (PackageId, ProtoId)) -> bool {
		match type_id {
			TypeId::User(pkg, id) => self
				.pkgs
				.get(pkg)
				.types
				.descs
				.get_type(id)
				.impls()
				.contains(&proto),
			TypeId::Native(id) => self.natives.get(id).impls.contains(&proto),
		}
	}

	pub fn member(&self, val: &Val, name: Sym) -> Option<Member> {
		let type_id = val.type_id();
		let namespace_type_id = val.namespace_type_id();
		if type_id != namespace_type_id {
			// type_id and namespace_type_id only ever differ when the Val is
			// itself a Type.
			let Val::Obj(obj) = val else {
				panic!();
			};
			let has_static = match namespace_type_id {
				TypeId::User(pkg, id) => self.pkgs.get(pkg).types.static_(id, name).is_some(),
				TypeId::Native(id) => self.natives.get(id).has_static(name),
			};
			if has_static {
				Some(Member::Static(obj.clone(), name))
			} else {
				let TypeId::Native(id) = type_id else {
					panic!();
				};
				if self.natives.get(id).has_method(name) {
					Some(Member::Native(val.clone(), name))
				} else {
					None
				}
			}
		} else {
			match namespace_type_id {
				TypeId::User(pkg, id) => {
					let Val::Obj(obj) = val else {
						panic!();
					};
					let types = &self.pkgs.get(pkg).types;
					if types.field(id, name) || types.method(id, name).is_some() {
						Some(Member::User(obj.clone(), name))
					} else {
						None
					}
				}
				TypeId::Native(id) => {
					if self.natives.get(id).has_method(name) {
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
pub fn eval(syms: &mut Interner, rt: &mut Runtime, prelude: &Prelude) -> Result<(), Raise> {
	let ids: Vec<_> = rt.pkgs.ids().collect();
	for pkg_id in ids {
		let pkg = pkg::Package::new(
			prelude,
			&rt.pkgs,
			&mut rt.natives,
			&rt.behaviors,
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
	NameError(Sym),
	KeyError(String),
}

#[derive(Debug)]
pub enum ProtocolError {
	NotIterable(TypeId),
	NotAccessible(TypeId),
	NotAppendable(TypeId),
	NotOrderable(TypeId),
	NotInspectable(TypeId),
	NotDisplayable(TypeId),
	NotHashable(TypeId),
	NotImplemented(TypeId, Sym),
}

#[derive(Debug)]
pub enum ArgumentError {
	Missing(Vec<Sym>),
	TooMany(usize, usize),
	Unknown(Sym),
	Duplicate(Sym),
}

#[derive(Debug)]
pub enum TypeError {
	IndexNonNum(String),
	ArithNonNum(TypeId),
	ConcatNonStr(TypeId),
	MixNonDigest(TypeId),
	NotCallable(TypeId),
	NotConstructible(TypeId),
	NotInvocable(TypeId),
	CaseNonType(TypeId),
}

#[derive(Debug)]
pub enum MemberError {
	Missing(String, Sym),
	ReadOnly(String, Sym),
}

#[derive(Debug)]
pub enum IndexError {
	OutOfRange(f64),
	NonIntegral(f64),
}

impl Error {
	pub fn message(&self, syms: &Interner, rt: &Runtime) -> String {
		match self {
			Self::ProtocolError(err) => err.message(syms, rt),
			Self::ArgumentError(err) => err.message(syms),
			Self::TypeError(err) => err.message(syms, rt),
			Self::MemberError(err) => err.message(syms),
			Self::IndexError(err) => err.message(),
			Self::NameError(name) => format!("name '{}' is not defined", syms.resolve(*name)),
			Self::KeyError(key) => format!("key {} not found", key),
		}
	}
}

impl ProtocolError {
	fn message(&self, syms: &Interner, rt: &Runtime) -> String {
		match self {
			Self::NotIterable(id) => format!("type {} is not iterable", rt.type_name(syms, *id)),
			Self::NotAccessible(id) => {
				format!("type {} is not accessible", rt.type_name(syms, *id))
			}
			Self::NotAppendable(id) => {
				format!("type {} is not appendable", rt.type_name(syms, *id))
			}
			Self::NotOrderable(id) => format!("type {} is not orderable", rt.type_name(syms, *id)),
			Self::NotInspectable(id) => {
				format!("type {} is not inspectable", rt.type_name(syms, *id))
			}
			Self::NotDisplayable(id) => {
				format!("type {} is not displayable", rt.type_name(syms, *id))
			}
			Self::NotHashable(id) => {
				format!("type {} is not hashable", rt.type_name(syms, *id))
			}
			Self::NotImplemented(id, member) => format!(
				"type {} does not implement '{}'",
				rt.type_name(syms, *id),
				syms.resolve(*member)
			),
		}
	}
}

impl ArgumentError {
	fn message(&self, syms: &Interner) -> String {
		match self {
			Self::Missing(names) => {
				let noun = if names.len() == 1 { "arg" } else { "args" };
				let names = names
					.iter()
					.map(|name| format!("'{}'", syms.resolve(*name)))
					.collect::<Vec<_>>()
					.join(", ");
				format!("missing {} {}", noun, names)
			}
			Self::TooMany(have, want) => {
				format!("too many args; have {}, want at most {}", have, want)
			}
			Self::Unknown(name) => format!("no param named '{}'", syms.resolve(*name)),
			Self::Duplicate(name) => format!("arg '{}' given twice", syms.resolve(*name)),
		}
	}
}

impl TypeError {
	fn message(&self, syms: &Interner, rt: &Runtime) -> String {
		match self {
			Self::IndexNonNum(index) => format!("index {} is not a number", index),
			Self::ArithNonNum(id) => format!(
				"type {} cannot be used in arithmetic",
				rt.type_name(syms, *id)
			),
			Self::ConcatNonStr(id) => {
				format!("type {} cannot be concatenated", rt.type_name(syms, *id))
			}
			Self::MixNonDigest(id) => format!(
				"type {} cannot be mixed into a digest",
				rt.type_name(syms, *id)
			),
			Self::NotCallable(id) => format!("type {} is not callable", rt.type_name(syms, *id)),
			Self::NotConstructible(id) => {
				format!("type {} cannot be constructed", rt.type_name(syms, *id))
			}
			Self::NotInvocable(id) => format!("type {} is not invocable", rt.type_name(syms, *id)),
			Self::CaseNonType(id) => {
				format!("type {} cannot be matched against", rt.type_name(syms, *id))
			}
		}
	}
}

impl MemberError {
	fn message(&self, syms: &Interner) -> String {
		match self {
			Self::Missing(namespace, name) => {
				format!("{} has no such member '{}'", namespace, syms.resolve(*name))
			}
			Self::ReadOnly(namespace, name) => {
				format!(
					"member '{}' on {} is read-only",
					syms.resolve(*name),
					namespace
				)
			}
		}
	}
}

impl IndexError {
	fn message(&self) -> String {
		match self {
			Self::OutOfRange(index) => format!("index {} is out of bounds", index),
			Self::NonIntegral(index) => format!("index {} is not a whole number", index),
		}
	}
}
