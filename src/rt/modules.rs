use std::cell::RefCell;
use std::rc::Rc;

use crate::intern::Interner;
use crate::rt::scope::{Scope, Scopes};
use crate::rt::val::Obj;
use crate::sem::modules::{self, ModuleId};
use crate::syn::nodes::ChunkId;

// The runtime image of every module in the package. Contains an object for each
// module, keyed by its ModuleId, and a corresponding scope keyed by its
// ChunkId.
pub struct Modules<'descs> {
	pub descs: &'descs modules::Modules,
	objs: Vec<Rc<RefCell<Obj>>>,
	scopes: Scopes,
}

impl<'descs> Modules<'descs> {
	pub fn new(descs: &'descs modules::Modules, scopes: Scopes) -> Self {
		let objs = descs
			.ids()
			.map(|id| Rc::new(RefCell::new(Obj::Module(id))))
			.collect();
		Self {
			descs,
			objs,
			scopes,
		}
	}

	pub fn obj(&self, id: ModuleId) -> Rc<RefCell<Obj>> {
		self.objs[id.index()].clone()
	}

	pub fn scope(&self, chunk: ChunkId) -> Rc<RefCell<Scope>> {
		self.scopes.module(chunk)
	}

	// A module's qualified dotted name.
	pub fn name(&self, syms: &Interner, id: ModuleId) -> String {
		syms.resolve_path(self.descs.path(id))
	}
}
