use std::cell::RefCell;
use std::rc::Rc;

use crate::pkg::PackageId;
use crate::rt::val::Obj;
use crate::sem::protos::{self, ProtoId};

// The runtime image of every protocol in the package. Contains an object for
// each protocol, keyed by its ProtoId, so that a protocol has one value for the
// life of the runtime.
pub struct Protos<'descs> {
	pub descs: &'descs protos::Protos,
	objs: Vec<Rc<RefCell<Obj>>>,
}

impl<'descs> Protos<'descs> {
	pub fn new(pkg_id: PackageId, descs: &'descs protos::Protos) -> Self {
		let objs = descs
			.proto_ids()
			.map(|id| Rc::new(RefCell::new(Obj::Proto(pkg_id, id))))
			.collect();
		Self { descs, objs }
	}

	pub fn obj(&self, id: ProtoId) -> Rc<RefCell<Obj>> {
		self.objs[id.index()].clone()
	}
}
