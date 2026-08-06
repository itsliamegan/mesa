use std::cell::RefCell;
use std::cmp::{Eq, PartialEq};
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

use crate::syn::{
	self, Access, Assign, Builtin, Call, Chunk, Decl, DeclId, Def, Each, Expr, ExprId, Ident,
	Interner, Lit, Location, Package, Place, Return, Script, SymId, When,
};

#[derive(Debug)]
pub enum Error {
	WrongArgCount(Location, usize, usize),
	CallNonCallable(Location, String),
	IterNonIterable(Location, String),
	AccessNonMember(Location, String, String),
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
			Self::AccessNonMember(loc, _, _) => loc,
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
			Self::AccessNonMember(_, typ, name) => {
				write!(f, "no such member '{}' on type '{}'", name, typ)
			}
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
	Type(TypeId),
	Inst(Inst),
}

#[derive(Debug, Clone, Copy)]
struct TypeId(usize);

#[derive(Debug)]
struct Type {
	name: SymId,
	fields: Vec<SymId>,
	methods: HashMap<SymId, Rc<RefCell<Proc>>>,
}

struct Types {
	types: Vec<Type>,
}

impl Types {
	fn new() -> Self {
		Self { types: Vec::new() }
	}

	fn get_type(&self, id: TypeId) -> &Type {
		&self.types[id.0]
	}

	fn add_type(&mut self, typ: Type) -> TypeId {
		let id = TypeId(self.types.len());
		self.types.push(typ);
		id
	}
}

#[derive(Debug)]
struct Inst {
	typ: TypeId,
	fields: HashMap<SymId, Val>,
}

#[derive(Debug)]
struct Proc {
	name: SymId,
	params: Vec<SymId>,
	body: Vec<ExprId>,
	scope: Rc<RefCell<Scope>>,
	inst: Option<Ref>,
}

#[derive(Debug)]
struct Scope {
	locals: HashMap<SymId, Val>,
	inst: Option<Ref>,
	outer: Option<Rc<RefCell<Scope>>>,
}

impl Scope {
	fn lookup(&self, name: SymId) -> Option<Val> {
		match self.locals.get(&name) {
			Some(val) => Some(val.clone()),
			None => match &self.inst {
				Some(rf) => match &*rf.get() {
					Obj::Inst(inst) => match inst.fields.get(&name) {
						Some(val) => Some(val.clone()),
						None => match &self.outer {
							Some(outer) => outer.borrow().lookup(name),
							None => None,
						},
					},
					_ => panic!(),
				},
				None => match &self.outer {
					Some(outer) => outer.borrow().lookup(name),
					None => None,
				},
			},
		}
	}

	fn assign(&mut self, name: SymId, val: Val) {
		self.locals.insert(name, val);
	}
}

pub struct Interpreter<'syms, 'pkg> {
	syms: &'syms Interner,
	pkg: &'pkg Package,
	types: Types,
	scope: Rc<RefCell<Scope>>,
}

enum Signal {
	Return(Val),
	Error(Error),
}

