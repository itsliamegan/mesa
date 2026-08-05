use std::cell::RefCell;
use std::cmp::{Eq, PartialEq};
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

use crate::syn::{
	Assign, Builtin, Call, Chunk, Decl, DeclId, Def, Each, Expr, ExprId, Ident, Interner, Lit,
	Location, Package, Place, Return, Script, SymId, Type, When,
};

#[derive(Debug)]
pub enum Error {
	WrongArgCount(Location, usize, usize),
	CallNonCallable(Location, String),
	IterNonIterable(Location, String),
	ScriptNonScriptable(Location, String),
	ScriptNonIndex(Location, String),
	UnboundIdent(Location, String),
}

impl Error {
	fn loc(&self) -> &Location {
		match self {
			Self::WrongArgCount(loc, _, _) => loc,
			Self::CallNonCallable(loc, _) => loc,
			Self::IterNonIterable(loc, _) => loc,
			Self::ScriptNonScriptable(loc, _) => loc,
			Self::ScriptNonIndex(loc, _) => loc,
			Self::UnboundIdent(loc, _) => loc,
		}
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		write!(f, "{}: runtime error: ", self.loc())?;
		match self {
			Self::WrongArgCount(_, have, want) => {
				write!(f, "wrong number of args; have {}, want {}", have, want)
			}
			Self::CallNonCallable(_, val) => write!(f, "call of non-callable {}", val),
			Self::IterNonIterable(_, val) => write!(f, "iter of non-iterable {}", val),
			Self::ScriptNonScriptable(_, val) => write!(f, "script of non-scriptable {}", val),
			Self::ScriptNonIndex(_, val) => write!(f, "script with non-index {}", val),
			Self::UnboundIdent(_, ident) => write!(f, "unbound ident '{}'", ident),
		}
	}
}

#[derive(Debug, Clone)]
enum Val {
	Num(f64),
	Bool(bool),
	Ref(Ref),
	Nil,
}

impl PartialEq for Val {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Num(num), Self::Num(other_num)) => num == other_num,
			(Self::Bool(bool), Self::Bool(other_bool)) => bool == other_bool,
			(Self::Ref(rf), Self::Ref(other_rf)) => rf == other_rf,
			(Self::Nil, Self::Nil) => true,
			_ => false,
		}
	}
}

impl Eq for Val {}

impl Hash for Val {
	fn hash<H: Hasher>(&self, state: &mut H) {
		match self {
			Self::Num(num) => num.to_bits().hash(state),
			Self::Bool(bool) => bool.hash(state),
			Self::Ref(rf) => rf.hash(state),
			Self::Nil => ().hash(state),
		}
	}
}

#[derive(Debug, Clone)]
struct Ref(Rc<RefCell<Obj>>);

impl Ref {
	fn new(obj: Obj) -> Self {
		Self(Rc::new(RefCell::new(obj)))
	}

	fn get(&self) -> impl Deref<Target = Obj> {
		self.0.borrow()
	}
}

impl PartialEq for Ref {
	fn eq(&self, other: &Self) -> bool {
		match (&*self.get(), &*other.get()) {
			(Obj::Str(str), Obj::Str(other_str)) => str == other_str,
			_ => self.0.as_ptr() == other.0.as_ptr(),
		}
	}
}

impl Eq for Ref {}

impl Hash for Ref {
	fn hash<H: Hasher>(&self, state: &mut H) {
		match &*self.get() {
			Obj::Str(str) => str.hash(state),
			_ => self.0.as_ptr().hash(state),
		}
	}
}

#[derive(Debug)]
enum Obj {
	Str(String),
	List(Vec<Val>),
	Dict(HashMap<Val, Val>),
	Proc(Proc),
}

#[derive(Debug)]
struct Proc {
	name: SymId,
	params: Vec<SymId>,
	body: Vec<ExprId>,
	scope: Rc<RefCell<Scope>>,
}

#[derive(Debug)]
struct Scope {
	vals: HashMap<SymId, Val>,
	outer: Option<Rc<RefCell<Scope>>>,
}

