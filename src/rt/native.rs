use std::cell::RefCell;
use std::rc::Rc;

use crate::rt::Error;
use crate::rt::types::{NativeParam, NativeTypeSpec, TypeId};
use crate::rt::val::{NativeData, Num, Obj, Val};

// The native types the stdlib provides.
pub const TYPES: &[NativeTypeSpec] = &[NativeTypeSpec {
	name: "Counter",
	new: None,
	members: &[
		("count", &[], Counter::count),
		("bump", &[], Counter::bump),
		(
			"order",
			&[NativeParam {
				name: "other",
				default: None,
			}],
			Counter::order,
		),
	],
	statics: &[("zero", &[], Counter::zero)],
}];

// A mutable tally. Deliberately dull: it exists to exercise native data end to
// end.
struct Counter {
	count: f64,
}

impl Counter {
	fn zero(val: &Val, _args: Vec<Val>) -> Result<Val, Error> {
		// The receiver of a static is the type value itself, so the id wanted
		// here is the type it names, not the type it is.
		let TypeId::Native(id) = val.namespace_type_id() else {
			panic!();
		};
		let counter = Counter { count: 0.0 };
		Ok(Val::Obj(Rc::new(RefCell::new(Obj::Native(
			id,
			NativeData(Box::new(counter)),
		)))))
	}

	fn order(val: &Val, args: Vec<Val>) -> Result<Val, Error> {
		let Val::Num(Num(count)) = Counter::count(val, Vec::new())? else {
			panic!();
		};
		let Val::Num(Num(other)) = Counter::count(&args[0], Vec::new())? else {
			panic!();
		};
		Ok(Val::Num(Num(count - other)))
	}

	fn count(val: &Val, _args: Vec<Val>) -> Result<Val, Error> {
		let Val::Obj(rf) = val else {
			panic!();
		};
		let Obj::Native(_, data) = &*rf.borrow() else {
			panic!();
		};
		Ok(Val::Num(Num(data
			.0
			.downcast_ref::<Counter>()
			.unwrap()
			.count)))
	}

	fn bump(val: &Val, _args: Vec<Val>) -> Result<Val, Error> {
		let Val::Obj(rf) = val else {
			panic!();
		};
		let Obj::Native(_, data) = &mut *rf.borrow_mut() else {
			panic!();
		};
		let counter = data.0.downcast_mut::<Counter>().unwrap();
		counter.count += 1.0;
		Ok(Val::Num(Num(counter.count)))
	}
}
