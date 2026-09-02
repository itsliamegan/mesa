use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use rustc_hash::{FxBuildHasher, FxHashMap};

use crate::intern::{Interner, Sym};
use crate::pkg::{self, PackageId};
use crate::rt::modules::Modules;
use crate::rt::scope::{Local, Place, Scope, Tier};
use crate::rt::types::{MethodImpl, NativeMethod, NativeParam, Natives, TypeId, Types};
use crate::rt::val::{
	Bool, Char, Dict, Instance, List, Member, Method, Nil, Num, Obj, Proc, Str, Val, namespace_name,
};
use crate::rt::{
	ArgumentError, Error, IndexError, MemberError, ProtocolError, Runtime, TypeError, inspect_val,
};
use crate::sem::modules;
use crate::sem::types::Type;
use crate::src::{Location, Span};
use crate::syn::nodes::{
	Arm, BinaryOp, BlockId, Builtin, Expr, ExprId, Lit, ModuleItem, ModuleItemId, Param, UnaryOp,
};
use crate::syn::{self, Chunk, ChunkId};

pub struct Prelude {
	pub(super) scope: Rc<RefCell<Scope>>,
}

// Bind every name 'Core.Prelude' declares to its canonical value. Only what
// that module declares is in the prelude.
pub fn build_prelude(
	descs: &pkg::Packages,
	syms: &mut Interner,
	stdlib: PackageId,
	natives: &Natives,
) -> Result<Prelude, String> {
	let stdlib_pkg = descs.get(stdlib);
	let path = [syms.intern("Core"), syms.intern("Prelude")];
	let Some(prelude_mod) = stdlib_pkg.modules().by_path(&path) else {
		return Err("stdlib does not declare a 'Core.Prelude' module".to_string());
	};
	let chunk_id = stdlib_pkg.modules().chunk(prelude_mod);
	let chunk = stdlib_pkg.chunks().get(chunk_id);

	let mut locals = HashMap::with_capacity_and_hasher(chunk.top.len(), FxBuildHasher);
	for item_id in &chunk.top {
		// 'Core.Prelude' declares nothing but extern types, so this is the
		// only module item that matters here.
		if let ModuleItem::Extern(extern_) = chunk.get_module_item(*item_id) {
			let type_id = stdlib_pkg.types().get_type_by_item(chunk_id, *item_id);
			let Type::Native(desc) = stdlib_pkg.types().get_type(type_id) else {
				panic!()
			};
			locals.insert(extern_.name, Val::Obj(natives.val(desc.provider)));
		}
	}
	let scope = Rc::new(RefCell::new(Scope {
		locals,
		outer: None,
		tier: Tier::Prelude,
	}));

	Ok(Prelude { scope })
}

pub struct Interpreter<'syms, 'descs, 'rt> {
	syms: &'syms mut Interner,
	rt: &'rt Runtime<'descs>,
	pkg_id: PackageId,
	scope: Rc<RefCell<Scope>>,
	receiver: Option<Val>,
	call_span: Option<Span>,
	printing: HashSet<*const RefCell<Obj>>,
	print_depth: usize,
	equating: Vec<(*const RefCell<Obj>, *const RefCell<Obj>)>,
	// Remaining node budget during a hash traversal; absent between traversals.
	hash_nodes: Option<usize>,
}

const MAX_PRINT_DEPTH: usize = 64;
const MAX_HASH_NODES: usize = 64;

pub enum Raised {
	Native(Error),
	Val(Val),
}

pub struct Raise(pub Raised, pub Vec<(String, Location)>);

enum Signal {
	Return(Val),
	Break(Val),
	Raise(Raise),
}

impl Signal {
	fn raise(raised: Raised, trace: Vec<(String, Location)>) -> Self {
		Self::Raise(Raise(raised, trace))
	}
}

impl<'syms, 'descs, 'rt> Interpreter<'syms, 'descs, 'rt> {
	pub fn new(syms: &'syms mut Interner, rt: &'rt Runtime<'descs>, pkg_id: PackageId) -> Self {
		let desc = rt.pkgs.desc(pkg_id);
		let first_chunk = desc.modules().chunk(desc.modules().ids().next().unwrap());
		let scope = rt.pkgs.get(pkg_id).modules.scope(first_chunk);

		Self {
			syms,
			rt,
			pkg_id,
			scope,
			receiver: None,
			call_span: None,
			printing: HashSet::new(),
			print_depth: 0,
			equating: Vec::new(),
			hash_nodes: None,
		}
	}

	#[allow(dead_code)]
	pub(crate) fn syms(&self) -> &Interner {
		self.syms
	}