impl Scope {
	fn root() -> Self {
		Self {
			vals: HashMap::new(),
			outer: None,
		}
	}

	fn within(outer: Rc<RefCell<Self>>) -> Self {
		Self {
			vals: HashMap::new(),
			outer: Some(outer),
		}
	}

	fn lookup(&self, name: SymId) -> Option<Val> {
		match self.vals.get(&name) {
			Some(val) => Some(val.clone()),
			None => match &self.outer {
				Some(outer) => outer.borrow().lookup(name),
				None => None,
			},
		}
	}

	fn assign(&mut self, name: SymId, val: Val) {
		self.vals.insert(name, val);
	}
}

pub struct Interpreter<'syms, 'pkg> {
	syms: &'syms Interner,
	pkg: &'pkg Package,
	scope: Rc<RefCell<Scope>>,
}

enum Signal {
	Return(Val),
	Error(Error),
}

impl<'syms, 'pkg> Interpreter<'syms, 'pkg> {
	pub fn new(syms: &'syms Interner, pkg: &'pkg Package) -> Self {
		Self {
			syms,
			pkg,
			scope: Rc::new(RefCell::new(Scope::root())),
		}
	}

	pub fn eval(mut self, chunk: Chunk) -> Result<(), Error> {
		for decl_id in &chunk.top {
			match self.eval_decl(&chunk, *decl_id) {
				Ok(()) => {}
				Err(Signal::Return(_)) => break,
				Err(Signal::Error(err)) => return Err(err),
			}
		}
		Ok(())
	}

	fn eval_decl(&mut self, chunk: &Chunk, decl_id: DeclId) -> Result<(), Signal> {
		match chunk.get_decl(decl_id) {
			Decl::Type(Type(_, name, attrs, body)) => Ok(()),
			Decl::Def(Def(_, name, params, body)) => {
				let obj = Obj::Proc(Proc {
					name: *name,
					params: params.to_vec(),
					body: body.to_vec(),
					scope: self.scope.clone(),
				});
				let val = Val::Ref(Ref::new(obj));
				self.scope.borrow_mut().assign(*name, val);
				Ok(())
			}
			Decl::Expr(expr_id) => {
				self.eval_expr(chunk, *expr_id)?;
				Ok(())
			}
		}
	}

