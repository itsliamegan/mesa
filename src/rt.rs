use std::cell::RefCell;
use std::cmp::{Eq, PartialEq};
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use ordermap::OrderMap;

use crate::intern::{Interner, Sym};
use crate::syn::{
	self, Assign, Binary, BinaryOp, BlockId, Builtin, Call, Chunk, Def, Each, Expr, ExprId, Lit,
	Location, ModuleItem, ModuleItemId, Name, Package, Place, Return, Script, Token, TypeItem,
	Unary, UnaryOp, When,
};

#[derive(Debug)]
pub enum Error {
	ProtocolError(ProtocolError),
	ArgumentError(ArgumentError),
	TypeError(TypeError),
	MemberError(MemberError),
	IndexError(IndexError),
	NameError(String),
	KeyError(String),
}

#[derive(Debug)]
pub enum ProtocolError {
	NotIterable(String),
	NotScriptable(String),
}

#[derive(Debug)]
pub enum ArgumentError {
	WrongCount(usize, usize),
}

#[derive(Debug)]
pub enum TypeError {
	IndexNonNum(String),
	ArithNonNum(String),
	NotCallable(String),
	NotInvokable(String),
	NotOrderable(String),
}

#[derive(Debug)]
pub enum MemberError {
	Missing(String, String),
	ReadOnly(String, String),
}

#[derive(Debug)]
pub enum IndexError {
	OutOfRange(usize),
	NonIntegral(f64),
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::ProtocolError(err) => Display::fmt(err, f),
			Self::ArgumentError(err) => Display::fmt(err, f),
			Self::TypeError(err) => Display::fmt(err, f),
			Self::MemberError(err) => Display::fmt(err, f),
			Self::IndexError(err) => Display::fmt(err, f),
			Self::NameError(name) => write!(f, "name '{}' is not defined", name),
			Self::KeyError(key) => write!(f, "key {} not found", key),
		}
	}
}

impl Display for ProtocolError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::NotIterable(type_name) => write!(f, "type {} is not iterable", type_name),
			Self::NotScriptable(type_name) => write!(f, "type {} is not scriptable", type_name),
		}
	}
}

impl Display for ArgumentError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::WrongCount(have, want) => {
				write!(f, "wrong number of args; have {}, want {}", have, want)
			}
		}
	}
}

impl Display for TypeError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::IndexNonNum(val) => write!(f, "index {} is not a number", val),
			Self::ArithNonNum(type_name) => {
				write!(f, "type {} cannot be used in arithmetic", type_name)
			}
			Self::NotCallable(type_name) => write!(f, "type {} is not callable", type_name),
			Self::NotInvokable(type_name) => write!(f, "type {} is not invokable", type_name),
			Self::NotOrderable(type_name) => write!(f, "type {} is not orderable", type_name),
		}
	}
}

impl Display for MemberError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::Missing(type_name, name) => {
				write!(f, "type {} has no such member '{}'", type_name, name)
			}
			Self::ReadOnly(type_name, name) => {
				write!(f, "member '{}' on type {} is read-only", name, type_name)
			}
		}
	}
}

impl Display for IndexError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::OutOfRange(idx) => write!(f, "index {} is out of bounds", idx),
			Self::NonIntegral(idx) => write!(f, "index {} is not a whole number", idx),
		}
	}
}

#[derive(Debug, Clone, Copy)]
struct Num(f64);

impl Num {
	fn new() -> Val {
		Val::Num(Num(0.0))
	}
}

#[derive(Debug, Clone, Copy)]
struct Bool(bool);

impl Bool {
	fn new() -> Val {
		Val::Bool(Bool(false))
	}
}

#[derive(Debug, Clone, Copy)]
struct Nil;

impl Nil {
	fn new() -> Val {
		Val::Nil(Nil)
	}
}

#[derive(Debug, Clone)]
enum Val {
	Num(Num),
	Bool(Bool),
	Obj(Rc<RefCell<Obj>>),
	Nil(Nil),
}

impl Val {
	fn type_id(&self) -> TypeId {
		match self {
			Val::Num(_) => TypeId::Native(NativeTypeId::NUM),
			Val::Bool(_) => TypeId::Native(NativeTypeId::BOOL),
			Val::Obj(rf) => rf.borrow().type_id(),
			Self::Nil(_) => TypeId::Native(NativeTypeId::NIL),
		}
	}

	fn namespace_type_id(&self) -> TypeId {
		match self {
			Val::Obj(rf) => match &*rf.borrow() {
				Obj::Type(id) => *id,
				obj => obj.type_id(),
			},
			val => val.type_id(),
		}
	}

	fn is_truthy(&self) -> bool {
		match self {
			Val::Bool(bool) => bool.0,
			Val::Nil(_) => false,
			_ => true,
		}
	}

