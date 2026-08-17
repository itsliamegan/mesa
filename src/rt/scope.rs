use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::intern::Sym;
use crate::rt::Error;
use crate::rt::types::{Static, TypeId, Types};
use crate::rt::val::{Member, Method, Obj, Val};
use crate::syn::ChunkId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
	Local,
	Module,
	Prelude,
}

#[derive(Debug)]
pub struct Scope {
	pub locals: FxHashMap<Sym, Val>,
	pub outer: Option<Rc<RefCell<Scope>>>,
	pub tier: Tier,
}

pub struct Local {
	pub name: Sym,
	pub tier: Tier,
	scope: Rc<RefCell<Scope>>,
}

impl Local {
	pub fn is_bound(&self) -> bool {
		self.scope.borrow().locals.contains_key(&self.name)
	}

	fn get(&self) -> Val {
		self.scope.borrow().locals.get(&self.name).cloned().unwrap()
	}

	fn set(&self, val: Val) {
		self.scope.borrow_mut().locals.insert(self.name, val);
	}
}

// Somewhere a value can be read from and written to. Either a name in a scope
// or a member of a receiver.
pub enum Place {
	Local(Local),
	Member(Member),
}

impl Place {
	// Whether this place holds a value yet. A member always does, because it
	// would not have resolved otherwise, but a local may have not been written
	// yet.
	pub fn is_bound(&self) -> bool {
		match self {
			Place::Local(local) => local.is_bound(),
			Place::Member(_) => true,
		}
	}

	pub fn get(&self, types: &Types) -> Result<Val, Error> {
		match self {
			Place::Local(local) => Ok(local.get()),
			Place::Member(Member::Static(type_rf, name)) => {
				// A static is only ever found on a user type.
				let Obj::Type(TypeId::User(type_id)) = *type_rf.borrow() else {
					panic!();
				};
				match types.static_(type_id, *name).unwrap() {
					Static::Proc(proc_rf) => Ok(Val::Obj(Rc::new(RefCell::new(Obj::Method(
						Method::User(type_rf.clone(), proc_rf),
					))))),
					Static::Type(id) => Ok(Val::Obj(types.user(id).val.clone())),
				}
			}
			Place::Member(Member::User(inst_rf, name)) => {
				let Obj::Instance(inst) = &*inst_rf.borrow() else {
					panic!()
				};
				if let Some(val) = inst.fields.get(name) {
					Ok(val.clone())
				} else if let Some(proc_rf) = types.method(inst.typ, *name) {
					Ok(Val::Obj(Rc::new(RefCell::new(Obj::Method(Method::User(
						inst_rf.clone(),
						proc_rf,
					))))))
				} else {
					panic!();
				}
			}
			Place::Member(Member::Native(recv, name)) => {
				let TypeId::Native(type_id) = recv.type_id() else {
					panic!();
				};
				match types.native(type_id).members.get(name) {
					Some(member) => Ok(Val::Obj(Rc::new(RefCell::new(Obj::Method(
						Method::Native(recv.clone(), *name, member.clone()),
					))))),
					None => panic!(),
				}
			}
		}
	}

	pub fn set(&self, types: &Types, val: Val) -> Result<(), ()> {
		match self {
			Place::Local(local) => {
				local.set(val);
				Ok(())
			}
			Place::Member(Member::Static(_, _)) => Err(()),
			Place::Member(Member::User(inst_rf, name)) => {
				let Obj::Instance(inst) = &mut *inst_rf.borrow_mut() else {
					panic!()
				};
				// A method cannot be replaced by a field of the same name.
				if !inst.fields.contains_key(name) && types.method(inst.typ, *name).is_some() {
					return Err(());
				}
				inst.fields.insert(*name, val);
				Ok(())
			}
			Place::Member(Member::Native(_, _)) => Err(()),
		}
	}
}

impl Scope {
	pub fn local(origin: &Rc<RefCell<Scope>>, name: Sym) -> Local {
		let mut scope = origin.clone();
		loop {
			if scope.borrow().locals.contains_key(&name) {
				let tier = scope.borrow().tier;
				return Local { name, tier, scope };
			}
			let outer = scope.borrow().outer.clone();
			if let Some(outer) = outer {
				scope = outer;
			} else {
				return Local {
					name,
					tier: origin.borrow().tier,
					scope: origin.clone(),
				};
			}
		}
	}
}

// The scope for every module, keyed by the module's ChunkId.
pub struct Scopes {
	modules: Vec<Rc<RefCell<Scope>>>,
}

impl Scopes {
	pub fn new(prelude: Rc<RefCell<Scope>>, chunks: usize) -> Self {
		let mut modules = Vec::with_capacity(chunks);
		for _ in 0..chunks {
			let scope = Rc::new(RefCell::new(Scope {
				locals: FxHashMap::default(),
				outer: Some(prelude.clone()),
				tier: Tier::Module,
			}));
			modules.push(scope);
		}
		Self { modules }
	}

	pub fn module(&self, chunk: ChunkId) -> Rc<RefCell<Scope>> {
		self.modules[chunk.index()].clone()
	}
}