	fn eval_expr(&mut self, chunk: &Chunk, expr_id: ExprId) -> Result<Val, Signal> {
		match chunk.get_expr(expr_id) {
			Expr::Each(Each(tok, name, iter, body)) => match self.eval_expr(chunk, *iter)? {
				Val::Ref(rf) => match &*rf.get() {
					Obj::List(items) => {
						let outer_scope = self.scope.clone();
						let inner_scope = Scope::within(outer_scope.clone());
						self.scope = Rc::new(RefCell::new(inner_scope));
						for item in items {
							self.scope.borrow_mut().assign(*name, item.clone());
							for expr_id in body {
								match self.eval_expr(chunk, *expr_id) {
									Ok(_) => {}
									Err(Signal::Return(val)) => {
										self.scope = outer_scope;
										return Err(Signal::Return(val));
									}
									Err(Signal::Error(err)) => {
										self.scope = outer_scope;
										return Err(Signal::Error(err));
									}
								}
							}
						}
						self.scope = outer_scope;
						Ok(Val::Nil)
					}
					obj => {
						let src = self.pkg.get_src(chunk.src);
						let loc = src.loc(tok.idx);
						Err(Signal::Error(Error::IterNonIterable(
							loc,
							format!("{:?}", obj),
						)))
					}
				},
				val => {
					let src = self.pkg.get_src(chunk.src);
					let loc = src.loc(tok.idx);
					Err(Signal::Error(Error::IterNonIterable(
						loc,
						format!("{:?}", val),
					)))
				}
			},
			Expr::When(When(_, cond, then_branch, else_branch)) => {
				let cond = match self.eval_expr(chunk, *cond)? {
					Val::Bool(true) => true,
					Val::Num(num) if num > 0.0 => true,
					Val::Ref(_) => true,
					_ => false,
				};
				if cond {
					let scope = Scope::within(self.scope.clone());
					self.eval_exprs(chunk, scope, then_branch)
				} else if let Some(else_branch) = else_branch {
					let scope = Scope::within(self.scope.clone());
					self.eval_exprs(chunk, scope, else_branch)
				} else {
					Ok(Val::Nil)
				}
			}
			Expr::Return(Return(_, val_expr_id)) => {
				let val = self.eval_expr(chunk, *val_expr_id)?;
				Err(Signal::Return(val))
			}
			Expr::Call(Call(tok, val_id, arg_ids)) => {
				let mut args = Vec::with_capacity(arg_ids.len());
				for arg_id in arg_ids {
					let arg = self.eval_expr(chunk, *arg_id)?;
					args.push(arg);
				}
				match self.eval_expr(chunk, *val_id)? {
					Val::Ref(rf) => match &*rf.get() {
						Obj::Proc(proc) => {
							if args.len() != proc.params.len() {
								let src = self.pkg.get_src(chunk.src);
								let loc = src.loc(tok.idx);
								return Err(Signal::Error(Error::WrongArgCount(
									loc,
									args.len(),
									proc.params.len(),
								)));
							}
							let mut scope = Scope::within(proc.scope.clone());
							for (arg, param) in args.into_iter().zip(proc.params.iter()) {
								scope.assign(*param, arg);
							}
							match self.eval_exprs(chunk, scope, &proc.body) {
								Ok(val) => Ok(val),
								Err(Signal::Return(val)) => Ok(val),
								Err(err) => Err(err),
							}
						}
						obj => {
							let src = self.pkg.get_src(chunk.src);
							let loc = src.loc(tok.idx);
							Err(Signal::Error(Error::CallNonCallable(
								loc,
								rt_debug(self.syms, &Val::Ref(rf.clone())),
							)))
						}
					},
					val => {
						let src = self.pkg.get_src(chunk.src);
						let loc = src.loc(tok.idx);
						Err(Signal::Error(Error::CallNonCallable(
							loc,
							rt_debug(self.syms, &val),
						)))
					}
				}
			}
			Expr::Script(Script(tok, val_id, key_id)) => match self.eval_expr(chunk, *val_id)? {
				Val::Ref(rf) => match &*rf.get() {
					Obj::List(items) => {
						let idx = match self.eval_expr(chunk, *key_id)? {
							Val::Num(num) if num >= 0.0 => num.trunc() as usize,
							val => {
								let src = self.pkg.get_src(chunk.src);
								let loc = src.loc(tok.idx);
								return Err(Signal::Error(Error::ScriptNonIndex(
									loc,
									rt_debug(self.syms, &val),
								)));
							}
						};
						match items.get(idx) {
							Some(val) => Ok(val.clone()),
							None => Ok(Val::Nil),
						}
					}
					Obj::Dict(pairs) => {
						let key = self.eval_expr(chunk, *key_id)?;
						match pairs.get(&key) {
							Some(val) => Ok(val.clone()),
							None => Ok(Val::Nil),
						}
					}
					obj => {
						let src = self.pkg.get_src(chunk.src);
						let loc = src.loc(tok.idx);
						Err(Signal::Error(Error::ScriptNonScriptable(
							loc,
							rt_debug(self.syms, &Val::Ref(rf.clone())),
						)))
					}
				},
				val => {
					let src = self.pkg.get_src(chunk.src);
					let loc = src.loc(tok.idx);
					Err(Signal::Error(Error::ScriptNonScriptable(
						loc,
						rt_debug(self.syms, &val),
					)))
				}
			},
			Expr::Assign(Assign(_, place, val_expr_id)) => {
				let val = self.eval_expr(chunk, *val_expr_id)?;
				match place {
					Place::Ident(Ident(_, sym_id)) => {
						self.scope.borrow_mut().assign(*sym_id, val.clone());
						Ok(val)
					}
				}
			}
			Expr::Ident(Ident(tok, sym_id)) => match self.scope.borrow().lookup(*sym_id) {
				Some(val) => Ok(val),
				None => {
					let src = self.pkg.get_src(chunk.src);
					let loc = src.loc(tok.idx);
					let name = self.syms.get_by_id(*sym_id);
					Err(Signal::Error(Error::UnboundIdent(loc, name.1.clone())))
				}
			},
			Expr::Builtin(builtin) => match builtin {
				Builtin::Debug(_, val_id) => {
					let val = self.eval_expr(chunk, *val_id)?;
					let desc = match &val {
						Val::Num(_) => "Num",
						Val::Bool(_) => "Bool",
						Val::Ref(rf) => match &*rf.get() {
							Obj::Str(_) => "Str",
							Obj::List(_) => "List",
							Obj::Dict(_) => "Dict",
							Obj::Proc(_) => "Proc",
						},
						Val::Nil => "Nil",
					};
					println!("({})\t{}", desc, rt_debug(self.syms, &val));
					Ok(val)
				}
			},
			Expr::Lit(lit) => Ok(match lit {
				Lit::Str(_, str) => Val::Ref(Ref::new(Obj::Str(str.clone()))),
				Lit::Num(_, num) => Val::Num(*num),
				Lit::Bool(_, bool) => Val::Bool(*bool),
				Lit::List(_, item_ids) => {
					let mut items = Vec::with_capacity(item_ids.len());
					for item_id in item_ids {
						let item = self.eval_expr(chunk, *item_id)?;
						items.push(item);
					}
					Val::Ref(Ref::new(Obj::List(items)))
				}
				Lit::Dict(_, pair_ids) => {
					let mut pairs = HashMap::with_capacity(pair_ids.len());
					for (key_id, val_id) in pair_ids {
						let key = self.eval_expr(chunk, *key_id)?;
						let val = self.eval_expr(chunk, *val_id)?;
						pairs.insert(key, val);
					}
					Val::Ref(Ref::new(Obj::Dict(pairs)))
				}
				Lit::Nil(_) => Val::Nil,
			}),
		}
	}