impl<'syms, 'pkg> Interpreter<'syms, 'pkg> {
	pub fn new(syms: &'syms mut Interner, pkg: &'pkg Package) -> Self {
		let mut types = Types::new();
		types.add_type(Type {
			name: syms.get_or_add("Num"),
			fields: Vec::new(),
			methods: HashMap::new(),
		});
		types.add_type(Type {
			name: syms.get_or_add("Bool"),
			fields: Vec::new(),
			methods: HashMap::new(),
		});
		types.add_type(Type {
			name: syms.get_or_add("Nil"),
			fields: Vec::new(),
			methods: HashMap::new(),
		});
		types.add_type(Type {
			name: syms.get_or_add("Str"),
			fields: Vec::new(),
			methods: HashMap::new(),
		});
		types.add_type(Type {
			name: syms.get_or_add("List"),
			fields: Vec::new(),
			methods: HashMap::new(),
		});
		types.add_type(Type {
			name: syms.get_or_add("Dict"),
			fields: Vec::new(),
			methods: HashMap::new(),
		});
		types.add_type(Type {
			name: syms.get_or_add("Type"),
			fields: Vec::new(),
			methods: HashMap::new(),
		});
		types.add_type(Type {
			name: syms.get_or_add("Proc"),
			fields: Vec::new(),
			methods: HashMap::new(),
		});

		let scope = Scope {
			locals: HashMap::new(),
			inst: None,
			outer: None,
		};

		Self {
			syms,
			pkg,
			types,
			scope: Rc::new(RefCell::new(scope)),
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
			Decl::Type(syn::Type(_, name, fields, items)) => {
				let mut methods = HashMap::new();
				for decl_id in items {
					match chunk.get_decl(*decl_id) {
						Decl::Type(syn::Type(_, _, _, _)) => todo!(),
						Decl::Def(Def(_, name, params, body)) => {
							let proc = Proc {
								name: *name,
								params: params.to_vec(),
								body: body.to_vec(),
								scope: self.scope.clone(),
								inst: None,
							};
							methods.insert(*name, Rc::new(RefCell::new(proc)));
						}
						Decl::Expr(_) => todo!(),
					}
				}
				let typ = Type {
					name: *name,
					fields: fields.to_vec(),
					methods,
				};
				let id = self.types.add_type(typ);
				let obj = Obj::Type(id);
				let val = Val::Ref(Ref::new(obj));
				self.scope.borrow_mut().assign(*name, val);
				Ok(())
			}
			Decl::Def(Def(_, name, params, body)) => {
				let obj = Obj::Proc(Proc {
					name: *name,
					params: params.to_vec(),
					body: body.to_vec(),
					scope: self.scope.clone(),
					inst: None,
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
						let inner_scope = Scope {
							locals: HashMap::new(),
							inst: outer_scope.borrow().inst.clone(),
							outer: Some(outer_scope.clone()),
						};
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
					let scope = Scope {
						locals: HashMap::new(),
						inst: self.scope.borrow().inst.clone(),
						outer: Some(self.scope.clone()),
					};
					self.eval_exprs(chunk, scope, then_branch)
				} else if let Some(else_branch) = else_branch {
					let scope = Scope {
						locals: HashMap::new(),
						inst: self.scope.borrow().inst.clone(),
						outer: Some(self.scope.clone()),
					};
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
							let mut scope = Scope {
								locals: HashMap::new(),
								inst: proc.inst.clone(),
								outer: Some(proc.scope.clone()),
							};
							for (arg, param) in args.into_iter().zip(proc.params.iter()) {
								scope.assign(*param, arg);
							}
							match self.eval_exprs(chunk, scope, &proc.body) {
								Ok(val) => Ok(val),
								Err(Signal::Return(val)) => Ok(val),
								Err(err) => Err(err),
							}
						}
						Obj::Type(type_id) => {
							let typ = self.types.get_type(*type_id);
							if args.len() != typ.fields.len() {
								let src = self.pkg.get_src(chunk.src);
								let loc = src.loc(tok.idx);
								return Err(Signal::Error(Error::WrongArgCount(
									loc,
									args.len(),
									typ.fields.len(),
								)));
							}
							let mut fields = HashMap::new();
							for (field, val) in typ.fields.iter().zip(args.into_iter()) {
								fields.insert(*field, val);
							}
							let obj = Obj::Inst(Inst {
								typ: *type_id,
								fields,
							});
							let val = Val::Ref(Ref::new(obj));
							Ok(val)
						}
						obj => {
							let src = self.pkg.get_src(chunk.src);
							let loc = src.loc(tok.idx);
							Err(Signal::Error(Error::CallNonCallable(
								loc,
								rt_debug(self.syms, &self.types, &Val::Ref(rf.clone())),
							)))
						}
					},
					val => {
						let src = self.pkg.get_src(chunk.src);
						let loc = src.loc(tok.idx);
						Err(Signal::Error(Error::CallNonCallable(
							loc,
							rt_debug(self.syms, &self.types, &val),
						)))
					}
				}
			}
			Expr::Access(Access(tok, val_id, name)) => match self.eval_expr(chunk, *val_id)? {
				Val::Ref(rf) => match &*rf.get() {
					Obj::Inst(inst) => match inst.fields.get(name) {
						Some(val) => Ok(val.clone()),
						None => {
							let typ = self.types.get_type(inst.typ);
							match typ.methods.get(name) {
								Some(proc_rf) => {
									let proc = proc_rf.borrow();
									Ok(Val::Ref(Ref::new(Obj::Proc(Proc {
										name: proc.name,
										params: proc.params.clone(),
										body: proc.body.clone(),
										scope: proc.scope.clone(),
										inst: Some(rf.clone()),
									}))))
								}
								None => {
									let src = self.pkg.get_src(chunk.src);
									let loc = src.loc(tok.idx);
									Err(Signal::Error(Error::AccessNonMember(
										loc,
										self.syms.get_by_id(typ.name).1.to_string(),
										self.syms.get_by_id(*name).1.to_string(),
									)))
								}
							}
						}
					},
					_ => todo!(),
				},
				_ => todo!(),
			},
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
									rt_debug(self.syms, &self.types, &val),
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
							rt_debug(self.syms, &self.types, &Val::Ref(rf.clone())),
						)))
					}
				},
				val => {
					let src = self.pkg.get_src(chunk.src);
					let loc = src.loc(tok.idx);
					Err(Signal::Error(Error::ScriptNonScriptable(
						loc,
						rt_debug(self.syms, &self.types, &val),
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
							Obj::Type(_) => "Type",
							Obj::Inst(inst) => {
								let typ = self.types.get_type(inst.typ);
								let name = self.syms.get_by_id(typ.name);
								name.1.as_str()
							}
						},
						Val::Nil => "Nil",
					};
					println!("({})\t{}", desc, rt_debug(self.syms, &self.types, &val));
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

fn rt_debug(syms: &Interner, types: &Types, val: &Val) -> String {
	match val {
		Val::Num(num) => format!("{}", num),
		Val::Bool(bool) => format!("{}", bool),
		Val::Ref(rf) => match &*rf.get() {
			Obj::Str(str) => format!("\"{}\"", str),
			Obj::List(items) => {
				let mut res = String::new();
				res.push('[');
				for (i, item) in items.iter().enumerate() {
					res.push_str(&rt_debug(syms, types, item));
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
					res.push_str(&rt_debug(syms, types, key));
					res.push_str(": ");
					res.push_str(&rt_debug(syms, types, val));
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
			Obj::Type(id) => {
				let typ = types.get_type(*id);
				let name = syms.get_by_id(typ.name);
				let mut res = String::new();
				res.push_str(&format!("type {}(", name.1));
				for (i, field) in typ.fields.iter().enumerate() {
					let field = syms.get_by_id(*field);
					res.push_str(&field.1);
					if i + 1 != typ.fields.len() {
						res.push_str(", ");
					}
				}
				res.push(')');
				res
			}
			Obj::Inst(inst) => {
				let typ = types.get_type(inst.typ);
				let name = syms.get_by_id(typ.name);
				let mut res = String::new();
				res.push_str(&format!("{}(", name.1));
				for (i, (_, field)) in inst.fields.iter().enumerate() {
					res.push_str(&rt_debug(syms, types, field));
					if i + 1 != inst.fields.len() {
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
