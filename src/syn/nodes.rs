use crate::intern::Sym;
use crate::src::Span;

pub trait NodeId: Copy {
	fn from_index(index: usize) -> Self;
	fn index(&self) -> usize;
}

#[derive(Debug)]
pub struct Nodes<Node, Id> {
	nodes: Vec<Node>,
	spans: Vec<Span>,
	_id: std::marker::PhantomData<Id>,
}

impl<Node, Id: NodeId> Nodes<Node, Id> {
	pub fn new() -> Self {
		Self {
			nodes: Vec::new(),
			spans: Vec::new(),
			_id: std::marker::PhantomData,
		}
	}

	pub fn len(&self) -> usize {
		self.nodes.len()
	}

	pub fn get(&self, id: Id) -> &Node {
		&self.nodes[id.index()]
	}

	pub fn get_span(&self, id: Id) -> Span {
		self.spans[id.index()]
	}

	pub fn add(&mut self, span: Span, node: Node) -> Id {
		let id = Id::from_index(self.nodes.len());
		self.nodes.push(node);
		self.spans.push(span);
		id
	}
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ModuleItemId(u32);

impl NodeId for ModuleItemId {
	fn from_index(index: usize) -> Self {
		Self(index as u32)
	}

	fn index(&self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug)]
pub enum ModuleItem {
	Module(Module),
	Import(Import),
	Export(Export),
	Type(Type),
	Extern(Extern),
	Proto(Proto),
	Def(Def),
	Expr(ExprId),
}

#[derive(Debug)]
pub struct Module {
	pub path: Vec<Sym>,
}

#[derive(Debug)]
pub struct Import {
	pub path: Vec<Sym>,
}

#[derive(Debug)]
pub struct Export {
	pub names: Vec<Sym>,
}

#[derive(Debug)]
pub struct Type {
	pub name: Sym,
	pub params: Vec<Param>,
	pub impls: Vec<Vec<Sym>>,
	pub items: Vec<TypeItemId>,
}

#[derive(Debug)]
pub struct Extern {
	pub name: Sym,
	pub impls: Vec<Vec<Sym>>,
	pub items: Vec<TypeItemId>,
}

#[derive(Debug)]
pub struct Proto {
	pub name: Sym,
	pub items: Vec<ProtoItemId>,
}

#[derive(Debug)]
pub struct Def {
	pub name: Sym,
	pub params: Vec<Param>,
	pub body: BlockId,
}

#[derive(Debug, Clone, Copy)]
pub struct Param {
	pub name: Sym,
	pub default: Option<ExprId>,
}

#[derive(Debug, Clone, Copy)]
pub struct TypeItemId(u32);

impl NodeId for TypeItemId {
	fn from_index(index: usize) -> Self {
		Self(index as u32)
	}

	fn index(&self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug)]
pub enum TypeItem {
	Case(Type),
	Field(Field),
	Type(Type),
	Method(Method),
	Extern(ExternMethod),
}

#[derive(Debug)]
pub struct Field {
	pub name: Sym,
	pub init: ExprId,
}

#[derive(Debug)]
pub enum Method {
	Instance(Def),
	Static(Def),
}

// A native member of an 'extern type', named in mesa source but implemented in
// Rust. It has no body to carry: the declaration says only that the member
// exists and what it takes.
#[derive(Debug)]
pub enum ExternMethod {
	Instance(ExternDef),
	Static(ExternDef),
}

#[derive(Debug)]
pub struct ExternDef {
	pub name: Sym,
	pub params: Vec<Param>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ProtoItemId(u32);

impl NodeId for ProtoItemId {
	fn from_index(index: usize) -> Self {
		Self(index as u32)
	}

	fn index(&self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug)]
pub struct ProtoItem {
	pub def: Def,
}

#[derive(Debug, Clone, Copy)]
pub struct ExprId(u32);

impl NodeId for ExprId {
	fn from_index(index: usize) -> Self {
		Self(index as u32)
	}