	fn member(&self, name: Sym, types: &TypeRegistry) -> Option<Member> {
		let type_id = self.type_id();
		let namespace_type_id = self.namespace_type_id();
		if type_id != namespace_type_id {
			// type_id and namespace_type_id only ever differ when the Val is
			// itself a Type.
			let Val::Obj(rf) = self else {
				panic!();
			};
			let has_static = match namespace_type_id {
				TypeId::User(id) => types.get_user_type(id).get_static(name).is_some(),
				TypeId::Native(id) => types.get_native_type(id).get_static(name).is_some(),
			};
			if has_static {
				Some(Member::Static(rf.clone(), name))
			} else {
				let TypeId::Native(id) = type_id else {
					panic!();
				};
				if types.get_native_type(id).has_member(name) {
					Some(Member::Native(self.clone(), name))
				} else {
					None
				}
			}
		} else {
			match namespace_type_id {
				TypeId::User(id) => {
					let Val::Obj(rf) = self else {
						panic!();
					};
					if types.get_user_type(id).has_member(name) {
						Some(Member::User(rf.clone(), name))
					} else {
						None
					}
				}
				TypeId::Native(id) => {
					if types.get_native_type(id).has_member(name) {
						Some(Member::Native(self.clone(), name))
					} else {
						None
					}
				}
			}
		}
	}
}

impl PartialEq for Val {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Num(num), Self::Num(other_num)) => num.0 == other_num.0,
			(Self::Bool(bool), Self::Bool(other_bool)) => bool.0 == other_bool.0,
			(Self::Obj(rf), Self::Obj(other_rf)) => match (&*rf.borrow(), &*other_rf.borrow()) {
				(Obj::Str(str), Obj::Str(other_str)) => str.chars == other_str.chars,
				_ => rf.as_ptr() == other_rf.as_ptr(),
			},
			(Self::Nil(_), Self::Nil(_)) => true,
			_ => false,
		}
	}
}

impl Eq for Val {}

