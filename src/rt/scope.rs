use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::intern::Sym;
use crate::pkg::PackageId;
use crate::rt::types::{MethodImpl, Static, TypeId};
use crate::rt::val::{Member, Method, Obj, Val};
use crate::rt::{Error, Runtime};
use crate::sem;
use crate::sem::modules::ModuleId;
use crate::syn::ChunkId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
	Local,
	Module(ModuleId),
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
	pub fn new(scope: Rc<RefCell<Scope>>, name: Sym) -> Local {
		let tier = scope.borrow().tier;
		Local { name, tier, scope }
	}

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

// Somewhere a value can be read from and (possibly) written to. A name in a
// scope, a member of a receiver, or a module reached by a module-form import.
pub enum Place {
	Local(Local),
	Member(Member),
	Module(PackageId, ModuleId),
}

impl Place {
	// Whether reading this place invokes what it holds. True only where a 'def'
	// declared the name: a method, a type-level method, or a module-level proc.
	// A field, a local, and a top-level assignment all read their value as-is,
	// whatever it holds.
	pub fn invokes(&self, rt: &Runtime, pkg: PackageId) -> bool {
		match self {
			// A 'def' never nests inside a proc body, so a local scope's names
			// are all bindings; a module scope holds both.
			Place::Local(local) => match local.tier {
				Tier::Local | Tier::Prelude => false,
				Tier::Module(id) => match rt.pkgs.get(pkg).modules.descs.member(id, local.name) {
					Some(sem::modules::Member::Proc(_)) => true,
					_ => false,
				},
			},
			Place::Member(Member::Module(pkg, id, name)) => {
				match rt.pkgs.get(*pkg).modules.descs.member(*id, *name) {
					Some(sem::modules::Member::Proc(_)) => true,
					_ => false,
				}
			}
			Place::Member(Member::Static(receiver, name)) => {
				let Obj::Type(type_id) = *receiver.borrow() else {
					panic!();
				};
				match type_id {
					// A nested type is declared by a 'type', not a 'def', so
					// reading it yields the type itself.
					TypeId::User(type_pkg, type_id) => {
						match rt.pkgs.get(type_pkg).types.static_(type_id, *name).unwrap() {
							Static::Proc(_) => true,
							Static::Type(_) => false,
						}
					}
					// A native type declares no nested types, only statics.
					TypeId::Native(_) => true,
				}
			}
			Place::Member(Member::Field(_, _)) => false,
			Place::Member(Member::Method(_, _)) => true,
			Place::Member(Member::Native(_, _)) => true,
			Place::Module(_, _) => false,
		}
	}