	fn eval_exprs(&mut self, chunk: &Chunk, scope: Scope, body: &[ExprId]) -> Result<Val, Signal> {
		let saved = self.scope.clone();
		self.scope = Rc::new(RefCell::new(scope));
		let mut res = Val::Nil;
		for expr_id in body {
			match self.eval_expr(chunk, *expr_id) {
				Ok(val) => {
					res = val;
				}
				Err(Signal::Return(val)) => {
					self.scope = saved;
					return Err(Signal::Return(val));
				}
				Err(Signal::Error(err)) => {
					self.scope = saved;
					return Err(Signal::Error(err));
				}
			}
		}
		self.scope = saved;
		Ok(res)
	}
}

fn rt_debug(syms: &Interner, val: &Val) -> String {
	match val {
		Val::Num(num) => format!("{}", num),
		Val::Bool(bool) => format!("{}", bool),
		Val::Ref(rf) => match &*rf.get() {
			Obj::Str(str) => format!("\"{}\"", str),
			Obj::List(items) => {
				let mut res = String::new();
				res.push('[');
				for (i, item) in items.iter().enumerate() {
					res.push_str(&rt_debug(syms, item));
					if i + 1 != items.len() {
						res.push_str(", ");
					}
				}
				res.push(']');
				res
			}
			Obj::Dict(entries) => {
				let mut res = String::new();
				res.push('{');
				for (i, (key, val)) in entries.iter().enumerate() {
					res.push_str(&rt_debug(syms, key));
					res.push_str(": ");
					res.push_str(&rt_debug(syms, val));
					if i + 1 != entries.len() {
						res.push_str(", ");
					}
				}
				res.push('}');
				res
			}
			Obj::Proc(proc) => {
				let mut res = String::new();
				let name = syms.get_by_id(proc.name);
				res.push_str(&format!("def {}(", name.1));
				for (i, param) in proc.params.iter().enumerate() {
					let param = syms.get_by_id(*param);
					res.push_str(&param.1);
					if i + 1 != proc.params.len() {
						res.push_str(", ");
					}
				}
				res.push(')');
				res
			}
		},
		Val::Nil => String::from("nil"),
	}
}
