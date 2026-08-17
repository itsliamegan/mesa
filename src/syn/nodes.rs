use crate::intern::Sym;
use crate::src::{SourceId, Span};

trait NodeId: Copy {
	fn from_index(i: u32) -> Self;
	fn index(self) -> u32;
}

#[derive(Debug)]
struct Nodes<Node, Id> {
	nodes: Vec<Node>,
	spans: Vec<Span>,
	_id: std::marker::PhantomData<Id>,
}

impl<Node, Id: NodeId> Nodes<Node, Id> {
	fn new() -> Self {
		Self {
			nodes: Vec::new(),
			spans: Vec::new(),
			_id: std::marker::PhantomData,
		}
	}

	fn len(&self) -> usize {
		self.nodes.len()
	}

	fn get(&self, id: Id) -> &Node {
		&self.nodes[id.index() as usize]
	}

	fn get_span(&self, id: Id) -> Span {
		self.spans[id.index() as usize]
	}

	fn add(&mut self, span: Span, node: Node) -> Id {
		let id = Id::from_index(self.nodes.len() as u32);
		self.nodes.push(node);
		self.spans.push(span);
		id
	}
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ModuleItemId(u32);

impl NodeId for ModuleItemId {
	fn from_index(i: u32) -> Self {
		Self(i)
	}

	fn index(self) -> u32 {
		self.0
	}
}

#[derive(Debug)]
pub enum ModuleItem {
	Module(Module),
	Import(Import),
	Export(Export),
	Type(Type),
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
	pub impls: Vec<Sym>,
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TypeItemId(u32);

impl NodeId for TypeItemId {
	fn from_index(i: u32) -> Self {
		Self(i)
	}

	fn index(self) -> u32 {
		self.0
	}
}

#[derive(Debug)]
pub enum TypeItem {
	Case(Type),
	Field(Field),
	Type(Type),
	Method(Method),
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ProtoItemId(u32);

impl NodeId for ProtoItemId {
	fn from_index(i: u32) -> Self {
		Self(i)
	}

	fn index(self) -> u32 {
		self.0
	}
}

#[derive(Debug)]
pub struct ProtoItem {
	pub def: Def,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ExprId(u32);

impl NodeId for ExprId {
	fn from_index(i: u32) -> Self {
		Self(i)
	}

	fn index(self) -> u32 {
		self.0
	}
}

#[derive(Debug)]
pub enum Expr {
	Each(Each),
	Loop(Loop),
	When(When),
	Match(Match),
	Return(Return),
	Break(Break),
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
pub struct Return {
	pub val: Option<ExprId>,
}

#[derive(Debug)]
pub struct Break {
	pub val: Option<ExprId>,
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ChunkId(u32);

impl ChunkId {
	pub fn new(i: u32) -> Self {
		Self(i)
	}

	pub fn index(self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug)]
pub struct Chunk {
	pub src: SourceId,
	pub top: Vec<ModuleItemId>,
	module_items: Nodes<ModuleItem, ModuleItemId>,
	type_items: Nodes<TypeItem, TypeItemId>,
	proto_items: Nodes<ProtoItem, ProtoItemId>,
	exprs: Nodes<Expr, ExprId>,
	blocks: Vec<Block>,
}

impl Chunk {
	pub fn new(src: SourceId) -> Self {
		Self {
			src,
			top: Vec::new(),
			module_items: Nodes::new(),
			type_items: Nodes::new(),
			proto_items: Nodes::new(),
			exprs: Nodes::new(),
			blocks: Vec::new(),
		}
	}

	pub fn get_module_item(&self, item_id: ModuleItemId) -> &ModuleItem {
		self.module_items.get(item_id)
	}

	pub fn get_module_item_span(&self, item_id: ModuleItemId) -> Span {
		self.module_items.get_span(item_id)
	}

	pub fn add_module_item(&mut self, span: Span, item: ModuleItem) -> ModuleItemId {
		self.module_items.add(span, item)
	}

	pub fn get_type_item(&self, item_id: TypeItemId) -> &TypeItem {
		self.type_items.get(item_id)
	}

	pub fn get_type_item_span(&self, item_id: TypeItemId) -> Span {
		self.type_items.get_span(item_id)
	}

	pub fn add_type_item(&mut self, span: Span, item: TypeItem) -> TypeItemId {
		self.type_items.add(span, item)
	}

	// Every type item in the chunk, cases and nested types included, at any
	// nesting depth, in the order they were added.
	pub fn type_item_ids(&self) -> impl ExactSizeIterator<Item = TypeItemId> {
		(0..self.type_items.len() as u32).map(TypeItemId::from_index)
	}

	pub fn get_proto_item(&self, item_id: ProtoItemId) -> &ProtoItem {
		self.proto_items.get(item_id)
	}

	pub fn get_proto_item_span(&self, item_id: ProtoItemId) -> Span {
		self.proto_items.get_span(item_id)
	}

	pub fn add_proto_item(&mut self, span: Span, item: ProtoItem) -> ProtoItemId {
		self.proto_items.add(span, item)
	}

	// Every protocol item in the chunk, across every protocol it declares, in
	// the order they were added.
	pub fn proto_item_ids(&self) -> impl ExactSizeIterator<Item = ProtoItemId> {
		(0..self.proto_items.len() as u32).map(ProtoItemId::from_index)
	}

	pub fn get_expr(&self, expr_id: ExprId) -> &Expr {
		self.exprs.get(expr_id)
	}

	pub fn get_expr_span(&self, expr_id: ExprId) -> Span {
		self.exprs.get_span(expr_id)
	}

	pub fn add_expr(&mut self, span: Span, expr: Expr) -> ExprId {
		self.exprs.add(span, expr)
	}

	pub fn get_block(&self, block_id: BlockId) -> &Block {
		&self.blocks[block_id.0 as usize]
	}

	pub fn add_block(&mut self, block: Block) -> BlockId {
		let id = BlockId(self.blocks.len() as u32);
		self.blocks.push(block);
		id
	}

	pub fn add_to_top(&mut self, item_id: ModuleItemId) {
		self.top.push(item_id);
	}
}