	fn index(&self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug)]
pub enum Expr {
	Each(Each),
	Loop(Loop),
	When(When),
	Match(Match),
	Do(Do),
	Return(Return),
	Break(Break),
	Raise(Raise),
	Self_,
	Call(Call),
	Member(Member),
	Access(Access),
	Mention(Mention),
	Assign(Assign),
	Binary(Binary),
	Unary(Unary),
	Name(Name),
	Builtin(Builtin),
	Lit(Lit),
}

#[derive(Debug)]
pub struct Each {
	pub item: Sym,
	pub iter: ExprId,
	pub body: BlockId,
}

#[derive(Debug)]
pub struct Loop {
	pub body: BlockId,
}

#[derive(Debug)]
pub struct When {
	pub cond: ExprId,
	pub then_branch: BlockId,
	pub else_branch: Option<BlockId>,
}

#[derive(Debug)]
pub struct Match {
	pub scrutinee: ExprId,
	pub arms: Vec<Arm>,
	pub else_branch: Option<BlockId>,
}

#[derive(Debug)]
pub struct Arm {
	pub path: ExprId,
	pub body: BlockId,
}

#[derive(Debug)]
pub struct Do {
	pub body: BlockId,
	pub binding: Option<Sym>,
	pub arms: Vec<Arm>,
	pub else_branch: Option<BlockId>,
}

#[derive(Debug)]
pub struct Return {
	pub val: Option<ExprId>,
}

#[derive(Debug)]
pub struct Break {
	pub val: Option<ExprId>,
}

#[derive(Debug)]
pub struct Raise {
	pub val: ExprId,
}

#[derive(Debug)]
pub struct Call {
	pub callee: ExprId,
	pub args: Vec<Arg>,
}

#[derive(Debug, Clone, Copy)]
pub struct Arg {
	pub name: Option<Sym>,
	pub val: ExprId,
}

#[derive(Debug, Clone)]
pub struct Member {
	pub receiver: ExprId,
	pub name: Sym,
}

#[derive(Debug, Clone)]
pub struct Access {
	pub receiver: ExprId,
	pub key: ExprId,
}

#[derive(Debug)]
pub struct Mention {
	pub val: ExprId,
}

#[derive(Debug)]
pub struct Assign {
	pub place: Place,
	pub val: ExprId,
}

#[derive(Debug, Clone)]
pub enum Place {
	Name(Name),
	Member(Member),
	Access(Access),
}

#[derive(Debug, Clone)]
pub struct Name {
	pub sym: Sym,
}

#[derive(Debug, Clone)]
pub enum Builtin {
	Print { val: ExprId },
	Type { val: ExprId },
}

#[derive(Debug)]
pub struct Binary {
	pub op: BinaryOp,
	pub lhs: ExprId,
	pub rhs: ExprId,
}

#[derive(Debug)]
pub enum BinaryOp {
	Append,

	Or,
	And,

	Eq,
	NotEq,

	Lt,
	Gt,
	LtEq,
	GtEq,

	Add,
	Sub,
	Mul,
	Div,
}

#[derive(Debug)]
pub struct Unary {
	pub op: UnaryOp,
	pub val: ExprId,
}

#[derive(Debug)]
pub enum UnaryOp {
	Not,
	Neg,
}

#[derive(Debug)]
pub enum Lit {
	Str(String),
	Char(char),
	Num(f64),
	Bool(bool),
	List(Vec<ExprId>),
	Dict(Vec<(ExprId, ExprId)>),
	Nil,
}

#[derive(Debug, Clone, Copy)]
pub struct BlockId(u32);

#[derive(Debug)]
pub struct Block {
	pub exprs: Vec<ExprId>,
}

impl BlockId {
	pub fn index(&self) -> usize {
		self.0 as usize
	}

	pub fn from_index(index: usize) -> Self {
		Self(index as u32)
	}
}
