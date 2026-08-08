use std::cell::RefCell;
use std::cmp::{Eq, PartialEq};
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::intern::{Interner, Sym};
use crate::syn::{
	self, Access, Assign, Binary, BinaryOp, BlockId, Builtin, Call, Chunk, Decl, DeclId, Def, Each,
	Expr, ExprId, Ident, Lit, Location, Package, Place, Return, Script, Token, When,
};

#[derive(Debug)]
pub enum Error {
	WrongArgCount(Location, usize, usize),
	CallNonCallable(Location, String),
	IterNonIterable(Location, String),
	AccessNonMember(Location, String, String),
	ScriptNonScriptable(Location, String),
	ScriptNonIndex(Location, String),
	ArithNonNum(Location, String),
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
			Self::ArithNonNum(loc, _) => loc,
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
			Self::ArithNonNum(_, val) => write!(f, "arithmetic on non-number {}", val),
			Self::UnboundIdent(_, ident) => write!(f, "unbound ident '{}'", ident),
		}
	}
}

#[derive(Debug, Clone)]
enum Val {
	Num(f64),
	Bool(bool),
	Obj(Rc<RefCell<Obj>>),
	Nil,
}

impl PartialEq for Val {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Num(num), Self::Num(other_num)) => num == other_num,
			(Self::Bool(bool), Self::Bool(other_bool)) => bool == other_bool,
			(Self::Obj(rf), Self::Obj(other_rf)) => match (&*rf.borrow(), &*other_rf.borrow()) {
				(Obj::Str(str), Obj::Str(other_str)) => str.chars == other_str.chars,
				_ => rf.as_ptr() == other_rf.as_ptr(),
			},
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
			Self::Obj(rf) => match &*rf.borrow() {
				Obj::Str(str) => str.chars.hash(state),
				_ => rf.as_ptr().hash(state),
			},
			Self::Nil => 0_u8.hash(state),
		}
	}
}

#[derive(Debug)]
enum Obj {
	Str(Str),
	List(List),
	Dict(Dict),
	Proc(Proc),
	Type(TypeId),
	Instance(Instance),
	Method(Method),
}

#[derive(Debug)]
struct Str {
	chars: Box<str>,
}

#[derive(Debug)]
struct List {
	items: Vec<Val>,
}

#[derive(Debug)]
struct Dict {
	pairs: HashMap<Val, Val>,
}

#[derive(Debug)]
struct Proc {
	name: Sym,
	params: Vec<Sym>,
	body: BlockId,
	scope: Rc<RefCell<Scope>>,
}

#[derive(Debug, Clone, Copy)]
struct TypeId(u32);

impl TypeId {
	const NIL: TypeId = TypeId(0);
	const NUM: TypeId = TypeId(1);
	const BOOL: TypeId = TypeId(2);
	const STR: TypeId = TypeId(3);
	const LIST: TypeId = TypeId(4);
	const DICT: TypeId = TypeId(5);
	const PROC: TypeId = TypeId(6);
	const TYPE: TypeId = TypeId(7);
}

const CORE_TYPES: &[(&str, TypeId)] = &[
	("Nil", TypeId::NIL),
	("Num", TypeId::NUM),
	("Bool", TypeId::BOOL),
	("Str", TypeId::STR),
	("List", TypeId::LIST),
	("Dict", TypeId::DICT),
	("Proc", TypeId::PROC),
	("Type", TypeId::TYPE),
];

#[derive(Debug)]
struct Type {
	name: Sym,
	fields: Vec<Sym>,
	methods: HashMap<Sym, Rc<RefCell<Proc>>>,
}

struct TypeRegistry {
	types: Vec<Type>,
}

impl TypeRegistry {
	fn new(syms: &mut Interner) -> Self {
		let mut types = Vec::with_capacity(CORE_TYPES.len());

		for (name, _id) in CORE_TYPES {
			types.push(Type {
				name: syms.intern(name),
				fields: Vec::new(),
				methods: HashMap::new(),
			});
		}

		Self { types }
	}