	#[allow(dead_code)]
	pub(crate) fn rt(&self) -> &Runtime<'_> {
		self.rt
	}

	pub(crate) fn native_error(&self, err: Error) -> Raise {
		let span = self.call_span.unwrap();
		let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
		Raise(Raised::Native(err), vec![(String::new(), loc)])
	}

	#[allow(dead_code)]
	pub(crate) fn call(
		&mut self,
		receiver: &Val,
		member: Sym,
		args: Vec<Val>,
	) -> Result<Val, Raise> {
		let span = self.call_span.unwrap();
		let args = args.into_iter().map(|arg| (None, arg)).collect();
		match self.call_member(span, receiver.clone(), member, args) {
			Ok(val) => Ok(val),
			Err(Signal::Raise(raise)) => Err(raise),
			Err(Signal::Return(_) | Signal::Break(_)) => panic!(),
		}
	}

	fn types(&self) -> &'rt Types<'descs> {
		&self.rt.pkgs.get(self.pkg_id).types
	}

	fn modules(&self) -> &'rt Modules<'descs> {
		&self.rt.pkgs.get(self.pkg_id).modules
	}

	pub fn eval(mut self) -> Result<(), Raise> {
		// Declarations are order-independent, so every module's items enter its
		// scope before anything is evaluated.
		for id in self.modules().descs.ids() {
			let chunk_id = self.modules().descs.chunk(id);
			self.scope = self.modules().scope(chunk_id);
			let chunk = self.rt.pkgs.desc(self.pkg_id).chunks().get(chunk_id);
			for item_id in &chunk.top {
				self.bind_module_item(chunk, chunk_id, *item_id);
			}
		}

		// Only evaluated bindings are sequenced, in import order, so a
		// dependency's top-level assignment always runs before a dependent
		// reads it.
		for id in self.modules().descs.order() {
			let chunk_id = self.modules().descs.chunk(*id);
			self.scope = self.modules().scope(chunk_id);
			let chunk = self.rt.pkgs.desc(self.pkg_id).chunks().get(chunk_id);
			for item_id in &chunk.top {
				match self.eval_module_expr(chunk, *item_id) {
					Ok(()) => {}
					Err(Signal::Return(_)) => break,
					Err(Signal::Break(_)) => panic!(),
					Err(Signal::Raise(raise)) => return Err(raise),
				}
			}
		}
		Ok(())
	}

	fn bind_module_item(&mut self, chunk: &Chunk, chunk_id: ChunkId, item_id: ModuleItemId) {
		match chunk.get_module_item(item_id) {
			ModuleItem::Module(_) => {}
			ModuleItem::Import(_) => {}
			ModuleItem::Export(_) => {}
			ModuleItem::Type(type_) => {
				let id = self.types().descs.get_type_by_item(chunk_id, item_id);
				let val = Val::Obj(self.types().user(id).val.clone());
				self.scope.borrow_mut().locals.insert(type_.name, val);
			}
			// An extern type's canonical value is the native type's, and its
			// members were merged into that type rather than described here, so
			// the name is all there is to resolve it by.
			ModuleItem::Extern(extern_) => {
				let id = self.types().descs.get_type_by_item(chunk_id, item_id);
				let Type::Native(desc) = self.types().descs.get_type(id) else {
					panic!()
				};
				let val = Val::Obj(self.rt.natives.val(desc.provider));
				self.scope.borrow_mut().locals.insert(extern_.name, val);
			}
			ModuleItem::Proto(proto) => {
				let id = self
					.rt
					.pkgs
					.desc(self.pkg_id)
					.protos()
					.get_proto_by_item(chunk_id, item_id);
				let val = Val::Obj(Rc::new(RefCell::new(Obj::Proto(self.pkg_id, id))));
				self.scope.borrow_mut().locals.insert(proto.name, val);
			}
			ModuleItem::Def(def_id) => {
				let def = chunk.get_def(*def_id);
				let obj = Obj::Proc(Proc {
					name: def.name,
					params: def.params.to_vec(),
					body: def.body,
					pkg: self.pkg_id,
					chunk: chunk_id,
					scope: self.scope.clone(),
				});
				let val = Val::Obj(Rc::new(RefCell::new(obj)));
				self.scope.borrow_mut().locals.insert(def.name, val);
			}
			ModuleItem::Expr(_) => {}
		}
	}

	fn eval_module_expr(&mut self, chunk: &Chunk, item_id: ModuleItemId) -> Result<(), Signal> {
		match chunk.get_module_item(item_id) {
			ModuleItem::Module(_) => Ok(()),
			ModuleItem::Import(_) => Ok(()),
			ModuleItem::Export(_) => Ok(()),
			ModuleItem::Type(_) => Ok(()),
			ModuleItem::Extern(_) => Ok(()),
			ModuleItem::Proto(_) => Ok(()),
			ModuleItem::Def(_) => Ok(()),
			ModuleItem::Expr(expr_id) => {
				self.eval_expr(chunk, *expr_id)?;
				Ok(())
			}
		}
	}

	fn eval_expr(&mut self, chunk: &Chunk, expr_id: ExprId) -> Result<Val, Signal> {
		match chunk.get_expr(expr_id) {
			Expr::Each(each) => match self.eval_expr(chunk, each.iter)? {
				Val::Str(str) => {
					for char in str.text.chars() {
						let scope = Rc::new(RefCell::new(Scope {
							locals: FxHashMap::default(),
							outer: Some(self.scope.clone()),
							tier: Tier::Local,
						}));
						scope
							.borrow_mut()
							.locals
							.insert(each.item, Val::Char(Char(char)));
						match self.eval_block(chunk, scope, each.body) {
							Ok(_) => {}
							Err(Signal::Break(val)) => return Ok(val),
							Err(signal) => return Err(signal),
						}
					}
					Ok(Val::Nil(Nil))
				}
				Val::Obj(obj) => match &*obj.borrow() {
					Obj::List(list) => {
						for item in &list.items {
							let scope = Rc::new(RefCell::new(Scope {
								locals: FxHashMap::default(),
								outer: Some(self.scope.clone()),
								tier: Tier::Local,
							}));
							scope.borrow_mut().locals.insert(each.item, item.clone());
							match self.eval_block(chunk, scope, each.body) {
								Ok(_) => {}
								Err(Signal::Break(val)) => return Ok(val),
								Err(signal) => return Err(signal),
							}
						}
						Ok(Val::Nil(Nil))
					}
					Obj::Dict(dict) => {
						for (key, _) in &dict.pairs {
							let scope = Rc::new(RefCell::new(Scope {
								locals: FxHashMap::default(),
								outer: Some(self.scope.clone()),
								tier: Tier::Local,
							}));
							scope.borrow_mut().locals.insert(each.item, key.clone());
							match self.eval_block(chunk, scope, each.body) {
								Ok(_) => {}
								Err(Signal::Break(val)) => return Ok(val),
								Err(signal) => return Err(signal),
							}
						}
						Ok(Val::Nil(Nil))
					}
					obj => {
						let loc = self
							.rt
							.pkgs
							.desc(self.pkg_id)
							.loc(chunk.get_expr_span(expr_id));
						Err(Signal::raise(
							Raised::Native(Error::ProtocolError(ProtocolError::NotIterable(
								obj.type_id(),
							))),
							vec![(String::new(), loc)],
						))
					}
				},
				val => {
					let loc = self
						.rt
						.pkgs
						.desc(self.pkg_id)
						.loc(chunk.get_expr_span(expr_id));
					Err(Signal::raise(
						Raised::Native(Error::ProtocolError(ProtocolError::NotIterable(
							val.type_id(),
						))),
						vec![(String::new(), loc)],
					))
				}
			},
			Expr::Loop(loop_) => loop {
				let scope = Rc::new(RefCell::new(Scope {
					locals: FxHashMap::default(),
					outer: Some(self.scope.clone()),
					tier: Tier::Local,
				}));
				match self.eval_block(chunk, scope, loop_.body) {
					Ok(_) => {}
					Err(Signal::Break(val)) => return Ok(val),
					Err(signal) => return Err(signal),
				}
			},
			Expr::When(when) => {
				let cond = self.eval_expr(chunk, when.cond)?.is_truthy();
				let scope = Rc::new(RefCell::new(Scope {
					locals: FxHashMap::default(),
					outer: Some(self.scope.clone()),
					tier: Tier::Local,
				}));
				if cond {
					self.eval_block(chunk, scope, when.then_branch)
				} else if let Some(else_branch) = when.else_branch {
					self.eval_block(chunk, scope, else_branch)
				} else {
					Ok(Val::Nil(Nil))
				}
			}
			Expr::Match(match_) => {
				let scrutinee_type_id = self.eval_expr(chunk, match_.scrutinee)?.type_id();
				for arm in &match_.arms {
					let arm_type_id = self.resolve_case_arm(chunk, arm)?;
					if arm_type_id == scrutinee_type_id {
						let scope = Rc::new(RefCell::new(Scope {
							locals: FxHashMap::default(),
							outer: Some(self.scope.clone()),
							tier: Tier::Local,
						}));
						return self.eval_block(chunk, scope, arm.body);
					}
				}
				if let Some(else_branch) = match_.else_branch {
					let scope = Rc::new(RefCell::new(Scope {
						locals: FxHashMap::default(),
						outer: Some(self.scope.clone()),
						tier: Tier::Local,
					}));
					self.eval_block(chunk, scope, else_branch)
				} else {
					Ok(Val::Nil(Nil))
				}
			}
			Expr::Do(do_) => {
				let scope = Rc::new(RefCell::new(Scope {
					locals: FxHashMap::default(),
					outer: Some(self.scope.clone()),
					tier: Tier::Local,
				}));
				let (raised, trace) = match self.eval_block(chunk, scope, do_.body) {
					Err(Signal::Raise(Raise(raised, trace))) => (raised, trace),
					other => return other,
				};
				let raised_type_id = match &raised {
					Raised::Native(err) => self.rt.errors.variant(self.syms, self.rt, err),
					Raised::Val(val) => val.type_id(),
				};
				for arm in &do_.arms {
					let arm_type_id = self.resolve_case_arm(chunk, arm)?;
					if arm_type_id == raised_type_id {
						let scope = Rc::new(RefCell::new(Scope {
							locals: FxHashMap::default(),
							outer: Some(self.scope.clone()),
							tier: Tier::Local,
						}));
						if let Some(binding) = do_.binding {
							let bound = match &raised {
								Raised::Native(err) => {
									self.rt.errors.reify(self.syms, self.rt, err)
								}
								Raised::Val(val) => val.clone(),
							};
							scope.borrow_mut().locals.insert(binding, bound);
						}
						return self.eval_block(chunk, scope, arm.body);
					}
				}
				if let Some(else_branch) = do_.else_branch {
					let scope = Rc::new(RefCell::new(Scope {
						locals: FxHashMap::default(),
						outer: Some(self.scope.clone()),
						tier: Tier::Local,
					}));
					self.eval_block(chunk, scope, else_branch)
				} else {
					Err(Signal::Raise(Raise(raised, trace)))
				}
			}
			Expr::Return(return_) => {
				let val = if let Some(val_expr_id) = return_.val {
					self.eval_expr(chunk, val_expr_id)?
				} else {
					Val::Nil(Nil)
				};
				Err(Signal::Return(val))
			}
			Expr::Break(break_) => {
				let val = if let Some(val_expr_id) = break_.val {
					self.eval_expr(chunk, val_expr_id)?
				} else {
					Val::Nil(Nil)
				};
				Err(Signal::Break(val))
			}
			Expr::Raise(raise) => {
				let val = self.eval_expr(chunk, raise.val)?;
				let loc = self
					.rt
					.pkgs
					.desc(self.pkg_id)
					.loc(chunk.get_expr_span(expr_id));
				Err(Signal::raise(Raised::Val(val), vec![(String::new(), loc)]))
			}
			Expr::Call(call) => {
				let span = chunk.get_expr_span(expr_id);
				let mut args = Vec::with_capacity(call.args.len());
				for arg in &call.args {
					let val = self.eval_expr(chunk, arg.val)?;
					args.push((arg.name, val));
				}
				let callee = self.eval_expr_raw(chunk, call.callee)?;
				match callee {
					Val::Obj(obj) => match &*obj.borrow() {
						Obj::Proc(proc) => self.eval_proc_call(span, proc, None, args),
						Obj::Type(type_id) => match type_id {
							TypeId::User(type_pkg, type_id) => {
								let (type_pkg, type_id) = (*type_pkg, *type_id);
								// The description outlives the interpreter, so a type's
								// fields are read where they were written rather than
								// copied onto every construction.
								let Type::User(desc) =
									self.rt.pkgs.get(type_pkg).types.descs.get_type(type_id)
								else {
									panic!()
								};
								// A type with variants cannot itself be constructed.
								if let Some(variants) = &desc.variants
									&& !variants.is_empty()
								{
									let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
									return Err(Signal::raise(
										Raised::Native(Error::TypeError(
											TypeError::NotConstructible(TypeId::User(
												type_pkg, type_id,
											)),
										)),
										vec![(String::new(), loc)],
									));
								}
								let ctor_fields = &desc.ctor_fields;
								let body_fields = &desc.body_fields;
								let type_chunk_id = desc.chunk;
								let type_scope =
									self.rt.pkgs.get(type_pkg).modules.scope(type_chunk_id);

								let slots = self.slot_args(span, ctor_fields, args)?;
								let scope = Rc::new(RefCell::new(Scope {
									locals: FxHashMap::default(),
									outer: Some(type_scope.clone()),
									tier: Tier::Local,
								}));
								// Every initializer below is written in the type's file, so
								// they are evaluated as the package that declared it.
								let saved_pkg_id = self.pkg_id;
								self.pkg_id = type_pkg;
								// The instance does not exist yet, so a ctor default sees the
								// fields to its left and the module, never self.
								let outer_receiver = self.receiver.take();
								let bound =
									self.bind_args(type_chunk_id, ctor_fields, slots, &scope);
								self.receiver = outer_receiver;
								if let Err(signal) = bound {
									self.pkg_id = saved_pkg_id;
									return Err(signal);
								}
								// The scope was just used for initialization, nothing else holds
								// a reference to it.
								let fields = Rc::try_unwrap(scope).unwrap().into_inner().locals;

								let type_chunk =
									self.rt.pkgs.desc(type_pkg).chunks().get(type_chunk_id);
								let receiver = Rc::new(RefCell::new(Obj::Instance(Instance {
									pkg: type_pkg,
									type_: type_id,
									fields,
								})));

								let saved_scope = self.scope.clone();
								let saved_receiver = self.receiver.clone();
								self.receiver = Some(Val::Obj(receiver.clone()));
								for (name, field) in body_fields {
									self.scope = Rc::new(RefCell::new(Scope {
										locals: FxHashMap::default(),
										outer: Some(type_scope.clone()),
										tier: Tier::Local,
									}));
									let val = match self.eval_expr(type_chunk, field.init) {
										Ok(val) => val,
										Err(signal) => {
											self.scope = saved_scope;
											self.receiver = saved_receiver;
											self.pkg_id = saved_pkg_id;
											return Err(signal);
										}
									};
									Place::Member(Member::User(receiver.clone(), *name))
										.set(&self.rt.pkgs, val)
										.unwrap();
								}
								self.scope = saved_scope;
								self.receiver = saved_receiver;
								self.pkg_id = saved_pkg_id;

								Ok(Val::Obj(receiver))
							}
							TypeId::Native(type_id) => {
								let type_ = self.rt.natives.get(*type_id);
								match type_.new {
									Some(new) => {
										// A native constructor defines no
										// params by definition, so route
										// through the usual machinery.
										self.bind_native_args(span, &[], args)?;
										Ok(new())
									}
									None => {
										let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
										Err(Signal::raise(
											Raised::Native(Error::TypeError(
												TypeError::NotConstructible(TypeId::Native(
													*type_id,
												)),
											)),
											vec![(String::new(), loc)],
										))
									}
								}
							}
						},
						Obj::Method(method) => match method {
							Method::User(receiver, proc) => {
								let name = proc.borrow().name;
								self.call_member(span, receiver.clone(), name, args)
							}
							Method::Native(receiver, name, _) => {
								self.call_member(span, receiver.clone(), *name, args)
							}
						},
						obj => {
							let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
							Err(Signal::raise(
								Raised::Native(Error::TypeError(TypeError::NotCallable(
									obj.type_id(),
								))),
								vec![(String::new(), loc)],
							))
						}
					},
					val => {
						let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
						Err(Signal::raise(
							Raised::Native(Error::TypeError(TypeError::NotCallable(val.type_id()))),
							vec![(String::new(), loc)],
						))
					}
				}
			}
			Expr::Member(member) => {
				let val = self.eval_member_raw(chunk, expr_id, member.receiver, member.name)?;
				self.invoke_or_return(chunk, expr_id, val)
			}
			Expr::Access(access) => {
				let receiver = self.eval_expr(chunk, access.receiver)?;
				let key = self.eval_expr(chunk, access.key)?;
				let span = chunk.get_expr_span(expr_id);
				if self
					.rt
					.conforms(receiver.type_id(), self.rt.behaviors.access.proto)
				{
					let member = self.rt.behaviors.access.access;
					self.call_member(span, receiver, member, vec![(None, key)])
				} else {
					let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
					Err(Signal::raise(
						Raised::Native(Error::ProtocolError(ProtocolError::NotAccessible(
							receiver.type_id(),
						))),
						vec![(String::new(), loc)],
					))
				}
			}
			Expr::Mention(mention) => {
				let val = self.eval_expr_raw(chunk, mention.val)?;
				self.expect_invocable(chunk, expr_id, val)
			}
			Expr::Assign(assign) => {
				let val = self.eval_expr(chunk, assign.val)?;
				match &assign.place {
					syn::nodes::Place::Name(name) => {
						let sym = name.sym;
						let place = {
							let place = self.resolve_name(sym);
							if let Place::Local(local) = &place
								&& local.tier == Tier::Prelude
							{
								Place::Local(Local::new(self.scope.clone(), sym))
							} else {
								place
							}
						};
						if let Err(()) = place.set(&self.rt.pkgs, val.clone()) {
							let namespace = match &place {
								Place::Member(Member::Module(pkg, id, name)) => {
									let pkg_modules = &self.rt.pkgs.get(*pkg).modules;
									let module = match pkg_modules.descs.member(*id, *name) {
										Some(modules::Member::Child(child)) => child,
										_ => *id,
									};
									Val::Obj(pkg_modules.obj(module))
								}
								_ => self.receiver.as_ref().unwrap().clone(),
							};
							let loc = self
								.rt
								.pkgs
								.desc(self.pkg_id)
								.loc(chunk.get_expr_span(expr_id));
							return Err(Signal::raise(
								Raised::Native(Error::MemberError(MemberError::ReadOnly(
									namespace_name(self.syms, self.rt, &namespace),
									sym,
								))),
								vec![(String::new(), loc)],
							));
						}
						Ok(val)
					}
					syn::nodes::Place::Member(member) => {
						let receiver = self.eval_expr(chunk, member.receiver)?;
						if let Some(m) = self.member(&receiver, member.name) {
							if let Err(()) = Place::Member(m).set(&self.rt.pkgs, val.clone()) {
								let loc = self
									.rt
									.pkgs
									.desc(self.pkg_id)
									.loc(chunk.get_expr_span(expr_id));
								return Err(Signal::raise(
									Raised::Native(Error::MemberError(MemberError::ReadOnly(
										namespace_name(self.syms, self.rt, &receiver),
										member.name,
									))),
									vec![(String::new(), loc)],
								));
							}
							Ok(val)
						} else {
							let loc = self
								.rt
								.pkgs
								.desc(self.pkg_id)
								.loc(chunk.get_expr_span(expr_id));
							Err(Signal::raise(
								Raised::Native(Error::MemberError(MemberError::Missing(
									namespace_name(self.syms, self.rt, &receiver),
									member.name,
								))),
								vec![(String::new(), loc)],
							))
						}
					}
					syn::nodes::Place::Access(access) => {
						let receiver = self.eval_expr(chunk, access.receiver)?;
						let key = self.eval_expr(chunk, access.key)?;
						let span = chunk.get_expr_span(expr_id);
						if self
							.rt
							.conforms(receiver.type_id(), self.rt.behaviors.access.proto)
						{
							let member = self.rt.behaviors.access.store;
							self.call_member(
								span,
								receiver,
								member,
								vec![(None, key), (None, val.clone())],
							)?;
							Ok(val)
						} else {
							let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
							Err(Signal::raise(
								Raised::Native(Error::ProtocolError(ProtocolError::NotAccessible(
									receiver.type_id(),
								))),
								vec![(String::new(), loc)],
							))
						}
					}
				}
			}
			Expr::Binary(binary) => match &binary.op {
				BinaryOp::Or => {
					let lhs = self.eval_expr(chunk, binary.lhs)?;
					if lhs.is_truthy() {
						Ok(lhs)
					} else {
						self.eval_expr(chunk, binary.rhs)
					}
				}
				BinaryOp::And => {
					let lhs = self.eval_expr(chunk, binary.lhs)?;
					if !lhs.is_truthy() {
						Ok(lhs)
					} else {
						self.eval_expr(chunk, binary.rhs)
					}
				}
				BinaryOp::Eq
				| BinaryOp::NotEq
				| BinaryOp::Lt
				| BinaryOp::Gt
				| BinaryOp::LtEq
				| BinaryOp::GtEq
				| BinaryOp::Append
				| BinaryOp::Add
				| BinaryOp::Sub
				| BinaryOp::Mul
				| BinaryOp::Div => {
					let lhs = self.eval_expr(chunk, binary.lhs)?;
					let rhs = self.eval_expr(chunk, binary.rhs)?;
					match &binary.op {
						BinaryOp::Eq | BinaryOp::NotEq => {
							let span = chunk.get_expr_span(expr_id);
							let equal = self.equal(span, &lhs, &rhs)?;
							Ok(Val::Bool(Bool(match &binary.op {
								BinaryOp::Eq => equal,
								BinaryOp::NotEq => !equal,
								_ => panic!(),
							})))
						}
						BinaryOp::Lt | BinaryOp::Gt | BinaryOp::LtEq | BinaryOp::GtEq => {
							match (lhs, rhs) {
								(Val::Num(lhs), Val::Num(rhs)) => {
									Ok(Val::Bool(Bool(match &binary.op {
										BinaryOp::Lt => lhs.0 < rhs.0,
										BinaryOp::Gt => lhs.0 > rhs.0,
										BinaryOp::LtEq => lhs.0 <= rhs.0,
										BinaryOp::GtEq => lhs.0 >= rhs.0,
										_ => panic!(),
									})))
								}
								(Val::Str(lhs), Val::Str(rhs)) => {
									Ok(Val::Bool(Bool(match &binary.op {
										BinaryOp::Lt => lhs.text < rhs.text,
										BinaryOp::Gt => lhs.text > rhs.text,
										BinaryOp::LtEq => lhs.text <= rhs.text,
										BinaryOp::GtEq => lhs.text >= rhs.text,
										_ => panic!(),
									})))
								}
								(Val::Char(lhs), Val::Char(rhs)) => {
									Ok(Val::Bool(Bool(match &binary.op {
										BinaryOp::Lt => lhs.0 < rhs.0,
										BinaryOp::Gt => lhs.0 > rhs.0,
										BinaryOp::LtEq => lhs.0 <= rhs.0,
										BinaryOp::GtEq => lhs.0 >= rhs.0,
										_ => panic!(),
									})))
								}
								(lhs, rhs) => {
									let span = chunk.get_expr_span(expr_id);
									if lhs.type_id() == rhs.type_id()
										&& self
											.rt
											.conforms(lhs.type_id(), self.rt.behaviors.order.proto)
									{
										let member = self.rt.behaviors.order.method;
										let sign = self.call_member(
											span,
											lhs.clone(),
											member,
											vec![(None, rhs.clone())],
										)?;
										// A conforming type promised a 'Num' sign. Anything
										// else is a in that type's implementation of 'Order'.
										let Val::Num(Num(sign)) = sign else {
											let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
											return Err(Signal::raise(
												Raised::Native(Error::ProtocolError(
													ProtocolError::NotOrderable(lhs.type_id()),
												)),
												vec![(String::new(), loc)],
											));
										};
										Ok(Val::Bool(Bool(match &binary.op {
											BinaryOp::Lt => sign < 0.0,
											BinaryOp::Gt => sign > 0.0,
											BinaryOp::LtEq => sign <= 0.0,
											BinaryOp::GtEq => sign >= 0.0,
											_ => panic!(),
										})))
									} else {
										let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
										let val = match (&lhs, &rhs) {
											(Val::Num(_), _) => rhs,
											(_, Val::Num(_)) => lhs,
											_ => lhs,
										};
										Err(Signal::raise(
											Raised::Native(Error::ProtocolError(
												ProtocolError::NotOrderable(val.type_id()),
											)),
											vec![(String::new(), loc)],
										))
									}
								}
							}
						}
						BinaryOp::Append => {
							if self
								.rt
								.conforms(lhs.type_id(), self.rt.behaviors.append.proto)
							{
								let span = chunk.get_expr_span(expr_id);
								let member = self.rt.behaviors.append.method;
								self.call_member(span, lhs.clone(), member, vec![(None, rhs)])
							} else {
								let loc = self
									.rt
									.pkgs
									.desc(self.pkg_id)
									.loc(chunk.get_expr_span(expr_id));
								Err(Signal::raise(
									Raised::Native(Error::ProtocolError(
										ProtocolError::NotAppendable(lhs.type_id()),
									)),
									vec![(String::new(), loc)],
								))
							}
						}
						BinaryOp::Add => match (lhs, rhs) {
							(Val::Num(lhs), Val::Num(rhs)) => Ok(Val::Num(Num(lhs.0 + rhs.0))),
							(Val::Str(lhs), Val::Str(rhs)) => {
								let mut text =
									String::with_capacity(lhs.text.len() + rhs.text.len());
								text.push_str(&lhs.text);
								text.push_str(&rhs.text);
								Ok(Val::Str(Rc::new(Str {
									text: Box::from(text.as_str()),
									size: Cell::new(None),
								})))
							}
							(Val::Str(_), rhs) => {
								let loc = self
									.rt
									.pkgs
									.desc(self.pkg_id)
									.loc(chunk.get_expr_span(expr_id));
								Err(Signal::raise(
									Raised::Native(Error::TypeError(TypeError::ConcatNonStr(
										rhs.type_id(),
									))),
									vec![(String::new(), loc)],
								))
							}
							(_, rhs) => {
								let loc = self
									.rt
									.pkgs
									.desc(self.pkg_id)
									.loc(chunk.get_expr_span(expr_id));
								Err(Signal::raise(
									Raised::Native(Error::TypeError(TypeError::ArithNonNum(
										rhs.type_id(),
									))),
									vec![(String::new(), loc)],
								))
							}
						},
						BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => match (lhs, rhs) {
							(Val::Num(lhs), Val::Num(rhs)) => Ok(Val::Num(Num(match &binary.op {
								BinaryOp::Sub => lhs.0 - rhs.0,
								BinaryOp::Mul => lhs.0 * rhs.0,
								BinaryOp::Div => lhs.0 / rhs.0,
								_ => panic!(),
							}))),
							(lhs, rhs) => {
								let loc = self
									.rt
									.pkgs
									.desc(self.pkg_id)
									.loc(chunk.get_expr_span(expr_id));
								let val = match (&lhs, &rhs) {
									(Val::Num(_), _) => rhs,
									(_, Val::Num(_)) => lhs,
									_ => lhs,
								};
								Err(Signal::raise(
									Raised::Native(Error::TypeError(TypeError::ArithNonNum(
										val.type_id(),
									))),
									vec![(String::new(), loc)],
								))
							}
						},
						BinaryOp::Or | BinaryOp::And => panic!(),
					}
				}
			},
			Expr::Unary(unary) => match &unary.op {
				UnaryOp::Not => {
					let val = self.eval_expr(chunk, unary.val)?;
					Ok(Val::Bool(Bool(!val.is_truthy())))
				}
				UnaryOp::Neg => match self.eval_expr(chunk, unary.val)? {
					Val::Num(num) => Ok(Val::Num(Num(-num.0))),
					val => {
						let loc = self
							.rt
							.pkgs
							.desc(self.pkg_id)
							.loc(chunk.get_expr_span(expr_id));
						Err(Signal::raise(
							Raised::Native(Error::TypeError(TypeError::ArithNonNum(val.type_id()))),
							vec![(String::new(), loc)],
						))
					}
				},
			},
			Expr::Self_ => match &self.receiver {
				Some(receiver) => Ok(receiver.clone()),
				None => todo!(),
			},
			Expr::Name(name) => {
				let val = self.eval_name_raw(chunk, expr_id, name.sym)?;
				self.invoke_or_return(chunk, expr_id, val)
			}
			Expr::Builtin(builtin) => match builtin {
				Builtin::Print { val: val_id } => {
					let val = self.eval_expr(chunk, *val_id)?;
					let span = chunk.get_expr_span(expr_id);
					println!("{}", self.print(span, &val)?);
					Ok(val)
				}
				Builtin::Type { val: val_id } => {
					let val = self.eval_expr(chunk, *val_id)?;
					let type_id = val.type_id();
					match type_id {
						TypeId::User(pkg, id) => {
							Ok(Val::Obj(self.rt.pkgs.get(pkg).types.user(id).val.clone()))
						}
						TypeId::Native(id) => Ok(Val::Obj(self.rt.natives.val(id))),
					}
				}
			},
			Expr::Lit(lit) => Ok(match lit {
				Lit::Str(str) => Val::Str(Rc::new(Str {
					text: Box::from(str.as_str()),
					size: Cell::new(None),
				})),
				Lit::Char(char) => Val::Char(Char(*char)),
				Lit::Num(num) => Val::Num(Num(*num)),
				Lit::Bool(bool) => Val::Bool(Bool(*bool)),
				Lit::List(item_ids) => {
					let mut items = Vec::with_capacity(item_ids.len());
					for item_id in item_ids {
						let item = self.eval_expr(chunk, *item_id)?;
						items.push(item);
					}
					Val::Obj(Rc::new(RefCell::new(Obj::List(List { items }))))
				}
				Lit::Dict(pair_ids) => {
					let mut dict = Dict::with_capacity(pair_ids.len());
					for (key_id, val_id) in pair_ids {
						let span = chunk.get_expr_span(*key_id);
						let key = self.eval_expr(chunk, *key_id)?;
						let hash = self.hash(span, &key)?;
						let val = self.eval_expr(chunk, *val_id)?;
						let mut found = false;
						for (index, candidate) in dict.candidates(hash) {
							if self.equal(span, &candidate, &key)? {
								dict.pairs[index].1 = val.clone();
								found = true;
								break;
							}
						}
						if !found {
							dict.insert(hash, key, val);
						}
					}
					Val::Obj(Rc::new(RefCell::new(Obj::Dict(dict))))
				}
				Lit::Nil => Val::Nil(Nil),
			}),
		}
	}

	// Resolve a case arm's path expression to the TypeId it names.
	fn resolve_case_arm(&mut self, chunk: &Chunk, arm: &Arm) -> Result<TypeId, Signal> {
		let path_val = self.eval_expr(chunk, arm.path)?;
		let arm_type_id = match &path_val {
			Val::Obj(obj) => match &*obj.borrow() {
				Obj::Type(id) => Some(*id),
				_ => None,
			},
			_ => None,
		};
		let Some(arm_type_id) = arm_type_id else {
			let loc = self
				.rt
				.pkgs
				.desc(self.pkg_id)
				.loc(chunk.get_expr_span(arm.path));
			return Err(Signal::raise(
				Raised::Native(Error::TypeError(TypeError::CaseNonType(path_val.type_id()))),
				vec![(String::new(), loc)],
			));
		};
		Ok(arm_type_id)
	}

	fn eval_expr_raw(&mut self, chunk: &Chunk, expr_id: ExprId) -> Result<Val, Signal> {
		match chunk.get_expr(expr_id) {
			Expr::Name(name) => self.eval_name_raw(chunk, expr_id, name.sym),
			Expr::Member(member) => {
				self.eval_member_raw(chunk, expr_id, member.receiver, member.name)
			}
			_ => self.eval_expr(chunk, expr_id),
		}
	}

	fn member(&self, val: &Val, name: Sym) -> Option<Member> {
		if let Val::Obj(obj) = val
			&& let Obj::Module(pkg, id) = &*obj.borrow()
		{
			let pkg_modules = &self.rt.pkgs.get(*pkg).modules;
			if pkg_modules.descs.member(*id, name).is_some() {
				return Some(Member::Module(*pkg, *id, name));
			}
		}
		self.rt.member(val, name)
	}

	fn hash(&mut self, span: Span, val: &Val) -> Result<u64, Signal> {
		let saved_hash_nodes = self.hash_nodes.replace(MAX_HASH_NODES);
		let result = self.hash_bounded(span, val);
		self.hash_nodes = saved_hash_nodes;
		result
	}

	fn hash_bounded(&mut self, span: Span, val: &Val) -> Result<u64, Signal> {
		let hash_nodes = self.hash_nodes.as_mut().unwrap();
		if *hash_nodes == 0 {
			return Ok(0);
		}
		if !self
			.rt
			.conforms(val.type_id(), self.rt.behaviors.hash.proto)
		{
			let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
			return Err(Signal::raise(
				Raised::Native(Error::ProtocolError(ProtocolError::NotHashable(
					val.type_id(),
				))),
				vec![(String::new(), loc)],
			));
		}

		*hash_nodes -= 1;

		let name = self.rt.behaviors.hash.method;
		let result = match self.rt.method(val.type_id(), name).unwrap() {
			MethodImpl::Native(method) => self.call_native(span, method, val, Vec::new()),
			MethodImpl::User(proc) => {
				self.eval_proc_call(span, &proc.borrow(), Some(val.clone()), Vec::new())
			}
		}?;
		let Val::Num(hash) = result else {
			let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
			return Err(Signal::raise(
				Raised::Native(Error::ProtocolError(ProtocolError::NotHashable(
					val.type_id(),
				))),
				vec![(String::new(), loc)],
			));
		};
		Ok(hash.0.to_bits())
	}

	pub(crate) fn hash_val(&mut self, val: &Val) -> Result<u64, Raise> {
		let span = self.call_span.unwrap();
		let top_level = self.hash_nodes.is_none();
		if top_level {
			self.hash_nodes = Some(MAX_HASH_NODES);
		}
		let result = match self.hash_bounded(span, val) {
			Ok(hash) => Ok(hash),
			Err(Signal::Raise(raise)) => Err(raise),
			Err(Signal::Return(_) | Signal::Break(_)) => panic!(),
		};
		if top_level {
			self.hash_nodes = None;
		}
		result
	}

	pub(crate) fn start_derived_hash(&mut self) -> bool {
		if self.hash_nodes.is_some() {
			return false;
		}
		self.hash_nodes = Some(MAX_HASH_NODES - 1);
		true
	}

	pub(crate) fn finish_derived_hash(&mut self, top_level: bool) {
		if top_level {
			self.hash_nodes = None;
		}
	}

	pub(crate) fn hash_exhausted(&self) -> bool {
		self.hash_nodes == Some(0)
	}

	fn equal(&mut self, span: Span, lhs: &Val, rhs: &Val) -> Result<bool, Signal> {
		let pair = match (lhs, rhs) {
			(Val::Obj(lhs), Val::Obj(rhs)) => Some((Rc::as_ptr(lhs), Rc::as_ptr(rhs))),
			_ => None,
		};
		if let Some(pair) = pair {
			if self.equating.contains(&pair) {
				return Ok(true);
			}
			self.equating.push(pair);
		}

		let name = self.rt.behaviors.equal.method;
		let result = match self.rt.method(lhs.type_id(), name).unwrap() {
			MethodImpl::Native(method) => self.call_native(span, method, lhs, vec![rhs.clone()]),
			MethodImpl::User(proc) => self.eval_proc_call(
				span,
				&proc.borrow(),
				Some(lhs.clone()),
				vec![(None, rhs.clone())],
			),
		};

		if pair.is_some() {
			self.equating.pop();
		}
		result.map(|val| val.is_truthy())
	}

	pub(crate) fn equal_vals(&mut self, lhs: &Val, rhs: &Val) -> Result<bool, Raise> {
		let span = self.call_span.unwrap();
		match self.equal(span, lhs, rhs) {
			Ok(equal) => Ok(equal),
			Err(Signal::Raise(raise)) => Err(raise),
			Err(Signal::Return(_) | Signal::Break(_)) => panic!(),
		}
	}

	fn print(&mut self, span: Span, val: &Val) -> Result<String, Signal> {
		let method = if self
			.rt
			.conforms(val.type_id(), self.rt.behaviors.display.proto)
		{
			self.rt.behaviors.display.method
		} else {
			self.rt.behaviors.inspect.method
		};
		self.print_via(span, val, method)
	}

	pub(crate) fn inspect(&mut self, val: &Val) -> Result<String, Raise> {
		let span = self.call_span.unwrap();
		let method = self.rt.behaviors.inspect.method;
		match self.print_via(span, val, method) {
			Ok(text) => Ok(text),
			Err(Signal::Raise(raise)) => Err(raise),
			Err(Signal::Return(_) | Signal::Break(_)) => panic!(),
		}
	}

	fn print_via(&mut self, span: Span, val: &Val, method: Sym) -> Result<String, Signal> {
		if self.print_depth == MAX_PRINT_DEPTH {
			return Ok(String::from("..."));
		}
		let ptr = match val {
			Val::Obj(obj) => Some(Rc::as_ptr(obj)),
			_ => None,
		};
		if let Some(ptr) = ptr
			&& !self.printing.insert(ptr)
		{
			return Ok(String::from("..."));
		}
		self.print_depth += 1;
		let result = self.call_member(span, val.clone(), method, vec![]);
		self.print_depth -= 1;
		if let Some(ptr) = ptr {
			self.printing.remove(&ptr);
		}
		// A conforming type promised a 'Str'. Anything else is a bug in that
		// type's implementation of the protocol.
		let Val::Str(str) = result? else {
			let err = if method == self.rt.behaviors.display.method {
				ProtocolError::NotDisplayable(val.type_id())
			} else {
				ProtocolError::NotInspectable(val.type_id())
			};
			let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
			return Err(Signal::raise(
				Raised::Native(Error::ProtocolError(err)),
				vec![(String::new(), loc)],
			));
		};
		Ok(str.text.to_string())
	}

	// Resolve and call a member by name on a receiver. The caller vouches for
	// the member's existence.
	fn call_member(
		&mut self,
		span: Span,
		receiver: Val,
		name: Sym,
		args: Vec<(Option<Sym>, Val)>,
	) -> Result<Val, Signal> {
		let member = self.rt.member(&receiver, name).unwrap();
		let val = match Place::Member(member).get(self.rt) {
			Ok(val) => val,
			Err(err) => {
				let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
				return Err(Signal::raise(
					Raised::Native(err),
					vec![(String::new(), loc)],
				));
			}
		};
		let Val::Obj(obj) = val else { panic!() };
		match &*obj.borrow() {
			Obj::Method(Method::User(receiver, proc)) => {
				self.eval_proc_call(span, &proc.borrow(), Some(receiver.clone()), args)
			}
			Obj::Method(Method::Native(receiver, _name, method)) => {
				let method = *method;
				let args = self.bind_native_args(span, method.params, args)?;
				self.call_native(span, method, receiver, args)
			}
			_ => panic!(),
		}
	}

	fn call_native(
		&mut self,
		span: Span,
		method: NativeMethod,
		receiver: &Val,
		args: Vec<Val>,
	) -> Result<Val, Signal> {
		let saved_call_span = self.call_span.replace(span);
		let result = (method.call)(self, receiver, args);
		self.call_span = saved_call_span;
		result.map_err(Signal::Raise)
	}

	fn resolve_name(&self, name: Sym) -> Place {
		let local = Scope::local(&self.scope, name);
		if local.is_bound() && local.tier == Tier::Local {
			Place::Local(local)
		} else if let Some(receiver) = &self.receiver
			&& let Some(member) = self.member(receiver, name)
		{
			Place::Member(member)
		} else if local.is_bound() {
			Place::Local(local)
		} else {
			let module_id = Scope::module(&self.scope);
			match self
				.modules()
				.descs
				.binding(self.rt.pkgs.descs(), module_id, name)
			{
				Some(modules::Binding::Own(_, _)) => Place::Local(local),
				Some(modules::Binding::Imported(pkg, owner, _)) => {
					Place::Member(Member::Module(pkg, owner, name))
				}
				Some(modules::Binding::Module(pkg, id)) => Place::Module(pkg, id),
				None => Place::Local(local),
			}
		}
	}

	fn eval_name_raw(&mut self, chunk: &Chunk, expr_id: ExprId, name: Sym) -> Result<Val, Signal> {
		let place = self.resolve_name(name);
		if !place.is_bound() {
			let loc = self
				.rt
				.pkgs
				.desc(self.pkg_id)
				.loc(chunk.get_expr_span(expr_id));
			return Err(Signal::raise(
				Raised::Native(Error::NameError(name)),
				vec![(String::new(), loc)],
			));
		}
		match place.get(self.rt) {
			Ok(val) => Ok(val),
			Err(err) => {
				let loc = self
					.rt
					.pkgs
					.desc(self.pkg_id)
					.loc(chunk.get_expr_span(expr_id));
				Err(Signal::raise(
					Raised::Native(err),
					vec![(String::new(), loc)],
				))
			}
		}
	}

	fn eval_member_raw(
		&mut self,
		chunk: &Chunk,
		expr_id: ExprId,
		val_id: ExprId,
		name: Sym,
	) -> Result<Val, Signal> {
		let val = self.eval_expr(chunk, val_id)?;

		if let Some(member) = self.member(&val, name) {
			match Place::Member(member).get(self.rt) {
				Ok(val) => Ok(val),
				Err(err) => {
					let loc = self
						.rt
						.pkgs
						.desc(self.pkg_id)
						.loc(chunk.get_expr_span(expr_id));
					Err(Signal::raise(
						Raised::Native(err),
						vec![(String::new(), loc)],
					))
				}
			}
		} else {
			let loc = self
				.rt
				.pkgs
				.desc(self.pkg_id)
				.loc(chunk.get_expr_span(expr_id));
			Err(Signal::raise(
				Raised::Native(Error::MemberError(MemberError::Missing(
					namespace_name(self.syms, self.rt, &val),
					name,
				))),
				vec![(String::new(), loc)],
			))
		}
	}

	pub(crate) fn index_of(&self, key: &Val, len: usize) -> Result<usize, Error> {
		let Val::Num(num) = key else {
			return Err(Error::TypeError(TypeError::IndexNonNum(inspect_val(
				self.syms, self.rt, key,
			))));
		};
		let num = num.0;
		if num != num.trunc() {
			return Err(Error::IndexError(IndexError::NonIntegral(num)));
		}
		if num < 0.0 || num >= len as f64 {
			return Err(Error::IndexError(IndexError::OutOfRange(num)));
		}
		Ok(num as usize)
	}

	fn expect_invocable(
		&mut self,
		chunk: &Chunk,
		expr_id: ExprId,
		val: Val,
	) -> Result<Val, Signal> {
		if let Val::Obj(obj) = &val {
			match &*obj.borrow() {
				Obj::Proc(_) | Obj::Method(_) => return Ok(val.clone()),
				_ => {}
			}
		}

		let loc = self
			.rt
			.pkgs
			.desc(self.pkg_id)
			.loc(chunk.get_expr_span(expr_id));
		Err(Signal::raise(
			Raised::Native(Error::TypeError(TypeError::NotInvocable(val.type_id()))),
			vec![(String::new(), loc)],
		))
	}

	fn invoke_or_return(
		&mut self,
		chunk: &Chunk,
		expr_id: ExprId,
		val: Val,
	) -> Result<Val, Signal> {
		let Val::Obj(obj) = &val else { return Ok(val) };
		let obj = obj.clone();
		let span = chunk.get_expr_span(expr_id);

		// Required params precede defaulted ones, so the first param alone says
		// whether this needs any arguments.
		let missing = match &*obj.borrow() {
			Obj::Proc(proc) => proc
				.params
				.first()
				.filter(|param| param.default.is_none())
				.map(|param| param.name),
			Obj::Method(Method::User(_, proc)) => proc
				.borrow()
				.params
				.first()
				.filter(|param| param.default.is_none())
				.map(|param| param.name),
			// Native types are defined in Rust, skipping the
			// required-before-default ordering check. Use find instead of
			// filter to ensure we find any required params.
			Obj::Method(Method::Native(_, _, method)) => method
				.params
				.iter()
				.find(|param| param.default.is_none())
				.map(|param| self.syms.intern(param.name)),
			_ => return Ok(val),
		};

		// Used in an invoking position but requires arguments; error.
		if let Some(name) = missing {
			let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
			return Err(Signal::raise(
				Raised::Native(Error::ArgumentError(ArgumentError::Missing(vec![name]))),
				vec![(String::new(), loc)],
			));
		}

		match &*obj.borrow() {
			Obj::Proc(proc) => self.eval_proc_call(span, proc, None, Vec::new()),
			Obj::Method(Method::User(receiver, proc)) => {
				self.eval_proc_call(span, &proc.borrow(), Some(receiver.clone()), Vec::new())
			}
			Obj::Method(Method::Native(receiver, _name, method)) => {
				let method = *method;
				self.call_native(span, method, receiver, method.defaults().unwrap())
			}
			_ => panic!(),
		}
	}

	fn bind_native_args(
		&mut self,
		span: Span,
		params: &'static [NativeParam],
		args: Vec<(Option<Sym>, Val)>,
	) -> Result<Vec<Val>, Signal> {
		let arg_count = args.len();
		let mut slots = vec![None; params.len()];
		let mut next = 0;
		for (name, val) in args {
			let index = match name {
				Some(name) => {
					let text = self.syms.resolve(name);
					match params.iter().position(|param| param.name == text) {
						Some(index) => index,
						None => {
							let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
							return Err(Signal::raise(
								Raised::Native(Error::ArgumentError(ArgumentError::Unknown(name))),
								vec![(String::new(), loc)],
							));
						}
					}
				}
				None => {
					if next == params.len() {
						let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
						return Err(Signal::raise(
							Raised::Native(Error::ArgumentError(ArgumentError::TooMany(
								arg_count,
								params.len(),
							))),
							vec![(String::new(), loc)],
						));
					}
					let index = next;
					next += 1;
					index
				}
			};
			if slots[index].is_some() {
				let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
				let name = self.syms.intern(params[index].name);
				return Err(Signal::raise(
					Raised::Native(Error::ArgumentError(ArgumentError::Duplicate(name))),
					vec![(String::new(), loc)],
				));
			}
			slots[index] = Some(val);
		}

		let missing = params
			.iter()
			.zip(&slots)
			.filter(|(param, slot)| slot.is_none() && param.default.is_none())
			.map(|(param, _)| self.syms.intern(param.name))
			.collect::<Vec<_>>();
		if !missing.is_empty() {
			let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
			return Err(Signal::raise(
				Raised::Native(Error::ArgumentError(ArgumentError::Missing(missing))),
				vec![(String::new(), loc)],
			));
		}

		Ok(params
			.iter()
			.zip(slots)
			.map(|(param, slot)| match slot {
				Some(val) => val,
				None => (param.default.unwrap())(),
			})
			.collect())
	}

	// Every failure here is the caller's, so the errors carry the call site and
	// no callee frame.
	fn slot_args(
		&self,
		span: Span,
		params: &[Param],
		args: Vec<(Option<Sym>, Val)>,
	) -> Result<Vec<Option<Val>>, Signal> {
		let arg_count = args.len();
		let mut slots = vec![None; params.len()];
		let mut next = 0;
		for (name, val) in args {
			let index = match name {
				Some(name) => match params.iter().position(|param| param.name == name) {
					Some(index) => index,
					None => {
						let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
						return Err(Signal::raise(
							Raised::Native(Error::ArgumentError(ArgumentError::Unknown(name))),
							vec![(String::new(), loc)],
						));
					}
				},
				None => {
					if next == params.len() {
						let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
						return Err(Signal::raise(
							Raised::Native(Error::ArgumentError(ArgumentError::TooMany(
								arg_count,
								params.len(),
							))),
							vec![(String::new(), loc)],
						));
					}
					let index = next;
					next += 1;
					index
				}
			};
			if slots[index].is_some() {
				let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
				return Err(Signal::raise(
					Raised::Native(Error::ArgumentError(ArgumentError::Duplicate(
						params[index].name,
					))),
					vec![(String::new(), loc)],
				));
			}
			slots[index] = Some(val);
		}

		// Every unfilled parameter is found here, before bind_args runs any
		// default, so a call that cannot succeed evaluates none of them.
		let missing = params
			.iter()
			.zip(&slots)
			.filter(|(param, slot)| slot.is_none() && param.default.is_none())
			.map(|(param, _)| param.name)
			.collect::<Vec<_>>();
		if !missing.is_empty() {
			let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
			return Err(Signal::raise(
				Raised::Native(Error::ArgumentError(ArgumentError::Missing(missing))),
				vec![(String::new(), loc)],
			));
		}
		Ok(slots)
	}

	// Fill the frame in declaration order, so a default sees every parameter to
	// its left. A default that raises does so inside the callee.
	fn bind_args(
		&mut self,
		chunk_id: ChunkId,
		params: &[Param],
		slots: Vec<Option<Val>>,
		scope: &Rc<RefCell<Scope>>,
	) -> Result<(), Signal> {
		let saved_scope = self.scope.clone();
		self.scope = scope.clone();
		for (param, slot) in params.iter().zip(slots) {
			let val = match slot {
				Some(val) => val,
				None => {
					let chunk = self.rt.pkgs.desc(self.pkg_id).chunks().get(chunk_id);
					match self.eval_expr(chunk, param.default.unwrap()) {
						Ok(val) => val,
						Err(signal) => {
							self.scope = saved_scope;
							return Err(signal);
						}
					}
				}
			};
			scope.borrow_mut().locals.insert(param.name, val);
		}
		self.scope = saved_scope;
		Ok(())
	}

	fn eval_proc_call(
		&mut self,
		span: Span,
		proc: &Proc,
		receiver: Option<Val>,
		args: Vec<(Option<Sym>, Val)>,
	) -> Result<Val, Signal> {
		let slots = self.slot_args(span, &proc.params, args)?;
		let scope = Rc::new(RefCell::new(Scope {
			locals: FxHashMap::default(),
			outer: Some(proc.scope.clone()),
			tier: Tier::Local,
		}));
		let saved_receiver = self.receiver.clone();
		self.receiver = receiver;
		// The body, and any default argument written alongside it, live in the
		// callee's package. Errors raised while running them must resolve
		// against that package's sources, not the caller's.
		let saved_pkg_id = self.pkg_id;
		self.pkg_id = proc.pkg;
		let body_chunk = self.rt.pkgs.desc(proc.pkg).chunks().get(proc.chunk);
		let result = match self.bind_args(proc.chunk, &proc.params, slots, &scope) {
			Ok(()) => self.eval_block(body_chunk, scope, proc.body),
			Err(signal) => Err(signal),
		};
		self.pkg_id = saved_pkg_id;
		self.receiver = saved_receiver;
		match result {
			Ok(val) => Ok(val),
			Err(Signal::Return(val)) => Ok(val),
			Err(Signal::Break(_)) => panic!(),
			Err(Signal::Raise(Raise(err, mut trace))) => {
				let proc_name = self.syms.resolve(proc.name);
				trace.last_mut().unwrap().0.push_str(proc_name);

				let loc = self.rt.pkgs.desc(self.pkg_id).loc(span);
				trace.push((String::new(), loc));
				Err(Signal::Raise(Raise(err, trace)))
			}
		}
	}

	fn eval_block(
		&mut self,
		chunk: &Chunk,
		scope: Rc<RefCell<Scope>>,
		block_id: BlockId,
	) -> Result<Val, Signal> {
		let block = chunk.get_block(block_id);
		let saved = self.scope.clone();
		self.scope = scope;
		let mut res = Val::Nil(Nil);
		for expr_id in &block.exprs {
			match self.eval_expr(chunk, *expr_id) {
				Ok(val) => {
					res = val;
				}
				Err(Signal::Return(val)) => {
					self.scope = saved;
					return Err(Signal::Return(val));
				}
				Err(Signal::Break(val)) => {
					self.scope = saved;
					return Err(Signal::Break(val));
				}
				Err(Signal::Raise(raise)) => {
					self.scope = saved;
					return Err(Signal::Raise(raise));
				}
			}
		}
		self.scope = saved;
		Ok(res)
	}
}
