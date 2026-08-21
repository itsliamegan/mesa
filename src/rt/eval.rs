use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use ordermap::OrderMap;
use rustc_hash::{FxBuildHasher, FxHashMap};

use crate::intern::{Interner, Sym};
use crate::pkg::{self, PackageId};
use crate::rt::modules::Modules;
use crate::rt::pkg::Packages;
use crate::rt::scope::{Place, Scope, Tier};
use crate::rt::types::{NativeParam, Natives, TypeId, Types};
use crate::rt::val::{
	Bool, Char, Dict, Instance, List, Member, Method, Nil, Num, Obj, Proc, Str, Val, rt_debug_val,
	rt_print_val,
};
use crate::rt::{ArgumentError, Error, IndexError, MemberError, ProtocolError, TypeError};
use crate::sem::modules;
use crate::src::{Location, Span};
use crate::syn::nodes::{
	BinaryOp, BlockId, Builtin, Expr, ExprId, Lit, ModuleItem, ModuleItemId, Param, UnaryOp,
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
	let Some(prelude_mod) = stdlib_pkg.modules.by_path(&path) else {
		return Err("stdlib does not declare a 'Core.Prelude' module".to_string());
	};
	let chunk_id = stdlib_pkg.modules.chunk(prelude_mod);
	let chunk = stdlib_pkg.chunks.get(chunk_id);

	let mut locals = HashMap::with_capacity_and_hasher(chunk.top.len(), FxBuildHasher);
	for item_id in &chunk.top {
		// 'Core.Prelude' declares nothing but extern types, so this is the
		// only module item that matters here.
		if let ModuleItem::Extern(extern_) = chunk.get_module_item(*item_id) {
			let id = natives.id(stdlib, extern_.name).unwrap();
			locals.insert(extern_.name, Val::Obj(natives.val(id)));
		}
	}
	let scope = Rc::new(RefCell::new(Scope {
		locals,
		outer: None,
		tier: Tier::Prelude,
	}));

	Ok(Prelude { scope })
}

pub struct Interpreter<'syms, 'descs, 'pkgs> {
	syms: &'syms Interner,
	pkgs: &'pkgs Packages<'descs>,
	pkg_id: PackageId,
	scope: Rc<RefCell<Scope>>,
	receiver: Option<Val>,
}

enum Signal {
	Return(Val),
	Break(Val),
	Error(Error, Vec<(String, Location)>),
}

impl<'syms, 'descs, 'pkgs> Interpreter<'syms, 'descs, 'pkgs> {
	pub fn new(
		syms: &'syms mut Interner,
		pkgs: &'pkgs Packages<'descs>,
		pkg_id: PackageId,
	) -> Self {
		let desc = pkgs.desc(pkg_id);
		let first_chunk = desc.modules.chunk(desc.modules.ids().next().unwrap());
		let scope = pkgs.get(pkg_id).mods.scope(first_chunk);

		Self {
			syms,
			pkgs,
			pkg_id,
			scope,
			receiver: None,
		}
	}

	fn types(&self) -> &'pkgs Types<'descs> {
		&self.pkgs.get(self.pkg_id).types
	}

	fn mods(&self) -> &'pkgs Modules<'descs> {
		&self.pkgs.get(self.pkg_id).mods
	}