	pub fn get(&self, rt: &Runtime) -> Result<Val, Error> {
		match self {
			Place::Local(local) => Ok(local.get()),
			Place::Module(pkg, id) => Ok(Val::Obj(rt.pkgs.get(*pkg).modules.obj(*id))),
			Place::Member(Member::Module(pkg, id, name)) => {
				let pkg_modules = &rt.pkgs.get(*pkg).modules;
				match pkg_modules.descs.member(*id, *name).unwrap() {
					sem::modules::Member::Child(child) => Ok(Val::Obj(pkg_modules.obj(child))),
					sem::modules::Member::Type(_)
					| sem::modules::Member::Proto(_)
					| sem::modules::Member::Proc(_)
					| sem::modules::Member::Var(_) => {
						let scope = pkg_modules.scope(pkg_modules.descs.chunk(*id));
						Ok(scope.borrow().locals.get(name).cloned().unwrap())
					}
				}
			}
			Place::Member(Member::Static(receiver, name)) => {
				let Obj::Type(type_id) = *receiver.borrow() else {
					panic!();
				};
				match type_id {
					TypeId::User(type_pkg, type_id) => {
						// A nested type is declared inside its enclosing one, so
						// both live in the same package.
						let types = &rt.pkgs.get(type_pkg).types;
						match types.static_(type_id, *name).unwrap() {
							Static::Proc(proc) => Ok(Val::Obj(Rc::new(RefCell::new(Obj::Method(
								Method::User(Val::Obj(receiver.clone()), proc),
							))))),
							Static::Type(id) => Ok(Val::Obj(types.user(id).val.clone())),
						}
					}
					TypeId::Native(type_id) => {
						let method = match rt.natives.get(type_id).statics.get(name).unwrap() {
							MethodImpl::Native(method) => {
								Method::Native(Val::Obj(receiver.clone()), *name, *method)
							}
							MethodImpl::User(proc) => {
								Method::User(Val::Obj(receiver.clone()), proc.clone())
							}
						};
						Ok(Val::Obj(Rc::new(RefCell::new(Obj::Method(method)))))
					}
				}
			}
			Place::Member(Member::Field(receiver, name)) => {
				let Obj::Instance(instance) = &*receiver.borrow() else {
					panic!()
				};
				Ok(instance.fields.get(name).unwrap().clone())
			}
			Place::Member(Member::Method(receiver, name)) => {
				let Obj::Instance(instance) = &*receiver.borrow() else {
					panic!()
				};
				let member = rt
					.pkgs
					.get(instance.pkg)
					.types
					.method(instance.type_, *name)
					.unwrap();
				let method = match member {
					MethodImpl::Native(method) => {
						Method::Native(Val::Obj(receiver.clone()), *name, method)
					}
					MethodImpl::User(proc) => Method::User(Val::Obj(receiver.clone()), proc),
				};
				Ok(Val::Obj(Rc::new(RefCell::new(Obj::Method(method)))))
			}
			Place::Member(Member::Native(receiver, name)) => {
				let TypeId::Native(type_id) = receiver.type_id() else {
					panic!();
				};
				let method = match rt.natives.get(type_id).methods.get(name).unwrap() {
					MethodImpl::Native(method) => Method::Native(receiver.clone(), *name, *method),
					MethodImpl::User(proc) => Method::User(receiver.clone(), proc.clone()),
				};
				Ok(Val::Obj(Rc::new(RefCell::new(Obj::Method(method)))))
			}
		}
	}

	pub fn set(&self, val: Val) -> Result<(), ()> {
		match self {
			// Setting a local overwrites the value.
			Place::Local(local) => {
				local.set(val);
				Ok(())
			}
			Place::Module(_, _) => Err(()),
			Place::Member(Member::Module(_, _, _)) => Err(()),
			// Statics cannot be reassigned.
			Place::Member(Member::Static(_, _)) => Err(()),
			// Setting a field overwrites its value.
			Place::Member(Member::Field(receiver, name)) => {
				let Obj::Instance(instance) = &mut *receiver.borrow_mut() else {
					panic!()
				};
				instance.fields.insert(*name, val);
				Ok(())
			}
			// A method cannot be replaced by a field of the same name.
			Place::Member(Member::Method(_, _)) => Err(()),
			// Native members cannot be reassigned.
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
				return Local::new(origin.clone(), name);
			}
		}
	}

	// The module that this scope is lexically part of. Every local scope nests
	// inside exactly one Module scope, though it may need to reach up several
	// layers of outer scope to reach it.
	pub fn module(origin: &Rc<RefCell<Scope>>) -> ModuleId {
		let mut scope = origin.clone();
		loop {
			if let Tier::Module(id) = scope.borrow().tier {
				return id;
			}
			let outer = scope.borrow().outer.clone().unwrap();
			scope = outer;
		}
	}
}

pub struct Scopes {
	by_module: Vec<Rc<RefCell<Scope>>>,
}

impl Scopes {
	pub fn new(prelude: Rc<RefCell<Scope>>, modules: &sem::modules::Modules) -> Self {
		let mut by_module = vec![None; modules.ids().count()];
		for id in modules.ids() {
			let scope = Rc::new(RefCell::new(Scope {
				locals: FxHashMap::default(),
				outer: Some(prelude.clone()),
				tier: Tier::Module(id),
			}));
			by_module[modules.chunk(id).index()] = Some(scope);
		}
		let by_module = by_module.into_iter().map(Option::unwrap).collect();
		Self { by_module }
	}

	pub fn module(&self, chunk: ChunkId) -> Rc<RefCell<Scope>> {
		self.by_module[chunk.index()].clone()
	}
}
