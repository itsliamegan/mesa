use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::intern::{Interner, Sym};
use crate::pkg::{self, PackageId};
use crate::rt::types::{Static, TypeId};
use crate::rt::val::{Instance, Obj, Str, Val};
use crate::rt::{ArgumentError, Error, IndexError, MemberError, ProtocolError, Runtime, TypeError};
use crate::sem::modules;
use crate::sem::types::{self as types, Type};

// Identity of the module declaring the built-in error taxonomy, resolved once
// at boot.
pub struct Errors {
	pkg: PackageId,
	module: modules::ModuleId,
}

// Resolve 'Core.Errors', the module declaring the built-in error taxonomy, by
// name. Check it declares exactly the taxonomy the runtime errors expect. A
// missing or misnamed error variant is a broken build.
pub fn build_errors(descs: &pkg::Packages, syms: &mut Interner, stdlib: PackageId) -> Errors {
	let stdlib_pkg = descs.get(stdlib);
	let path = [syms.intern("Core"), syms.intern("Errors")];
	let Some(module) = stdlib_pkg.modules.by_path(&path) else {
		panic!("stdlib does not declare a 'Core.Errors' module");
	};

	let errors = Errors {
		pkg: stdlib,
		module,
	};
	validate_taxonomy(stdlib_pkg, syms, &errors);
	errors
}

// Every outer variant of 'Error', and, for a nested one, every inner variant of
// its own top-level group type. This table exists to ensure that the
// declarations in the standard library don't drift from the runtime.
const TAXONOMY: &[(&str, &[&str])] = &[
	(
		"ProtocolError",
		&[
			"NotIterable",
			"NotAccessible",
			"NotAppendable",
			"NotOrderable",
		],
	),
	(
		"ArgumentError",
		&["Missing", "TooMany", "Unknown", "Duplicate"],
	),
	(
		"TypeError",
		&[
			"IndexNonNum",
			"ArithNonNum",
			"ConcatNonStr",
			"NotCallable",
			"NotConstructible",
			"NotInvokable",
			"NotRaisable",
			"CaseNonType",
		],
	),
	("MemberError", &["Missing", "ReadOnly"]),
	("IndexError", &["OutOfRange", "NonIntegral"]),
	("NameError", &[]),
	("KeyError", &[]),
];

// A module member that names a type.
fn module_type(
	pkg: &pkg::Package,
	syms: &Interner,
	module: modules::ModuleId,
	name: Sym,
) -> types::TypeId {
	let Some(modules::Member::Type(item_id)) = pkg.modules.member(module, name) else {
		panic!("'Core.Errors' does not declare '{}'", syms.resolve(name));
	};
	pkg.types
		.get_type_by_item(pkg.modules.chunk(module), item_id)
}

// A case variant of a type.
fn case_type(pkg: &pkg::Package, syms: &Interner, base: types::TypeId, name: Sym) -> types::TypeId {
	let Type::User(desc) = pkg.types.get_type(base) else {
		panic!()
	};
	let Some(types::Static::Type(id)) = desc.statics.get(&name) else {
		panic!("type does not declare case '{}'", syms.resolve(name));
	};
	*id
}

// Walk 'TAXONOMY' against the standard library's actual description, checking
// every path resolves and every case has the single constructor field a reified
// instance needs ('message' on a leaf, 'error' on a group)
fn validate_taxonomy(pkg: &pkg::Package, syms: &mut Interner, errors: &Errors) {
	let error_sym = syms.intern("Error");
	let error_id = module_type(pkg, syms, errors.module, error_sym);

	for (outer_name, inner_names) in TAXONOMY {
		let outer = syms.intern(outer_name);
		let outer_id = case_type(pkg, syms, error_id, outer);
		check_ctor_field(pkg, outer_id);

		if inner_names.is_empty() {
			continue;
		}
		let group_id = module_type(pkg, syms, errors.module, outer);
		for inner_name in *inner_names {
			let inner = syms.intern(inner_name);
			let inner_id = case_type(pkg, syms, group_id, inner);
			check_ctor_field(pkg, inner_id);
		}
	}
}

fn check_ctor_field(pkg: &pkg::Package, id: types::TypeId) {
	let Type::User(desc) = pkg.types.get_type(id) else {
		panic!()
	};
	if desc.ctor_fields.len() != 1 {
		panic!(
			"error case must declare exactly one field, has {}",
			desc.ctor_fields.len()
		);
	}
}