	pub fn eval(mut self) -> Result<(), (Error, Vec<(String, Location)>)> {
		// Declarations are order-independent, so every module's items enter its
		// scope before anything is evaluated.
		for id in self.mods().descs.ids() {
			let chunk_id = self.mods().descs.chunk(id);
			self.scope = self.mods().scope(chunk_id);
			let chunk = self.pkgs.desc(self.pkg_id).chunks.get(chunk_id);
			for item_id in &chunk.top {
				self.bind_module_item(chunk, chunk_id, *item_id);
			}
		}

		// Only evaluated bindings are sequenced, in import order, so a
		// dependency's top-level assignment always runs before a dependent
		// reads it.
		for id in self.mods().descs.order() {
			let chunk_id = self.mods().descs.chunk(*id);
			self.scope = self.mods().scope(chunk_id);
			let chunk = self.pkgs.desc(self.pkg_id).chunks.get(chunk_id);
			for item_id in &chunk.top {
				match self.eval_module_expr(chunk, *item_id) {
					Ok(()) => {}
					Err(Signal::Return(_)) => break,
					Err(Signal::Break(_)) => panic!(),
					Err(Signal::Error(err, trace)) => return Err((err, trace)),
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
			ModuleItem::Type(typ) => {
				let id = self.types().descs.get_type_by_item(chunk_id, item_id);
				let val = Val::Obj(self.types().user(id).val.clone());
				self.scope.borrow_mut().locals.insert(typ.name, val);
			}
			// An extern type's canonical value is the native type's, and its
			// members were merged into that type rather than described here, so
			// the name is all there is to resolve it by.
			ModuleItem::Extern(extern_) => {
				let id = self.pkgs.native_id(self.pkg_id, extern_.name).unwrap();
				let val = Val::Obj(self.pkgs.native_val(id));
				self.scope.borrow_mut().locals.insert(extern_.name, val);
			}
			ModuleItem::Proto(proto) => {
				let id = self.types().descs.get_proto_by_item(chunk_id, item_id);
				let val = Val::Obj(Rc::new(RefCell::new(Obj::Proto(self.pkg_id, id))));
				self.scope.borrow_mut().locals.insert(proto.name, val);
			}
			ModuleItem::Def(def) => {
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
				Val::Obj(rf) => match &*rf.borrow() {
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
						for key in dict.pairs.keys() {
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
							.pkgs
							.desc(self.pkg_id)
							.loc(chunk.get_expr_span(expr_id));
						let type_name = self.types().name(self.syms, self.pkgs, obj.type_id());
						Err(Signal::Error(
							Error::ProtocolError(ProtocolError::NotIterable(type_name)),
							vec![(String::new(), loc)],
						))
					}
				},
				val => {
					let loc = self
						.pkgs
						.desc(self.pkg_id)
						.loc(chunk.get_expr_span(expr_id));
					let type_name = self.types().name(self.syms, self.pkgs, val.type_id());
					Err(Signal::Error(
						Error::ProtocolError(ProtocolError::NotIterable(type_name)),
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
					let path_val = self.eval_expr(chunk, arm.path)?;
					let arm_type_id = match &path_val {
						Val::Obj(rf) => match &*rf.borrow() {
							Obj::Type(id) => Some(*id),
							_ => None,
						},
						_ => None,
					};
					let Some(arm_type_id) = arm_type_id else {
						let loc = self
							.pkgs
							.desc(self.pkg_id)
							.loc(chunk.get_expr_span(arm.path));
						let type_name = self.types().name(self.syms, self.pkgs, path_val.type_id());
						return Err(Signal::Error(
							Error::TypeError(TypeError::CaseNonType(type_name)),
							vec![(String::new(), loc)],
						));
					};
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
			Expr::Call(call) => {
				let span = chunk.get_expr_span(expr_id);
				let mut args = Vec::with_capacity(call.args.len());
				for arg in &call.args {
					let val = self.eval_expr(chunk, arg.val)?;
					args.push((arg.name, val));
				}
				let callee = self.eval_expr_raw(chunk, call.callee)?;
				match callee {
					Val::Obj(rf) => match &*rf.borrow() {
						Obj::Proc(proc) => self.eval_proc_call(span, proc, None, args),
						Obj::Type(type_id) => match type_id {
							TypeId::User(type_pkg, type_id) => {
								let (type_pkg, type_id) = (*type_pkg, *type_id);
								// The description outlives the interpreter, so a type's
								// fields are read where they were written rather than
								// copied onto every construction.
								let desc = self.pkgs.get(type_pkg).types.descs.get_type(type_id);
								// A type with variants cannot itself be constructed.
								if let Some(variants) = &desc.variants
									&& !variants.is_empty()
								{
									let loc = self.pkgs.desc(self.pkg_id).loc(span);
									let type_name = self.types().name(
										self.syms,
										self.pkgs,
										TypeId::User(type_pkg, type_id),
									);
									return Err(Signal::Error(
										Error::TypeError(TypeError::NotConstructible(type_name)),
										vec![(String::new(), loc)],
									));
								}
								let ctor_fields = &desc.ctor_fields;
								let body_fields = &desc.body_fields;
								let type_chunk_id = desc.chunk;
								let type_scope = self.pkgs.get(type_pkg).mods.scope(type_chunk_id);

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

								let type_chunk = self.pkgs.desc(type_pkg).chunks.get(type_chunk_id);
								let inst_rf = Rc::new(RefCell::new(Obj::Instance(Instance {
									pkg: type_pkg,
									typ: type_id,
									fields,
								})));

								let saved_scope = self.scope.clone();
								let saved_receiver = self.receiver.clone();
								self.receiver = Some(Val::Obj(inst_rf.clone()));
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
									Place::Member(Member::User(inst_rf.clone(), *name))
										.set(self.pkgs, val)
										.unwrap();
								}
								self.scope = saved_scope;
								self.receiver = saved_receiver;
								self.pkg_id = saved_pkg_id;

								Ok(Val::Obj(inst_rf))
							}
							TypeId::Native(type_id) => {
								let typ = self.pkgs.native(*type_id);
								match typ.new {
									Some(new) => {
										// A native constructor defines no
										// params by definition, so route
										// through the usual machinery.
										self.bind_native_args(span, &[], args)?;
										Ok(new())
									}
									None => {
										let loc = self.pkgs.desc(self.pkg_id).loc(span);
										let type_name = self.syms.resolve(typ.desc.name);
										Err(Signal::Error(
											Error::TypeError(TypeError::NotConstructible(
												type_name.to_string(),
											)),
											vec![(String::new(), loc)],
										))
									}
								}
							}
						},
						Obj::Method(meth) => match meth {
							Method::User(recv, proc) => {
								self.eval_proc_call(span, &proc.borrow(), Some(recv.clone()), args)
							}
							Method::Native(recv, _name, meth) => {
								// Native params carry &'static str names and
								// fn() -> Val defaults, so they are matched and
								// filled without an Interner or a scope.
								let args = self.bind_native_args(span, meth.params, args)?;
								match (meth.call)(recv, args) {
									Ok(val) => Ok(val),
									Err(err) => {
										let loc = self.pkgs.desc(self.pkg_id).loc(span);
										Err(Signal::Error(err, vec![(String::new(), loc)]))
									}
								}
							}
						},
						obj => {
							let loc = self.pkgs.desc(self.pkg_id).loc(span);
							let type_name = self.types().name(self.syms, self.pkgs, obj.type_id());
							Err(Signal::Error(
								Error::TypeError(TypeError::NotCallable(type_name)),
								vec![(String::new(), loc)],
							))
						}
					},
					val => {
						let loc = self.pkgs.desc(self.pkg_id).loc(span);
						let type_name = self.types().name(self.syms, self.pkgs, val.type_id());
						Err(Signal::Error(
							Error::TypeError(TypeError::NotCallable(type_name)),
							vec![(String::new(), loc)],
						))
					}
				}
			}
			Expr::Member(member) => {
				let val = self.eval_member_raw(chunk, expr_id, member.receiver, member.name)?;
				self.invoke_or_return(chunk, expr_id, val)
			}
			Expr::Access(access) => match self.eval_expr(chunk, access.receiver)? {
				Val::Obj(rf) => match &*rf.borrow() {
					Obj::List(list) => {
						let idx = self.eval_index(chunk, expr_id, access.key, list.items.len())?;
						Ok(list.items[idx].clone())
					}
					Obj::Dict(dict) => {
						let key = self.eval_expr(chunk, access.key)?;
						match dict.pairs.get(&key) {
							Some(val) => Ok(val.clone()),
							None => {
								let loc = self
									.pkgs
									.desc(self.pkg_id)
									.loc(chunk.get_expr_span(expr_id));
								Err(Signal::Error(
									Error::KeyError(rt_debug_val(
										self.syms,
										self.types(),
										self.pkgs,
										&key,
									)),
									vec![(String::new(), loc)],
								))
							}
						}
					}
					obj => {
						let loc = self
							.pkgs
							.desc(self.pkg_id)
							.loc(chunk.get_expr_span(expr_id));
						let type_name = self.types().name(self.syms, self.pkgs, obj.type_id());
						Err(Signal::Error(
							Error::ProtocolError(ProtocolError::NotAccessible(type_name)),
							vec![(String::new(), loc)],
						))
					}
				},
				val => {
					let loc = self
						.pkgs
						.desc(self.pkg_id)
						.loc(chunk.get_expr_span(expr_id));
					let type_name = self.types().name(self.syms, self.pkgs, val.type_id());
					Err(Signal::Error(
						Error::ProtocolError(ProtocolError::NotAccessible(type_name)),
						vec![(String::new(), loc)],
					))
				}
			},
			Expr::Mention(mention) => {
				let val = self.eval_expr_raw(chunk, mention.val)?;
				self.expect_invocable(chunk, expr_id, val)
			}
			Expr::Assign(assign) => {
				let val = self.eval_expr(chunk, assign.val)?;
				match &assign.place {
					syn::nodes::Place::Name(name) => {
						let sym = name.sym;
						let place = self.resolve_name(sym);
						if let Err(()) = place.set(self.pkgs, val.clone()) {
							let type_name = match &place {
								Place::Member(Member::Module(pkg, id, name)) => {
									let pkg_mods = &self.pkgs.get(*pkg).mods;
									match pkg_mods.descs.member(*id, *name) {
										Some(modules::Member::Child(child)) => {
											format!("module {}", pkg_mods.name(self.syms, child))
										}
										_ => format!("module {}", pkg_mods.name(self.syms, *id)),
									}
								}
								_ => self.namespace_name(self.receiver.as_ref().unwrap()),
							};
							let loc = self
								.pkgs
								.desc(self.pkg_id)
								.loc(chunk.get_expr_span(expr_id));
							return Err(Signal::Error(
								Error::MemberError(MemberError::ReadOnly(
									type_name,
									self.syms.resolve(sym).to_string(),
								)),
								vec![(String::new(), loc)],
							));
						}
						Ok(val)
					}
					syn::nodes::Place::Member(member) => {
						let receiver = self.eval_expr(chunk, member.receiver)?;
						if let Some(m) = self.member(&receiver, member.name) {
							if let Err(()) = Place::Member(m).set(self.pkgs, val.clone()) {
								let type_name = self.namespace_name(&receiver);
								let loc = self
									.pkgs
									.desc(self.pkg_id)
									.loc(chunk.get_expr_span(expr_id));
								return Err(Signal::Error(
									Error::MemberError(MemberError::ReadOnly(
										type_name,
										self.syms.resolve(member.name).to_string(),
									)),
									vec![(String::new(), loc)],
								));
							}
							Ok(val)
						} else {
							let type_name = self.namespace_name(&receiver);
							let loc = self
								.pkgs
								.desc(self.pkg_id)
								.loc(chunk.get_expr_span(expr_id));
							Err(Signal::Error(
								Error::MemberError(MemberError::Missing(
									type_name,
									self.syms.resolve(member.name).to_string(),
								)),
								vec![(String::new(), loc)],
							))
						}
					}
					syn::nodes::Place::Access(access) => {
						let receiver = self.eval_expr(chunk, access.receiver)?;
						match &receiver {
							Val::Obj(rf) => match &mut *rf.borrow_mut() {
								Obj::List(list) => {
									let idx = self.eval_index(
										chunk,
										expr_id,
										access.key,
										list.items.len(),
									)?;
									list.items[idx] = val.clone();
									Ok(val)
								}
								Obj::Dict(dict) => {
									let key = self.eval_expr(chunk, access.key)?;
									dict.pairs.insert(key, val.clone());
									Ok(val)
								}
								obj => {
									let loc = self
										.pkgs
										.desc(self.pkg_id)
										.loc(chunk.get_expr_span(expr_id));
									let type_name =
										self.types().name(self.syms, self.pkgs, obj.type_id());
									Err(Signal::Error(
										Error::ProtocolError(ProtocolError::NotAccessible(
											type_name,
										)),
										vec![(String::new(), loc)],
									))
								}
							},
							val => {
								let loc = self
									.pkgs
									.desc(self.pkg_id)
									.loc(chunk.get_expr_span(expr_id));
								let type_name =
									self.types().name(self.syms, self.pkgs, val.type_id());
								Err(Signal::Error(
									Error::ProtocolError(ProtocolError::NotAccessible(type_name)),
									vec![(String::new(), loc)],
								))
							}
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
						BinaryOp::Eq => Ok(Val::Bool(Bool(lhs == rhs))),
						BinaryOp::NotEq => Ok(Val::Bool(Bool(lhs != rhs))),
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
									let loc = self
										.pkgs
										.desc(self.pkg_id)
										.loc(chunk.get_expr_span(expr_id));
									let val = match (&lhs, &rhs) {
										(Val::Num(_), _) => rhs,
										(_, Val::Num(_)) => lhs,
										_ => lhs,
									};
									let type_name =
										self.types().name(self.syms, self.pkgs, val.type_id());
									Err(Signal::Error(
										Error::ProtocolError(ProtocolError::NotOrderable(
											type_name,
										)),
										vec![(String::new(), loc)],
									))
								}
							}
						}
						BinaryOp::Append => {
							let Val::Obj(rf) = lhs.clone() else {
								let loc = self
									.pkgs
									.desc(self.pkg_id)
									.loc(chunk.get_expr_span(expr_id));
								let type_name =
									self.types().name(self.syms, self.pkgs, lhs.type_id());
								return Err(Signal::Error(
									Error::ProtocolError(ProtocolError::NotAppendable(type_name)),
									vec![(String::new(), loc)],
								));
							};
							let Obj::List(list) = &mut *rf.borrow_mut() else {
								let loc = self
									.pkgs
									.desc(self.pkg_id)
									.loc(chunk.get_expr_span(expr_id));
								let type_name =
									self.types()
										.name(self.syms, self.pkgs, rf.borrow().type_id());
								return Err(Signal::Error(
									Error::ProtocolError(ProtocolError::NotAppendable(type_name)),
									vec![(String::new(), loc)],
								));
							};
							list.items.push(rhs);
							Ok(lhs)
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
									.pkgs
									.desc(self.pkg_id)
									.loc(chunk.get_expr_span(expr_id));
								let type_name =
									self.types().name(self.syms, self.pkgs, rhs.type_id());
								Err(Signal::Error(
									Error::TypeError(TypeError::ConcatNonStr(type_name)),
									vec![(String::new(), loc)],
								))
							}
							(_, rhs) => {
								let loc = self
									.pkgs
									.desc(self.pkg_id)
									.loc(chunk.get_expr_span(expr_id));
								let type_name =
									self.types().name(self.syms, self.pkgs, rhs.type_id());
								Err(Signal::Error(
									Error::TypeError(TypeError::ArithNonNum(type_name)),
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
									.pkgs
									.desc(self.pkg_id)
									.loc(chunk.get_expr_span(expr_id));
								let val = match (&lhs, &rhs) {
									(Val::Num(_), _) => rhs,
									(_, Val::Num(_)) => lhs,
									_ => lhs,
								};
								let type_name =
									self.types().name(self.syms, self.pkgs, val.type_id());
								Err(Signal::Error(
									Error::TypeError(TypeError::ArithNonNum(type_name)),
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
							.pkgs
							.desc(self.pkg_id)
							.loc(chunk.get_expr_span(expr_id));
						let type_name = self.types().name(self.syms, self.pkgs, val.type_id());
						Err(Signal::Error(
							Error::TypeError(TypeError::ArithNonNum(type_name)),
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
					println!("{}", rt_print_val(self.syms, self.types(), self.pkgs, &val));
					Ok(val)
				}
				Builtin::Type { val: val_id } => {
					let val = self.eval_expr(chunk, *val_id)?;
					let type_id = val.type_id();
					match type_id {
						TypeId::User(pkg, id) => {
							Ok(Val::Obj(self.pkgs.get(pkg).types.user(id).val.clone()))
						}
						TypeId::Native(id) => Ok(Val::Obj(self.pkgs.native_val(id))),
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
					let mut pairs = OrderMap::with_capacity(pair_ids.len());
					for (key_id, val_id) in pair_ids {
						let key = self.eval_expr(chunk, *key_id)?;
						let val = self.eval_expr(chunk, *val_id)?;
						pairs.insert(key, val);
					}
					Val::Obj(Rc::new(RefCell::new(Obj::Dict(Dict { pairs }))))
				}
				Lit::Nil => Val::Nil(Nil),
			}),
		}
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
		if let Val::Obj(rf) = val
			&& let Obj::Module(pkg, id) = &*rf.borrow()
		{
			let pkg_mods = &self.pkgs.get(*pkg).mods;
			if pkg_mods.descs.member(*id, name).is_some() {
				return Some(Member::Module(*pkg, *id, name));
			}
		}
		self.types().member(self.pkgs, val, name)
	}

	fn namespace_name(&self, val: &Val) -> String {
		if let Val::Obj(rf) = val
			&& let Obj::Module(pkg, id) = &*rf.borrow()
		{
			let pkg_mods = &self.pkgs.get(*pkg).mods;
			return format!("module {}", pkg_mods.name(self.syms, *id));
		}
		format!(
			"type {}",
			self.types()
				.name(self.syms, self.pkgs, val.namespace_type_id())
		)
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
				.mods()
				.descs
				.binding(self.pkgs.descs(), module_id, name)
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
				.pkgs
				.desc(self.pkg_id)
				.loc(chunk.get_expr_span(expr_id));
			let name = self.syms.resolve(name);
			return Err(Signal::Error(
				Error::NameError(name.to_string()),
				vec![(String::new(), loc)],
			));
		}
		match place.get(self.pkgs) {
			Ok(val) => Ok(val),
			Err(err) => {
				let loc = self
					.pkgs
					.desc(self.pkg_id)
					.loc(chunk.get_expr_span(expr_id));
				Err(Signal::Error(err, vec![(String::new(), loc)]))
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
			match Place::Member(member).get(self.pkgs) {
				Ok(val) => Ok(val),
				Err(err) => {
					let loc = self
						.pkgs
						.desc(self.pkg_id)
						.loc(chunk.get_expr_span(expr_id));
					Err(Signal::Error(err, vec![(String::new(), loc)]))
				}
			}
		} else {
			let type_name = self.namespace_name(&val);
			let loc = self
				.pkgs
				.desc(self.pkg_id)
				.loc(chunk.get_expr_span(expr_id));
			Err(Signal::Error(
				Error::MemberError(MemberError::Missing(
					type_name,
					self.syms.resolve(name).to_string(),
				)),
				vec![(String::new(), loc)],
			))
		}
	}

	fn eval_index(
		&mut self,
		chunk: &Chunk,
		expr_id: ExprId,
		key_id: ExprId,
		len: usize,
	) -> Result<usize, Signal> {
		let num = match self.eval_expr(chunk, key_id)? {
			Val::Num(num) => num.0,
			val => {
				let loc = self
					.pkgs
					.desc(self.pkg_id)
					.loc(chunk.get_expr_span(expr_id));
				return Err(Signal::Error(
					Error::TypeError(TypeError::IndexNonNum(rt_print_val(
						self.syms,
						self.types(),
						self.pkgs,
						&val,
					))),
					vec![(String::new(), loc)],
				));
			}
		};

		if num != num.trunc() {
			let loc = self
				.pkgs
				.desc(self.pkg_id)
				.loc(chunk.get_expr_span(expr_id));
			return Err(Signal::Error(
				Error::IndexError(IndexError::NonIntegral(num)),
				vec![(String::new(), loc)],
			));
		}

		if num < 0.0 || num >= len as f64 {
			let loc = self
				.pkgs
				.desc(self.pkg_id)
				.loc(chunk.get_expr_span(expr_id));
			return Err(Signal::Error(
				Error::IndexError(IndexError::OutOfRange(num)),
				vec![(String::new(), loc)],
			));
		}

		Ok(num as usize)
	}

	fn expect_invocable(
		&mut self,
		chunk: &Chunk,
		expr_id: ExprId,
		val: Val,
	) -> Result<Val, Signal> {
		if let Val::Obj(rf) = &val {
			match &*rf.borrow() {
				Obj::Proc(_) | Obj::Method(_) => return Ok(val.clone()),
				_ => {}
			}
		}

		let loc = self
			.pkgs
			.desc(self.pkg_id)
			.loc(chunk.get_expr_span(expr_id));
		let type_name = self.types().name(self.syms, self.pkgs, val.type_id());
		Err(Signal::Error(
			Error::TypeError(TypeError::NotInvokable(type_name)),
			vec![(String::new(), loc)],
		))
	}

	fn invoke_or_return(
		&mut self,
		chunk: &Chunk,
		expr_id: ExprId,
		val: Val,
	) -> Result<Val, Signal> {
		let Val::Obj(rf) = &val else { return Ok(val) };
		let rf = rf.clone();
		let span = chunk.get_expr_span(expr_id);

		// Required params precede defaulted ones, so the first param alone says
		// whether this needs any arguments.
		let missing = match &*rf.borrow() {
			Obj::Proc(proc) => proc
				.params
				.first()
				.filter(|param| param.default.is_none())
				.map(|param| self.syms.resolve(param.name).to_string()),
			Obj::Method(Method::User(_, proc)) => proc
				.borrow()
				.params
				.first()
				.filter(|param| param.default.is_none())
				.map(|param| self.syms.resolve(param.name).to_string()),
			// Native types are defined in Rust, skipping the
			// required-before-default ordering check. Use find instead of
			// filter to ensure we find any required params.
			Obj::Method(Method::Native(_, _, meth)) => meth
				.params
				.iter()
				.find(|param| param.default.is_none())
				.map(|param| param.name.to_string()),
			_ => return Ok(val),
		};

		// Used in an invoking position but requires arguments; error.
		if let Some(name) = missing {
			let loc = self.pkgs.desc(self.pkg_id).loc(span);
			return Err(Signal::Error(
				Error::ArgumentError(ArgumentError::Missing(vec![name])),
				vec![(String::new(), loc)],
			));
		}

		match &*rf.borrow() {
			Obj::Proc(proc) => self.eval_proc_call(span, proc, None, Vec::new()),
			Obj::Method(Method::User(recv, proc)) => {
				self.eval_proc_call(span, &proc.borrow(), Some(recv.clone()), Vec::new())
			}
			Obj::Method(Method::Native(recv, _name, meth)) => {
				match (meth.call)(recv, meth.defaults().unwrap()) {
					Ok(val) => Ok(val),
					Err(err) => {
						let loc = self.pkgs.desc(self.pkg_id).loc(span);
						Err(Signal::Error(err, vec![(String::new(), loc)]))
					}
				}
			}
			_ => panic!(),
		}
	}

	fn bind_native_args(
		&self,
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
					let name = self.syms.resolve(name);
					match params.iter().position(|param| param.name == name) {
						Some(index) => index,
						None => {
							let loc = self.pkgs.desc(self.pkg_id).loc(span);
							return Err(Signal::Error(
								Error::ArgumentError(ArgumentError::Unknown(name.to_string())),
								vec![(String::new(), loc)],
							));
						}
					}
				}
				None => {
					if next == params.len() {
						let loc = self.pkgs.desc(self.pkg_id).loc(span);
						return Err(Signal::Error(
							Error::ArgumentError(ArgumentError::TooMany(arg_count, params.len())),
							vec![(String::new(), loc)],
						));
					}
					let index = next;
					next += 1;
					index
				}
			};
			if slots[index].is_some() {
				let loc = self.pkgs.desc(self.pkg_id).loc(span);
				let name = params[index].name.to_string();
				return Err(Signal::Error(
					Error::ArgumentError(ArgumentError::Duplicate(name)),
					vec![(String::new(), loc)],
				));
			}
			slots[index] = Some(val);
		}

		let missing = params
			.iter()
			.zip(&slots)
			.filter(|(param, slot)| slot.is_none() && param.default.is_none())
			.map(|(param, _)| param.name.to_string())
			.collect::<Vec<_>>();
		if !missing.is_empty() {
			let loc = self.pkgs.desc(self.pkg_id).loc(span);
			return Err(Signal::Error(
				Error::ArgumentError(ArgumentError::Missing(missing)),
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
						let loc = self.pkgs.desc(self.pkg_id).loc(span);
						let name = self.syms.resolve(name).to_string();
						return Err(Signal::Error(
							Error::ArgumentError(ArgumentError::Unknown(name)),
							vec![(String::new(), loc)],
						));
					}
				},
				None => {
					if next == params.len() {
						let loc = self.pkgs.desc(self.pkg_id).loc(span);
						return Err(Signal::Error(
							Error::ArgumentError(ArgumentError::TooMany(arg_count, params.len())),
							vec![(String::new(), loc)],
						));
					}
					let index = next;
					next += 1;
					index
				}
			};
			if slots[index].is_some() {
				let loc = self.pkgs.desc(self.pkg_id).loc(span);
				let name = self.syms.resolve(params[index].name).to_string();
				return Err(Signal::Error(
					Error::ArgumentError(ArgumentError::Duplicate(name)),
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
			.map(|(param, _)| self.syms.resolve(param.name).to_string())
			.collect::<Vec<_>>();
		if !missing.is_empty() {
			let loc = self.pkgs.desc(self.pkg_id).loc(span);
			return Err(Signal::Error(
				Error::ArgumentError(ArgumentError::Missing(missing)),
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
					let chunk = self.pkgs.desc(self.pkg_id).chunks.get(chunk_id);
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
		let body_chunk = self.pkgs.desc(proc.pkg).chunks.get(proc.chunk);
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
			Err(Signal::Error(err, mut trace)) => {
				let proc_name = self.syms.resolve(proc.name);
				trace.last_mut().unwrap().0.push_str(proc_name);

				let loc = self.pkgs.desc(self.pkg_id).loc(span);
				trace.push((String::new(), loc));
				Err(Signal::Error(err, trace))
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
				Err(Signal::Error(err, loc)) => {
					self.scope = saved;
					return Err(Signal::Error(err, loc));
				}
			}
		}
		self.scope = saved;
		Ok(res)
	}
}