impl Hash for Val {
	fn hash<H: Hasher>(&self, state: &mut H) {
		match self {
			Self::Num(num) => num.0.to_bits().hash(state),
			Self::Bool(bool) => bool.0.hash(state),
			Self::Obj(rf) => match &*rf.borrow() {
				Obj::Str(str) => str.chars.hash(state),
				_ => rf.as_ptr().hash(state),
			},
			Self::Nil(_) => 0_u8.hash(state),
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

impl Obj {
	fn type_id(&self) -> TypeId {
		match self {
			Self::Str(_) => TypeId::Native(NativeTypeId::STR),
			Self::List(_) => TypeId::Native(NativeTypeId::LIST),
			Self::Dict(_) => TypeId::Native(NativeTypeId::DICT),
			Self::Proc(_) => TypeId::Native(NativeTypeId::PROC),
			Self::Type(_) => TypeId::Native(NativeTypeId::TYPE),
			Self::Instance(inst) => TypeId::User(inst.typ),
			Self::Method(_) => TypeId::Native(NativeTypeId::PROC),
		}
	}
}

#[derive(Debug)]
struct Str {
	chars: Box<str>,
}

impl Str {
	fn new() -> Val {
		Val::Obj(Rc::new(RefCell::new(Obj::Str(Str {
			chars: Box::from(""),
		}))))
	}

	fn size(val: &Val) -> Val {
		let Val::Obj(rf) = val else { panic!() };
		let Obj::Str(str) = &*rf.borrow() else {
			panic!()
		};
		Val::Num(Num(str.chars.len() as f64))
	}
}

#[derive(Debug)]
struct List {
	items: Vec<Val>,
}

impl List {
	fn new() -> Val {
		Val::Obj(Rc::new(RefCell::new(Obj::List(List { items: Vec::new() }))))
	}

	fn size(val: &Val) -> Val {
		let Val::Obj(rf) = val else { panic!() };
		let Obj::List(list) = &*rf.borrow() else {
			panic!()
		};
		Val::Num(Num(list.items.len() as f64))
	}
}

#[derive(Debug)]
struct Dict {
	pairs: OrderMap<Val, Val>,
}

impl Dict {
	fn new() -> Val {
		Val::Obj(Rc::new(RefCell::new(Obj::Dict(Dict {
			pairs: OrderMap::new(),
		}))))
	}

	fn size(val: &Val) -> Val {
		let Val::Obj(rf) = val else { panic!() };
		let Obj::Dict(dict) = &*rf.borrow() else {
			panic!()
		};
		Val::Num(Num(dict.pairs.len() as f64))
	}
}

#[derive(Debug)]
struct Proc {
	name: Sym,
	params: Vec<Sym>,
	body: BlockId,
	scope: Rc<RefCell<Scope>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeId {
	User(UserTypeId),
	Native(NativeTypeId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UserTypeId(u32);

#[derive(Debug)]
struct UserType {
	name: Sym,
	ctor_fields: Vec<Sym>,
	body_fields: Vec<(Sym, ExprId)>,
	methods: HashMap<Sym, Rc<RefCell<Proc>>>,
	statics: HashMap<Sym, Static>,
	scope: Rc<RefCell<Scope>>,
}

impl UserType {
	fn has_member(&self, name: Sym) -> bool {
		self.ctor_fields.contains(&name)
			|| self.body_fields.iter().any(|(field, _)| *field == name)
			|| self.methods.contains_key(&name)
	}

	fn get_static(&self, name: Sym) -> Option<Static> {
		self.statics.get(&name).cloned()
	}
}

#[derive(Debug, Clone)]
enum Static {
	Proc(Rc<RefCell<Proc>>),
	Type(Val),
	NativeMethod(NativeMethod),
	NativeField(NativeField),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeTypeId(u32);

#[derive(Debug, Clone)]
struct NativeField {
	get: fn(&Val) -> Val,
}

#[derive(Debug, Clone)]
struct NativeMethod {
	arity: usize,
	call: fn(&Val, Vec<Val>) -> Val,
}

#[derive(Debug)]
struct NativeType {
	name: Sym,
	new: Option<fn() -> Val>,
	fields: HashMap<Sym, NativeField>,
	methods: HashMap<Sym, NativeMethod>,
	statics: HashMap<Sym, Static>,
}

impl NativeType {
	fn has_member(&self, name: Sym) -> bool {
		self.fields.contains_key(&name) || self.methods.contains_key(&name)
	}

	fn get_static(&self, name: Sym) -> Option<Static> {
		self.statics.get(&name).cloned()
	}
}

const CORE_TYPES: &[(
	&str,
	NativeTypeId,
	Option<fn() -> Val>,
	&[(&str, fn(&Val) -> Val)],
	&[(&str, fn(&Val, Vec<Val>) -> Val, usize)],
)] = &[
	("Nil", NativeTypeId::NIL, Some(Nil::new), &[], &[]),
	("Num", NativeTypeId::NUM, Some(Num::new), &[], &[]),
	("Bool", NativeTypeId::BOOL, Some(Bool::new), &[], &[]),
	(
		"Str",
		NativeTypeId::STR,
		Some(Str::new),
		&[("size", Str::size)],
		&[],
	),
	(
		"List",
		NativeTypeId::LIST,
		Some(List::new),
		&[("size", List::size)],
		&[],
	),
	(
		"Dict",
		NativeTypeId::DICT,
		Some(Dict::new),
		&[("size", Dict::size)],
		&[],
	),
	("Proc", NativeTypeId::PROC, None, &[], &[]),
	("Type", NativeTypeId::TYPE, None, &[], &[]),
];

pub(crate) const CORE_TYPE_NAMES: [&str; CORE_TYPES.len()] = {
	let mut names = [""; CORE_TYPES.len()];
	let mut i = 0;
	while i < CORE_TYPES.len() {
		names[i] = CORE_TYPES[i].0;
		i += 1;
	}
	names
};

impl NativeTypeId {
	const NIL: NativeTypeId = NativeTypeId(0);
	const NUM: NativeTypeId = NativeTypeId(1);
	const BOOL: NativeTypeId = NativeTypeId(2);
	const STR: NativeTypeId = NativeTypeId(3);
	const LIST: NativeTypeId = NativeTypeId(4);
	const DICT: NativeTypeId = NativeTypeId(5);
	const PROC: NativeTypeId = NativeTypeId(6);
	const TYPE: NativeTypeId = NativeTypeId(7);
}

struct TypeRegistry {
	user: Vec<UserType>,
	native: Vec<NativeType>,
}

impl TypeRegistry {
	fn new(syms: &mut Interner) -> Self {
		let mut native = Vec::with_capacity(CORE_TYPES.len());

		for (name, _id, new, field_pairs, method_pairs) in CORE_TYPES {
			let mut fields = HashMap::with_capacity(field_pairs.len());
			for (name, get) in *field_pairs {
				fields.insert(syms.intern(name), NativeField { get: *get });
			}
			let mut methods = HashMap::with_capacity(method_pairs.len());
			for (name, call, arity) in *method_pairs {
				methods.insert(
					syms.intern(name),
					NativeMethod {
						arity: *arity,
						call: *call,
					},
				);
			}
			native.push(NativeType {
				name: syms.intern(name),
				new: *new,
				fields,
				methods,
				statics: HashMap::new(),
			});
		}

		Self {
			user: Vec::new(),
			native,
		}
	}

	fn get_type_name(&self, id: TypeId) -> Sym {
		match id {
			TypeId::User(id) => self.get_user_type(id).name,
			TypeId::Native(id) => self.get_native_type(id).name,
		}
	}

	fn get_user_type(&self, id: UserTypeId) -> &UserType {
		&self.user[id.0 as usize]
	}

	fn add_user_type(&mut self, typ: UserType) -> UserTypeId {
		let id = UserTypeId(self.user.len() as u32);
		self.user.push(typ);
		id
	}

	fn get_native_type(&self, id: NativeTypeId) -> &NativeType {
		&self.native[id.0 as usize]
	}
}

#[derive(Debug)]
struct Instance {
	typ: UserTypeId,
	fields: HashMap<Sym, Val>,
}

#[derive(Debug)]
enum Member {
	Static(Rc<RefCell<Obj>>, Sym),
	User(Rc<RefCell<Obj>>, Sym),
	Native(Val, Sym),
}

#[derive(Debug)]
enum Method {
	User(Rc<RefCell<Obj>>, Rc<RefCell<Proc>>),
	Native(Val, Sym, NativeMethod),
}

impl Member {
	fn get(&self, types: &TypeRegistry) -> Val {
		match self {
			Member::Static(type_rf, name) => {
				let Obj::Type(type_id) = *type_rf.borrow() else {
					panic!();
				};
				match type_id {
					TypeId::User(type_id) => {
						match types.get_user_type(type_id).get_static(*name).unwrap() {
							Static::Proc(proc_rf) => Val::Obj(Rc::new(RefCell::new(Obj::Method(
								Method::User(type_rf.clone(), proc_rf),
							)))),
							Static::Type(val) => val,
							Static::NativeField(_) | Static::NativeMethod(_) => panic!(),
						}
					}
					TypeId::Native(type_id) => {
						match types.get_native_type(type_id).get_static(*name).unwrap() {
							Static::Proc(_) => panic!(),
							Static::Type(_) => panic!(),
							Static::NativeField(field) => (field.get)(&Val::Obj(type_rf.clone())),
							Static::NativeMethod(meth) => Val::Obj(Rc::new(RefCell::new(
								Obj::Method(Method::Native(Val::Obj(type_rf.clone()), *name, meth)),
							))),
						}
					}
				}
			}
			Member::User(inst_rf, name) => {
				let Obj::Instance(inst) = &*inst_rf.borrow() else {
					panic!()
				};
				let typ = types.get_user_type(inst.typ);
				if let Some(val) = inst.fields.get(name) {
					val.clone()
				} else if let Some(proc_rf) = typ.methods.get(name) {
					Val::Obj(Rc::new(RefCell::new(Obj::Method(Method::User(
						inst_rf.clone(),
						proc_rf.clone(),
					)))))
				} else {
					panic!();
				}
			}
			Member::Native(recv, name) => {
				let TypeId::Native(type_id) = recv.type_id() else {
					panic!();
				};
				let typ = types.get_native_type(type_id);
				if let Some(field) = typ.fields.get(name) {
					(field.get)(recv)
				} else if let Some(meth) = typ.methods.get(name) {
					Val::Obj(Rc::new(RefCell::new(Obj::Method(Method::Native(
						recv.clone(),
						*name,
						meth.clone(),
					)))))
				} else {
					panic!();
				}
			}
		}
	}

	fn set(&self, val: Val) -> Result<(), ()> {
		match self {
			Member::Static(_, _) => Err(()),
			Member::User(inst, name) => {
				let Obj::Instance(inst) = &mut *inst.borrow_mut() else {
					panic!()
				};
				inst.fields.insert(*name, val);
				Ok(())
			}
			Member::Native(_, _) => Err(()),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tier {
	Local,
	Module,
	Prelude,
}

#[derive(Debug)]
struct Scope {
	locals: HashMap<Sym, Val>,
	outer: Option<Rc<RefCell<Scope>>>,
	tier: Tier,
}

struct Local {
	name: Sym,
	scope: Rc<RefCell<Scope>>,
	tier: Tier,
}

impl Local {
	fn is_bound(&self) -> bool {
		self.scope.borrow().locals.contains_key(&self.name)
	}

	fn get(&self) -> Val {
		self.scope.borrow().locals.get(&self.name).cloned().unwrap()
	}

	fn set(&self, val: Val) {
		self.scope.borrow_mut().locals.insert(self.name, val);
	}
}

impl Scope {
	fn local(origin: &Rc<RefCell<Scope>>, name: Sym) -> Local {
		let mut scope = origin.clone();
		loop {
			if scope.borrow().locals.contains_key(&name) {
				let tier = scope.borrow().tier;
				return Local { name, scope, tier };
			}
			let outer = scope.borrow().outer.clone();
			if let Some(outer) = outer {
				scope = outer;
			} else {
				return Local {
					name,
					scope: origin.clone(),
					tier: origin.borrow().tier,
				};
			}
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
	Error(Error, Vec<(String, Location)>),
}

impl<'syms, 'pkg> Interpreter<'syms, 'pkg> {
	pub fn new(syms: &'syms mut Interner, pkg: &'pkg Package) -> Self {
		let types = TypeRegistry::new(syms);

		let prelude = {
			let mut locals = HashMap::with_capacity(CORE_TYPES.len());
			for (_, id, _, _, _) in CORE_TYPES {
				let typ = types.get_native_type(*id);
				let obj = Obj::Type(TypeId::Native(*id));
				locals.insert(typ.name, Val::Obj(Rc::new(RefCell::new(obj))));
			}
			Rc::new(RefCell::new(Scope {
				locals,
				outer: None,
				tier: Tier::Prelude,
			}))
		};

		let scope = Rc::new(RefCell::new(Scope {
			locals: HashMap::new(),
			outer: Some(prelude),
			tier: Tier::Module,
		}));

		Self {
			syms,
			pkg,
			types,
			scope,
			inst: None,
		}
	}

	pub fn eval(mut self, chunk: Chunk) -> Result<(), (Error, Vec<(String, Location)>)> {
		for item_id in &chunk.top {
			match self.eval_module_item(&chunk, *item_id) {
				Ok(()) => {}
				Err(Signal::Return(_)) => break,
				Err(Signal::Error(err, trace)) => return Err((err, trace)),
			}
		}
		Ok(())
	}

	fn eval_module_item(&mut self, chunk: &Chunk, item_id: ModuleItemId) -> Result<(), Signal> {
		match chunk.get_module_item(item_id) {
			ModuleItem::Type(syn::Type(name, ctor_fields, items)) => {
				let mut methods = HashMap::new();
				let mut statics = HashMap::new();
				let mut body_fields = Vec::new();
				for item_id in items {
					match chunk.get_type_item(*item_id) {
						TypeItem::Field(syn::Field(name, init)) => {
							body_fields.push((*name, *init));
						}
						TypeItem::Method(syn::Method::Instance(Def(name, params, body))) => {
							let proc = Rc::new(RefCell::new(Proc {
								name: *name,
								params: params.to_vec(),
								body: *body,
								scope: self.scope.clone(),
							}));
							methods.insert(*name, proc);
						}
						TypeItem::Method(syn::Method::Static(Def(name, params, body))) => {
							let proc = Rc::new(RefCell::new(Proc {
								name: *name,
								params: params.to_vec(),
								body: *body,
								scope: self.scope.clone(),
							}));
							statics.insert(*name, Static::Proc(proc));
						}
					}
				}
				let typ = UserType {
					name: *name,
					ctor_fields: ctor_fields.to_vec(),
					body_fields,
					methods,
					statics,
					scope: self.scope.clone(),
				};
				let id = self.types.add_user_type(typ);
				let obj = Obj::Type(TypeId::User(id));
				let val = Val::Obj(Rc::new(RefCell::new(obj)));
				self.scope.borrow_mut().locals.insert(*name, val);
				Ok(())
			}
			ModuleItem::Def(Def(name, params, body)) => {
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
			ModuleItem::Expr(expr_id) => {
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
						for item in &list.items {
							let scope = Rc::new(RefCell::new(Scope {
								locals: HashMap::new(),
								outer: Some(self.scope.clone()),
								tier: Tier::Local,
							}));
							scope.borrow_mut().locals.insert(*name, item.clone());
							self.eval_block(chunk, scope.clone(), *body_id)?;
						}
						Ok(Val::Nil(Nil))
					}
					Obj::Dict(dict) => {
						let scope = Rc::new(RefCell::new(Scope {
							locals: HashMap::new(),
							outer: Some(self.scope.clone()),
							tier: Tier::Local,
						}));
						for key in dict.pairs.keys() {
							scope.borrow_mut().locals.insert(*name, key.clone());
							self.eval_block(chunk, scope.clone(), *body_id)?;
						}
						Ok(Val::Nil(Nil))
					}
					obj => {
						let src = self.pkg.get_src(chunk.src);
						let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
						let type_name = self.syms.resolve(self.types.get_type_name(obj.type_id()));
						Err(Signal::Error(
							Error::ProtocolError(ProtocolError::NotIterable(type_name.to_string())),
							vec![(String::new(), loc)],
						))
					}
				},
				val => {
					let src = self.pkg.get_src(chunk.src);
					let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
					let type_name = self.syms.resolve(self.types.get_type_name(val.type_id()));
					Err(Signal::Error(
						Error::ProtocolError(ProtocolError::NotIterable(type_name.to_string())),
						vec![(String::new(), loc)],
					))
				}
			},
			Expr::When(When(cond, then_branch, else_branch)) => {
				let cond = self.eval_expr(chunk, *cond)?.is_truthy();
				let scope = Rc::new(RefCell::new(Scope {
					locals: HashMap::new(),
					outer: Some(self.scope.clone()),
					tier: Tier::Local,
				}));
				if cond {
					self.eval_block(chunk, scope, *then_branch)
				} else if let Some(else_branch) = else_branch {
					self.eval_block(chunk, scope, *else_branch)
				} else {
					Ok(Val::Nil(Nil))
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
						Obj::Type(type_id) => match type_id {
							TypeId::User(type_id) => {
								let type_id = *type_id;
								let typ = self.types.get_user_type(type_id);
								if args.len() != typ.ctor_fields.len() {
									let src = self.pkg.get_src(chunk.src);
									let loc = src.loc(tok.pos);
									return Err(Signal::Error(
										Error::ArgumentError(ArgumentError::WrongCount(
											args.len(),
											typ.ctor_fields.len(),
										)),
										vec![(String::new(), loc)],
									));
								}
								let mut fields = HashMap::new();
								for (field, val) in typ.ctor_fields.iter().zip(args) {
									fields.insert(*field, val);
								}
								let body_fields = typ.body_fields.clone();
								let type_scope = typ.scope.clone();
								let inst_rc = Rc::new(RefCell::new(Obj::Instance(Instance {
									typ: type_id,
									fields,
								})));

								let saved_scope = self.scope.clone();
								let saved_inst = self.inst.clone();
								self.inst = Some(inst_rc.clone());
								for (name, init_id) in &body_fields {
									self.scope = Rc::new(RefCell::new(Scope {
										locals: HashMap::new(),
										outer: Some(type_scope.clone()),
										tier: Tier::Local,
									}));
									let val = match self.eval_expr(chunk, *init_id) {
										Ok(val) => val,
										Err(signal) => {
											self.scope = saved_scope;
											self.inst = saved_inst;
											return Err(signal);
										}
									};
									Member::User(inst_rc.clone(), *name).set(val).unwrap();
								}
								self.scope = saved_scope;
								self.inst = saved_inst;

								Ok(Val::Obj(inst_rc))
							}
							TypeId::Native(type_id) => {
								let typ = self.types.get_native_type(*type_id);
								match typ.new {
									Some(new) => {
										if args.is_empty() {
											Ok(new())
										} else {
											let src = self.pkg.get_src(chunk.src);
											let loc = src.loc(tok.pos);
											Err(Signal::Error(
												Error::ArgumentError(ArgumentError::WrongCount(
													args.len(),
													0,
												)),
												vec![(String::new(), loc)],
											))
										}
									}
									None => {
										let src = self.pkg.get_src(chunk.src);
										let loc = src.loc(tok.pos);
										let type_name = self.syms.resolve(typ.name);
										Err(Signal::Error(
											Error::TypeError(TypeError::NotCallable(
												type_name.to_string(),
											)),
											vec![(String::new(), loc)],
										))
									}
								}
							}
						},
						Obj::Method(meth) => match meth {
							Method::User(inst, proc) => self.eval_proc_call(
								chunk,
								&tok,
								&proc.borrow(),
								Some(inst.clone()),
								args,
							),
							Method::Native(recv, _name, meth) => {
								if args.len() != meth.arity {
									let src = self.pkg.get_src(chunk.src);
									let loc = src.loc(tok.pos);
									Err(Signal::Error(
										Error::ArgumentError(ArgumentError::WrongCount(
											args.len(),
											meth.arity,
										)),
										vec![(String::new(), loc)],
									))
								} else {
									Ok((meth.call)(recv, args))
								}
							}
						},
						obj => {
							let src = self.pkg.get_src(chunk.src);
							let loc = src.loc(tok.pos);
							let type_name =
								self.syms.resolve(self.types.get_type_name(obj.type_id()));
							Err(Signal::Error(
								Error::TypeError(TypeError::NotCallable(type_name.to_string())),
								vec![(String::new(), loc)],
							))
						}
					},
					val => {
						let src = self.pkg.get_src(chunk.src);
						let loc = src.loc(tok.pos);
						let type_name = self.syms.resolve(self.types.get_type_name(val.type_id()));
						Err(Signal::Error(
							Error::TypeError(TypeError::NotCallable(type_name.to_string())),
							vec![(String::new(), loc)],
						))
					}
				}
			}
			Expr::Member(syn::Member(val_id, name)) => {
				let val = self.eval_expr(chunk, *val_id)?;

				if let Some(member) = val.member(*name, &self.types) {
					Ok(member.get(&self.types))
				} else {
					let type_name = self.types.get_type_name(val.namespace_type_id());
					let src = self.pkg.get_src(chunk.src);
					let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
					Err(Signal::Error(
						Error::MemberError(MemberError::Missing(
							self.syms.resolve(type_name).to_string(),
							self.syms.resolve(*name).to_string(),
						)),
						vec![(String::new(), loc)],
					))
				}
			}
			Expr::Script(Script(val_id, key_id)) => match self.eval_expr(chunk, *val_id)? {
				Val::Obj(rf) => match &*rf.borrow() {
					Obj::List(list) => {
						let idx = match self.eval_expr(chunk, *key_id)? {
							Val::Num(num) => {
								if num.0.fract() <= 1e-10 {
									num.0.trunc() as usize
								} else {
									let src = self.pkg.get_src(chunk.src);
									let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
									return Err(Signal::Error(
										Error::IndexError(IndexError::NonIntegral(num.0)),
										vec![(String::new(), loc)],
									));
								}
							}
							val => {
								let src = self.pkg.get_src(chunk.src);
								let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
								return Err(Signal::Error(
									Error::TypeError(TypeError::IndexNonNum(rt_print_val(
										self.syms,
										&self.types,
										&val,
									))),
									vec![(String::new(), loc)],
								));
							}
						};
						match list.items.get(idx) {
							Some(val) => Ok(val.clone()),
							None => Ok(Val::Nil(Nil)),
						}
					}
					Obj::Dict(dict) => {
						let key = self.eval_expr(chunk, *key_id)?;
						match dict.pairs.get(&key) {
							Some(val) => Ok(val.clone()),
							None => Ok(Val::Nil(Nil)),
						}
					}
					obj => {
						let src = self.pkg.get_src(chunk.src);
						let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
						let type_name = self.syms.resolve(self.types.get_type_name(obj.type_id()));
						Err(Signal::Error(
							Error::ProtocolError(ProtocolError::NotScriptable(
								type_name.to_string(),
							)),
							vec![(String::new(), loc)],
						))
					}
				},
				val => {
					let src = self.pkg.get_src(chunk.src);
					let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
					let type_name = self.syms.resolve(self.types.get_type_name(val.type_id()));
					Err(Signal::Error(
						Error::ProtocolError(ProtocolError::NotScriptable(type_name.to_string())),
						vec![(String::new(), loc)],
					))
				}
			},
			Expr::Assign(Assign(place, val_expr_id)) => {
				let val = self.eval_expr(chunk, *val_expr_id)?;
				match place {
					Place::Name(Name(sym)) => {
						let local = Scope::local(&self.scope, *sym);
						if local.is_bound() && local.tier == Tier::Local {
							local.set(val.clone());
						} else if let Some(rf) = &self.inst
							&& let Some(member) = Val::Obj(rf.clone()).member(*sym, &self.types)
						{
							member.set(val.clone()).unwrap();
						} else {
							local.set(val.clone());
						}
						Ok(val)
					}
					Place::Member(syn::Member(target_id, name)) => {
						let target = self.eval_expr(chunk, *target_id)?;

						if let Some(member) = target.member(*name, &self.types) {
							if let Err(()) = member.set(val.clone()) {
								let type_name =
									self.types.get_type_name(target.namespace_type_id());
								let src = self.pkg.get_src(chunk.src);
								let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
								return Err(Signal::Error(
									Error::MemberError(MemberError::ReadOnly(
										self.syms.resolve(type_name).to_string(),
										self.syms.resolve(*name).to_string(),
									)),
									vec![(String::new(), loc)],
								));
							}
							Ok(val)
						} else {
							let type_name = self.types.get_type_name(target.namespace_type_id());
							let src = self.pkg.get_src(chunk.src);
							let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
							Err(Signal::Error(
								Error::MemberError(MemberError::Missing(
									self.syms.resolve(type_name).to_string(),
									self.syms.resolve(*name).to_string(),
								)),
								vec![(String::new(), loc)],
							))
						}
					}
					Place::Script(syn::Script(target_id, key_id)) => {
						let target = self.eval_expr(chunk, *target_id)?;
						match &target {
							Val::Obj(rf) => match &mut *rf.borrow_mut() {
								Obj::List(list) => {
									let idx = match self.eval_expr(chunk, *key_id)? {
										Val::Num(num) => {
											if num.0.fract() <= 1e-10 {
												num.0.trunc() as usize
											} else {
												let src = self.pkg.get_src(chunk.src);
												let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
												return Err(Signal::Error(
													Error::IndexError(IndexError::NonIntegral(
														num.0,
													)),
													vec![(String::new(), loc)],
												));
											}
										}
										val => {
											let src = self.pkg.get_src(chunk.src);
											let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
											return Err(Signal::Error(
												Error::TypeError(TypeError::IndexNonNum(
													rt_print_val(self.syms, &self.types, &val),
												)),
												vec![(String::new(), loc)],
											));
										}
									};
									if idx < list.items.len() {
										list.items[idx] = val.clone();
										Ok(val)
									} else {
										let src = self.pkg.get_src(chunk.src);
										let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
										Err(Signal::Error(
											Error::IndexError(IndexError::OutOfRange(idx)),
											vec![(String::new(), loc)],
										))
									}
								}
								Obj::Dict(dict) => {
									let key = self.eval_expr(chunk, *key_id)?;
									dict.pairs.insert(key, val.clone());
									Ok(val)
								}
								obj => {
									let src = self.pkg.get_src(chunk.src);
									let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
									let type_name =
										self.syms.resolve(self.types.get_type_name(obj.type_id()));
									Err(Signal::Error(
										Error::ProtocolError(ProtocolError::NotScriptable(
											type_name.to_string(),
										)),
										vec![(String::new(), loc)],
									))
								}
							},
							val => {
								let src = self.pkg.get_src(chunk.src);
								let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
								let type_name =
									self.syms.resolve(self.types.get_type_name(val.type_id()));
								Err(Signal::Error(
									Error::ProtocolError(ProtocolError::NotScriptable(
										type_name.to_string(),
									)),
									vec![(String::new(), loc)],
								))
							}
						}
					}
				}
			}
			Expr::Binary(Binary(op, lhs_id, rhs_id)) => match op {
				BinaryOp::Or => {
					let lhs = self.eval_expr(chunk, *lhs_id)?;
					if lhs.is_truthy() {
						Ok(lhs)
					} else {
						self.eval_expr(chunk, *rhs_id)
					}
				}
				BinaryOp::And => {
					let lhs = self.eval_expr(chunk, *lhs_id)?;
					if !lhs.is_truthy() {
						Ok(lhs)
					} else {
						self.eval_expr(chunk, *rhs_id)
					}
				}
				BinaryOp::Eq
				| BinaryOp::NotEq
				| BinaryOp::Lt
				| BinaryOp::Gt
				| BinaryOp::LtEq
				| BinaryOp::GtEq
				| BinaryOp::Add
				| BinaryOp::Sub
				| BinaryOp::Mul
				| BinaryOp::Div => {
					let lhs = self.eval_expr(chunk, *lhs_id)?;
					let rhs = self.eval_expr(chunk, *rhs_id)?;
					match op {
						BinaryOp::Eq => Ok(Val::Bool(Bool(lhs == rhs))),
						BinaryOp::NotEq => Ok(Val::Bool(Bool(lhs != rhs))),
						BinaryOp::Lt | BinaryOp::Gt | BinaryOp::LtEq | BinaryOp::GtEq => {
							match (lhs, rhs) {
								(Val::Num(lhs), Val::Num(rhs)) => Ok(Val::Bool(Bool(match op {
									BinaryOp::Lt => lhs.0 < rhs.0,
									BinaryOp::Gt => lhs.0 > rhs.0,
									BinaryOp::LtEq => lhs.0 <= rhs.0,
									BinaryOp::GtEq => lhs.0 >= rhs.0,
									_ => panic!(),
								}))),
								(lhs, rhs) => {
									let src = self.pkg.get_src(chunk.src);
									let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
									let val = match (&lhs, &rhs) {
										(Val::Num(_), _) => rhs,
										(_, Val::Num(_)) => lhs,
										_ => lhs,
									};
									let type_name =
										self.syms.resolve(self.types.get_type_name(val.type_id()));
									Err(Signal::Error(
										Error::TypeError(TypeError::NotOrderable(
											type_name.to_string(),
										)),
										vec![(String::new(), loc)],
									))
								}
							}
						}
						BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
							match (lhs, rhs) {
								(Val::Num(lhs), Val::Num(rhs)) => Ok(Val::Num(Num(match op {
									BinaryOp::Add => lhs.0 + rhs.0,
									BinaryOp::Sub => lhs.0 - rhs.0,
									BinaryOp::Mul => lhs.0 * rhs.0,
									BinaryOp::Div => lhs.0 / rhs.0,
									_ => panic!(),
								}))),
								(lhs, rhs) => {
									let src = self.pkg.get_src(chunk.src);
									let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
									let val = match (&lhs, &rhs) {
										(Val::Num(_), _) => rhs,
										(_, Val::Num(_)) => lhs,
										_ => lhs,
									};
									let type_name =
										self.syms.resolve(self.types.get_type_name(val.type_id()));
									Err(Signal::Error(
										Error::TypeError(TypeError::ArithNonNum(
											type_name.to_string(),
										)),
										vec![(String::new(), loc)],
									))
								}
							}
						}
						BinaryOp::Or | BinaryOp::And => panic!(),
					}
				}
			},
			Expr::Unary(Unary(op, val_id)) => match op {
				UnaryOp::Not => {
					let val = self.eval_expr(chunk, *val_id)?;
					Ok(Val::Bool(Bool(!val.is_truthy())))
				}
				UnaryOp::Neg => match self.eval_expr(chunk, *val_id)? {
					Val::Num(num) => Ok(Val::Num(Num(-num.0))),
					val => {
						let src = self.pkg.get_src(chunk.src);
						let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
						let type_name = self.syms.resolve(self.types.get_type_name(val.type_id()));
						Err(Signal::Error(
							Error::TypeError(TypeError::ArithNonNum(type_name.to_string())),
							vec![(String::new(), loc)],
						))
					}
				},
			},
			Expr::Self_ => match &self.inst {
				Some(inst) => Ok(Val::Obj(inst.clone())),
				None => todo!(),
			},
			Expr::Name(Name(name)) => {
				let local = Scope::local(&self.scope, *name);
				if local.is_bound() && local.tier == Tier::Local {
					Ok(local.get())
				} else if let Some(rf) = &self.inst
					&& let Some(member) = Val::Obj(rf.clone()).member(*name, &self.types)
				{
					Ok(member.get(&self.types))
				} else if local.is_bound() {
					Ok(local.get())
				} else {
					let src = self.pkg.get_src(chunk.src);
					let loc = src.loc(chunk.get_expr_tok(expr_id).pos);
					let name = self.syms.resolve(*name);
					Err(Signal::Error(
						Error::NameError(name.to_string()),
						vec![(String::new(), loc)],
					))
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
			return Err(Signal::Error(
				Error::ArgumentError(ArgumentError::WrongCount(args.len(), proc.params.len())),
				vec![(String::new(), loc)],
			));
		}
		let mut scope = Scope {
			locals: HashMap::new(),
			outer: Some(proc.scope.clone()),
			tier: Tier::Local,
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
			Err(Signal::Error(err, mut trace)) => {
				let proc_name = self.syms.resolve(proc.name);
				trace.last_mut().unwrap().0.push_str(proc_name);

				let src = self.pkg.get_src(chunk.src);
				let loc = src.loc(tok.pos);
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
		for expr_id in &block.0 {
			match self.eval_expr(chunk, *expr_id) {
				Ok(val) => {
					res = val;
				}
				Err(Signal::Return(val)) => {
					self.scope = saved;
					return Err(Signal::Return(val));
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

fn rt_print_val(syms: &Interner, types: &TypeRegistry, val: &Val) -> String {
	match val {
		Val::Num(num) => format!("{}", num.0),
		Val::Bool(bool) => format!("{}", bool.0),
		Val::Obj(rf) => rt_print_obj(syms, types, &rf.borrow()),
		Val::Nil(_) => String::from("nil"),
	}
}

fn rt_print_obj(syms: &Interner, types: &TypeRegistry, obj: &Obj) -> String {
	match obj {
		Obj::Str(str) => format!("{}", str.chars),
		Obj::List(list) => {
			let mut res = String::new();
			res.push('[');
			for (i, item) in list.items.iter().enumerate() {
				res.push_str(&rt_debug_val(syms, types, item));
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
				res.push_str(&rt_debug_val(syms, types, key));
				res.push_str(": ");
				res.push_str(&rt_debug_val(syms, types, val));
				if i + 1 != dict.pairs.len() {
					res.push_str(", ");
				}
			}
			res.push('}');
			res
		}
		Obj::Proc(proc) => rt_print_proc(syms, proc),
		Obj::Type(type_id) => match type_id {
			TypeId::User(type_id) => {
				let typ = types.get_user_type(*type_id);
				let name = syms.resolve(typ.name);
				let mut res = String::new();
				res.push_str(&format!("type {}(", name));
				for (i, field) in typ.ctor_fields.iter().enumerate() {
					let field = syms.resolve(*field);
					res.push_str(field);
					if i + 1 != typ.ctor_fields.len() {
						res.push_str(", ");
					}
				}
				res.push(')');
				res
			}
			TypeId::Native(type_id) => {
				let typ = types.get_native_type(*type_id);
				let name = syms.resolve(typ.name);
				format!("type {}", name)
			}
		},
		Obj::Instance(inst) => {
			let typ = types.get_user_type(inst.typ);
			let name = syms.resolve(typ.name);
			let mut res = String::new();
			res.push_str(&format!("{}(", name));
			for (i, name) in typ.ctor_fields.iter().enumerate() {
				let val = inst.fields.get(name).unwrap();
				res.push_str(&rt_print_val(syms, types, val));
				if i + 1 != typ.ctor_fields.len() {
					res.push_str(", ");
				}
			}
			res.push(')');
			res
		}
		Obj::Method(meth) => match meth {
			Method::User(_, proc) => rt_print_proc(syms, &proc.borrow()),
			Method::Native(_, name, _) => {
				format!("def {}", syms.resolve(*name))
			}
		},
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

fn rt_debug_val(syms: &Interner, types: &TypeRegistry, val: &Val) -> String {
	match val {
		Val::Obj(rf) => match &*rf.borrow() {
			Obj::Str(str) => format!("\"{}\"", str.chars),
			obj => rt_print_obj(syms, types, obj),
		},
		val => rt_print_val(syms, types, val),
	}
}