// The path to an error variant that a runtime error takes to reify. An outer
// type name followed by an optional variant name.
fn path(err: &Error) -> (&'static str, Option<&'static str>) {
	match err {
		Error::ProtocolError(err) => (
			"ProtocolError",
			Some(match err {
				ProtocolError::NotIterable(_) => "NotIterable",
				ProtocolError::NotAccessible(_) => "NotAccessible",
				ProtocolError::NotAppendable(_) => "NotAppendable",
				ProtocolError::NotOrderable(_) => "NotOrderable",
			}),
		),
		Error::ArgumentError(err) => (
			"ArgumentError",
			Some(match err {
				ArgumentError::Missing(_) => "Missing",
				ArgumentError::TooMany(_, _) => "TooMany",
				ArgumentError::Unknown(_) => "Unknown",
				ArgumentError::Duplicate(_) => "Duplicate",
			}),
		),
		Error::TypeError(err) => (
			"TypeError",
			Some(match err {
				TypeError::IndexNonNum(_) => "IndexNonNum",
				TypeError::ArithNonNum(_) => "ArithNonNum",
				TypeError::ConcatNonStr(_) => "ConcatNonStr",
				TypeError::NotCallable(_) => "NotCallable",
				TypeError::NotConstructible(_) => "NotConstructible",
				TypeError::NotInvokable(_) => "NotInvokable",
				TypeError::NotRaisable(_) => "NotRaisable",
				TypeError::CaseNonType(_) => "CaseNonType",
			}),
		),
		Error::MemberError(err) => (
			"MemberError",
			Some(match err {
				MemberError::Missing(_, _) => "Missing",
				MemberError::ReadOnly(_, _) => "ReadOnly",
			}),
		),
		Error::IndexError(err) => (
			"IndexError",
			Some(match err {
				IndexError::OutOfRange(_) => "OutOfRange",
				IndexError::NonIntegral(_) => "NonIntegral",
			}),
		),
		Error::NameError(_) => ("NameError", None),
		Error::KeyError(_) => ("KeyError", None),
	}
}

impl Errors {
	// Resolve a two-segment path to an error. The first component is a member
	// of 'Core.Errors' or of an outer variant's own group type ('base'). The
	// second component is one of the outer type's case variants ('leaf').
	fn resolve(&self, rt: &Runtime, syms: &Interner, base: Sym, leaf: Sym) -> types::TypeId {
		let mods = &rt.pkgs.get(self.pkg).mods;
		let Some(modules::Member::Type(item_id)) = mods.descs.member(self.module, base) else {
			panic!("'Core.Errors' does not declare '{}'", syms.resolve(base));
		};
		let chunk_id = mods.descs.chunk(self.module);
		let base_id = rt
			.pkgs
			.get(self.pkg)
			.types
			.descs
			.get_type_by_item(chunk_id, item_id);
		let Some(Static::Type(leaf_id)) = rt.pkgs.get(self.pkg).types.static_(base_id, leaf) else {
			panic!(
				"'{}' does not declare case '{}'",
				syms.resolve(base),
				syms.resolve(leaf)
			);
		};
		leaf_id
	}

	// Create an instance of an error type, setting the first constructor field
	// (the error message) to a particular value.
	fn instance(&self, rt: &Runtime, type_id: types::TypeId, val: Val) -> Val {
		let types::Type::User(desc) = rt.pkgs.get(self.pkg).types.descs.get_type(type_id) else {
			panic!()
		};
		let mut fields = FxHashMap::default();
		fields.insert(desc.ctor_fields[0].name, val);
		Val::Obj(Rc::new(RefCell::new(Obj::Instance(Instance {
			pkg: self.pkg,
			typ: type_id,
			fields,
		}))))
	}

	// Determine the TypeId of the outer variant this error reifies to.
	pub fn variant(&self, syms: &mut Interner, rt: &Runtime, err: &Error) -> TypeId {
		let (outer_name, _) = path(err);
		let error = syms.intern("Error");
		let outer = syms.intern(outer_name);
		TypeId::User(self.pkg, self.resolve(rt, syms, error, outer))
	}

	// Reify a runtime error as a user-visible error value.
	pub fn reify(&self, syms: &mut Interner, rt: &Runtime, err: &Error) -> Val {
		let (outer_name, inner_name) = path(err);
		let error = syms.intern("Error");
		let outer = syms.intern(outer_name);
		let outer_id = self.resolve(rt, syms, error, outer);
		let message = Val::Str(Rc::new(Str {
			text: err.message(syms, rt).into_boxed_str(),
			size: Cell::new(None),
		}));

		match inner_name {
			None => self.instance(rt, outer_id, message),
			Some(inner_name) => {
				let inner = syms.intern(inner_name);
				let inner_id = self.resolve(rt, syms, outer, inner);
				let inner_val = self.instance(rt, inner_id, message);
				self.instance(rt, outer_id, inner_val)
			}
		}
	}
}