	fn get_type(&self, id: TypeId) -> &Type {
		&self.types[id.0 as usize]
	}

	fn add_type(&mut self, typ: Type) -> TypeId {
		let id = TypeId(self.types.len() as u32);
		self.types.push(typ);
		id
	}
}

#[derive(Debug)]
struct Instance {
	typ: TypeId,
	fields: HashMap<Sym, Val>,
}

#[derive(Debug)]
struct Member {
	name: Sym,
	inst: Rc<RefCell<Obj>>,
}

#[derive(Debug)]
struct Method {
	proc: Rc<RefCell<Proc>>,
	inst: Rc<RefCell<Obj>>,
}

impl Instance {
	fn member(obj: Rc<RefCell<Obj>>, name: Sym, types: &TypeRegistry) -> Option<Member> {
		if let Obj::Instance(inst) = &*obj.borrow() {
			let typ = types.get_type(inst.typ);
			if inst.fields.contains_key(&name) || typ.methods.contains_key(&name) {
				return Some(Member {
					name,
					inst: obj.clone(),
				});
			}
		}
		None
	}
}

impl Member {
	fn get(&self, types: &TypeRegistry) -> Val {
		let Obj::Instance(inst) = &*self.inst.borrow() else {
			panic!();
		};
		let typ = types.get_type(inst.typ);
		if let Some(val) = inst.fields.get(&self.name) {
			val.clone()
		} else if let Some(proc_rf) = typ.methods.get(&self.name) {
			Val::Obj(Rc::new(RefCell::new(Obj::Method(Method {
				proc: proc_rf.clone(),
				inst: self.inst.clone(),
			}))))
		} else {
			panic!();
		}
	}

	fn set(&self, val: Val) {
		let Obj::Instance(inst) = &mut *self.inst.borrow_mut() else {
			panic!();
		};
		inst.fields.insert(self.name, val);
	}
}

#[derive(Debug)]
struct Scope {
	locals: HashMap<Sym, Val>,
	outer: Option<Rc<RefCell<Scope>>>,
}

struct Local {
	name: Sym,
	scope: Rc<RefCell<Scope>>,
}

impl Local {
	fn get(&self) -> Val {
		self.scope.borrow().locals.get(&self.name).cloned().unwrap()
	}

	fn set(&self, val: Val) {
		self.scope.borrow_mut().locals.insert(self.name, val);
	}
}

impl Scope {
	fn local(scope: &Rc<RefCell<Scope>>, name: Sym) -> Option<Local> {
		if scope.borrow().locals.contains_key(&name) {
			Some(Local {
				name,
				scope: scope.clone(),
			})
		} else if let Some(outer) = &scope.borrow().outer {
			Scope::local(outer, name)
		} else {
			None
		}
	}
}

pub struct Interpreter<'syms, 'pkg> {
	syms: &'syms Interner,
	pkg: &'pkg Package,
	types: TypeRegistry,
	scope: Rc<RefCell<Scope>>,
	inst: Option<Rc<RefCell<Obj>>>,
}

enum Signal {
	Return(Val),
	Error(Error),
}

impl<'syms, 'pkg> Interpreter<'syms, 'pkg> {
	pub fn new(syms: &'syms mut Interner, pkg: &'pkg Package) -> Self {
		let types = TypeRegistry::new(syms);
		Self {
			syms,
			pkg,
			types,
			scope: Rc::new(RefCell::new(Scope {
				locals: HashMap::new(),
				outer: None,
			})),
			inst: None,
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
			Decl::Type(syn::Type(name, fields, items)) => {
				let mut methods = HashMap::new();
				for decl_id in items {
					match chunk.get_decl(*decl_id) {
						Decl::Type(syn::Type(_, _, _)) => todo!(),
						Decl::Def(Def(name, params, body)) => {
							let proc = Proc {
								name: *name,
								params: params.to_vec(),
								body: *body,
								scope: self.scope.clone(),
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
				let val = Val::Obj(Rc::new(RefCell::new(obj)));
				self.scope.borrow_mut().locals.insert(*name, val);
				Ok(())
			}
			Decl::Def(Def(name, params, body)) => {
				let obj = Obj::Proc(Proc {
					name: *name,
					params: params.to_vec(),
					body: *body,
					scope: self.scope.clone(),
				});
				let val = Val::Obj(Rc::new(RefCell::new(obj)));
				self.scope.borrow_mut().locals.insert(*name, val);
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
			Expr::Each(Each(name, iter, body_id)) => match self.eval_expr(chunk, *iter)? {
				Val::Obj(rf) => match &*rf.borrow() {
					Obj::List(list) => {
						let outer_scope = self.scope.clone();
						let inner_scope = Scope {
							locals: HashMap::new(),
							outer: Some(outer_scope.clone()),
						};
						self.scope = Rc::new(RefCell::new(inner_scope));
						let body = chunk.get_block(*body_id);
						for item in &list.items {
							self.scope.borrow_mut().locals.insert(*name, item.clone());
							for expr_id in &body.0 {
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
						let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
						Err(Signal::Error(Error::IterNonIterable(
							loc,
							format!("{:?}", obj),
						)))
					}
				},
				val => {
					let src = self.pkg.get_src(chunk.src);
					let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
					Err(Signal::Error(Error::IterNonIterable(
						loc,
						format!("{:?}", val),
					)))
				}
			},
			Expr::When(When(cond, then_branch, else_branch)) => {
				let cond = match self.eval_expr(chunk, *cond)? {
					Val::Bool(true) => true,
					Val::Num(num) if num > 0.0 => true,
					Val::Obj(_) => true,
					_ => false,
				};
				let scope = Rc::new(RefCell::new(Scope {
					locals: HashMap::new(),
					outer: Some(self.scope.clone()),
				}));
				if cond {
					self.eval_block(chunk, scope, *then_branch)
				} else if let Some(else_branch) = else_branch {
					self.eval_block(chunk, scope, *else_branch)
				} else {
					Ok(Val::Nil)
				}
			}
			Expr::Return(Return(val_expr_id)) => {
				let val = self.eval_expr(chunk, *val_expr_id)?;
				Err(Signal::Return(val))
			}
			Expr::Call(Call(val_id, arg_ids)) => {
				let tok = chunk.get_expr_tok(expr_id);
				let mut args = Vec::with_capacity(arg_ids.len());
				for arg_id in arg_ids {
					let arg = self.eval_expr(chunk, *arg_id)?;
					args.push(arg);
				}
				match self.eval_expr(chunk, *val_id)? {
					Val::Obj(rf) => match &*rf.borrow() {
						Obj::Proc(proc) => self.eval_proc_call(chunk, &tok, proc, None, args),
						Obj::Type(type_id) => {
							let typ = self.types.get_type(*type_id);
							if args.len() != typ.fields.len() {
								let src = self.pkg.get_src(chunk.src);
								let loc = src.loc(tok.pos);
								return Err(Signal::Error(Error::WrongArgCount(
									loc,
									args.len(),
									typ.fields.len(),
								)));
							}
							let mut fields = HashMap::new();
							for (field, val) in typ.fields.iter().zip(args) {
								fields.insert(*field, val);
							}
							let obj = Obj::Instance(Instance {
								typ: *type_id,
								fields,
							});
							let val = Val::Obj(Rc::new(RefCell::new(obj)));
							Ok(val)
						}
						Obj::Method(meth) => self.eval_proc_call(
							chunk,
							&tok,
							&meth.proc.borrow(),
							Some(meth.inst.clone()),
							args,
						),
						_ => {
							let src = self.pkg.get_src(chunk.src);
							let loc = src.loc(tok.pos);
							Err(Signal::Error(Error::CallNonCallable(
								loc,
								rt_print_val(self.syms, &self.types, &Val::Obj(rf.clone())),
							)))
						}
					},
					val => {
						let src = self.pkg.get_src(chunk.src);
						let loc = src.loc(tok.pos);
						Err(Signal::Error(Error::CallNonCallable(
							loc,
							rt_print_val(self.syms, &self.types, &val),
						)))
					}
				}
			}
			Expr::Access(Access(val_id, name)) => {
				let val = self.eval_expr(chunk, *val_id)?;
				let typ = self.type_of(&val);

				if let Val::Obj(obj_rf) = val
					&& let Some(member) = Instance::member(obj_rf.clone(), *name, &self.types)
				{
					Ok(member.get(&self.types))
				} else {
					let src = self.pkg.get_src(chunk.src);
					let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
					Err(Signal::Error(Error::AccessNonMember(
						loc,
						self.syms.resolve(typ.name).to_string(),
						self.syms.resolve(*name).to_string(),
					)))
				}
			}
			Expr::Script(Script(val_id, key_id)) => match self.eval_expr(chunk, *val_id)? {
				Val::Obj(rf) => match &*rf.borrow() {
					Obj::List(list) => {
						let idx = match self.eval_expr(chunk, *key_id)? {
							Val::Num(num) if num >= 0.0 => num.trunc() as usize,
							val => {
								let src = self.pkg.get_src(chunk.src);
								let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
								return Err(Signal::Error(Error::ScriptNonIndex(
									loc,
									rt_print_val(self.syms, &self.types, &val),
								)));
							}
						};
						match list.items.get(idx) {
							Some(val) => Ok(val.clone()),
							None => Ok(Val::Nil),
						}
					}
					Obj::Dict(dict) => {
						let key = self.eval_expr(chunk, *key_id)?;
						match dict.pairs.get(&key) {
							Some(val) => Ok(val.clone()),
							None => Ok(Val::Nil),
						}
					}
					_ => {
						let src = self.pkg.get_src(chunk.src);
						let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
						Err(Signal::Error(Error::ScriptNonScriptable(
							loc,
							rt_print_val(self.syms, &self.types, &Val::Obj(rf.clone())),
						)))
					}
				},
				val => {
					let src = self.pkg.get_src(chunk.src);
					let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
					Err(Signal::Error(Error::ScriptNonScriptable(
						loc,
						rt_print_val(self.syms, &self.types, &val),
					)))
				}
			},
			Expr::Assign(Assign(place, val_expr_id)) => {
				let val = self.eval_expr(chunk, *val_expr_id)?;
				match place {
					Place::Ident(Ident(sym)) => {
						if let Some(rf) = &self.inst
							&& let Some(member) = Instance::member(rf.clone(), *sym, &self.types)
						{
							member.set(val.clone());
						} else if let Some(local) = Scope::local(&self.scope, *sym) {
							local.set(val.clone());
						} else {
							self.scope.borrow_mut().locals.insert(*sym, val.clone());
						}
						Ok(val)
					}
					Place::Member(syn::Member(target_id, name)) => {
						let target = self.eval_expr(chunk, *target_id)?;

						if let Val::Obj(rf) = &target
							&& let Some(member) = Instance::member(rf.clone(), *name, &self.types)
						{
							member.set(val.clone());
							return Ok(val);
						}

						let typ = self.type_of(&target);
						let src = self.pkg.get_src(chunk.src);
						let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
						Err(Signal::Error(Error::AccessNonMember(
							loc,
							self.syms.resolve(typ.name).to_string(),
							self.syms.resolve(*name).to_string(),
						)))
					}
				}
			}
			Expr::Binary(Binary(op, lhs_id, rhs_id)) => {
				let lhs = self.eval_expr(chunk, *lhs_id)?;
				let rhs = self.eval_expr(chunk, *rhs_id)?;
				match (lhs, rhs) {
					(Val::Num(lhs), Val::Num(rhs)) => Ok(Val::Num(match op {
						BinaryOp::Add => lhs + rhs,
						BinaryOp::Sub => lhs - rhs,
						BinaryOp::Mul => lhs * rhs,
						BinaryOp::Div => lhs / rhs,
					})),
					(lhs, rhs) => {
						let src = self.pkg.get_src(chunk.src);
						let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
						let val = match (&lhs, &rhs) {
							(Val::Num(_), _) => rhs,
							(_, Val::Num(_)) => lhs,
							_ => lhs,
						};
						Err(Signal::Error(Error::ArithNonNum(
							loc,
							rt_print_val(self.syms, &self.types, &val),
						)))
					}
				}
			}
			Expr::Ident(Ident(name)) => {
				if *name == Sym::SELF
					&& let Some(inst) = &self.inst
				{
					Ok(Val::Obj(inst.clone()))
				} else if let Some(local) = Scope::local(&self.scope, *name) {
					Ok(local.get())
				} else if let Some(rf) = &self.inst
					&& let Some(member) = Instance::member(rf.clone(), *name, &self.types)
				{
					Ok(member.get(&self.types))
				} else {
					let src = self.pkg.get_src(chunk.src);
					let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
					let name = self.syms.resolve(*name);
					Err(Signal::Error(Error::UnboundIdent(loc, name.to_string())))
				}
			}
			Expr::Builtin(builtin) => match builtin {
				Builtin::Print(val_id) => {
					let val = self.eval_expr(chunk, *val_id)?;
					println!("{}", rt_print_val(self.syms, &self.types, &val));
					Ok(val)
				}
			},
			Expr::Lit(lit) => Ok(match lit {
				Lit::Str(str) => Val::Obj(Rc::new(RefCell::new(Obj::Str(Str {
					chars: Box::from(str.as_str()),
				})))),
				Lit::Num(num) => Val::Num(*num),
				Lit::Bool(bool) => Val::Bool(*bool),
				Lit::List(item_ids) => {
					let mut items = Vec::with_capacity(item_ids.len());
					for item_id in item_ids {
						let item = self.eval_expr(chunk, *item_id)?;
						items.push(item);
					}
					Val::Obj(Rc::new(RefCell::new(Obj::List(List { items }))))
				}
				Lit::Dict(pair_ids) => {
					let mut pairs = HashMap::with_capacity(pair_ids.len());
					for (key_id, val_id) in pair_ids {
						let key = self.eval_expr(chunk, *key_id)?;
						let val = self.eval_expr(chunk, *val_id)?;
						pairs.insert(key, val);
					}
					Val::Obj(Rc::new(RefCell::new(Obj::Dict(Dict { pairs }))))
				}
				Lit::Nil => Val::Nil,
			}),
		}
	}

	fn eval_proc_call(
		&mut self,
		chunk: &Chunk,
		tok: &Token,
		proc: &Proc,
		inst: Option<Rc<RefCell<Obj>>>,
		args: Vec<Val>,
	) -> Result<Val, Signal> {
		if args.len() != proc.params.len() {
			let src = self.pkg.get_src(chunk.src);
			let loc = src.loc(tok.pos);
			return Err(Signal::Error(Error::WrongArgCount(
				loc,
				args.len(),
				proc.params.len(),
			)));
		}
		let mut scope = Scope {
			locals: HashMap::new(),
			outer: Some(proc.scope.clone()),
		};
		for (arg, param) in args.into_iter().zip(proc.params.iter()) {
			scope.locals.insert(*param, arg);
		}
		let saved_inst = self.inst.clone();
		self.inst = inst;
		let result = self.eval_block(chunk, Rc::new(RefCell::new(scope)), proc.body);
		self.inst = saved_inst;
		match result {
			Ok(val) => Ok(val),
			Err(Signal::Return(val)) => Ok(val),
			Err(err) => Err(err),
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
		let mut res = Val::Nil;
		for expr_id in &block.0 {
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

	fn type_of(&self, val: &Val) -> &Type {
		match val {
			Val::Nil => self.types.get_type(TypeId::NIL),
			Val::Num(_) => self.types.get_type(TypeId::NUM),
			Val::Bool(_) => self.types.get_type(TypeId::BOOL),
			Val::Obj(rf) => match &*rf.borrow() {
				Obj::Str(_) => self.types.get_type(TypeId::STR),
				Obj::List(_) => self.types.get_type(TypeId::LIST),
				Obj::Dict(_) => self.types.get_type(TypeId::DICT),
				Obj::Proc(_) => self.types.get_type(TypeId::PROC),
				Obj::Type(_) => self.types.get_type(TypeId::TYPE),
				Obj::Instance(inst) => self.types.get_type(inst.typ),
				Obj::Method(_) => self.types.get_type(TypeId::PROC),
			},
		}
	}
}

fn rt_print_val(syms: &Interner, types: &TypeRegistry, val: &Val) -> String {
	match val {
		Val::Num(num) => format!("{}", num),
		Val::Bool(bool) => format!("{}", bool),
		Val::Obj(rf) => rt_print_obj(syms, types, &rf.borrow()),
		Val::Nil => String::from("nil"),
	}
}
fn rt_print_obj(syms: &Interner, types: &TypeRegistry, obj: &Obj) -> String {
	match obj {
		Obj::Str(str) => format!("{}", str.chars),
		Obj::List(list) => {
			let mut res = String::new();
			res.push('[');
			for (i, item) in list.items.iter().enumerate() {
				res.push_str(&rt_print_val(syms, types, item));
				if i + 1 != list.items.len() {
					res.push_str(", ");
				}
			}
			res.push(']');
			res
		}
		Obj::Dict(dict) => {
			let mut res = String::new();
			res.push('{');
			for (i, (key, val)) in dict.pairs.iter().enumerate() {
				res.push_str(&rt_print_val(syms, types, key));
				res.push_str(": ");
				res.push_str(&rt_print_val(syms, types, val));
				if i + 1 != dict.pairs.len() {
					res.push_str(", ");
				}
			}
			res.push('}');
			res
		}
		Obj::Proc(proc) => rt_print_proc(syms, proc),
		Obj::Type(id) => {
			let typ = types.get_type(*id);
			let name = syms.resolve(typ.name);
			let mut res = String::new();
			res.push_str(&format!("type {}(", name));
			for (i, field) in typ.fields.iter().enumerate() {
				let field = syms.resolve(*field);
				res.push_str(field);
				if i + 1 != typ.fields.len() {
					res.push_str(", ");
				}
			}
			res.push(')');
			res
		}
		Obj::Instance(inst) => {
			let typ = types.get_type(inst.typ);
			let name = syms.resolve(typ.name);
			let mut res = String::new();
			res.push_str(&format!("{}(", name));
			for (i, name) in typ.fields.iter().enumerate() {
				let val = inst.fields.get(name).unwrap();
				res.push_str(&rt_print_val(syms, types, val));
				if i + 1 != typ.fields.len() {
					res.push_str(", ");
				}
			}
			res.push(')');
			res
		}
		Obj::Method(meth) => rt_print_proc(syms, &meth.proc.borrow()),
	}
}

fn rt_print_proc(syms: &Interner, proc: &Proc) -> String {
	let mut res = String::new();
	let name = syms.resolve(proc.name);
	res.push_str(&format!("def {}(", name));
	for (i, param) in proc.params.iter().enumerate() {
		let param = syms.resolve(*param);
		res.push_str(param);
		if i + 1 != proc.params.len() {
			res.push_str(", ");
		}
	}
	res.push(')');
	res
}
