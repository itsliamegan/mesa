pub mod lex;
pub mod nodes;
pub mod parse;

use std::fmt::{self, Display, Formatter};

use crate::src::{Location, SourceId, Span};
use crate::syn::lex::TokenTag;
use crate::syn::nodes::*;

#[derive(Debug)]
pub enum Error {
	UnexpectedChar(Location, char),
	UnexpectedToken(Location, TokenTag),
	UnterminatedStrLit(Location),
	UnterminatedCharLit(Location),
	MultiCharLit(Location),
	UnknownEsc(Location, char),
	UnknownBuiltin(Location, String),
	PositionalAfterKeyword(Location),
}

impl Error {
	fn loc(&self) -> &Location {
		match self {
			Self::UnexpectedChar(loc, _) => loc,
			Self::UnexpectedToken(loc, _) => loc,
			Self::UnterminatedStrLit(loc) => loc,
			Self::UnterminatedCharLit(loc) => loc,
			Self::MultiCharLit(loc) => loc,
			Self::UnknownEsc(loc, _) => loc,
			Self::UnknownBuiltin(loc, _) => loc,
			Self::PositionalAfterKeyword(loc) => loc,
		}
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		write!(f, "{}: syntax error: ", self.loc())?;
		match self {
			Self::UnexpectedChar(_, char) => write!(f, "unexpected character '{}'", char),
			Self::UnexpectedToken(_, tag) => write!(f, "unexpected token {}", tag.name()),
			Self::UnterminatedStrLit(_) => write!(f, "unterminated string literal"),
			Self::UnterminatedCharLit(_) => write!(f, "unterminated character literal"),
			Self::MultiCharLit(_) => write!(f, "character literal must hold exactly one character"),
			Self::UnknownEsc(_, esc) => write!(f, "unknown escape sequence '\\{}'", esc),
			Self::UnknownBuiltin(_, builtin) => write!(f, "unknown builtin '{}'", builtin),
			Self::PositionalAfterKeyword(_) => {
				write!(f, "positional argument after keyword argument")
			}
		}
	}
}

#[derive(Debug)]
pub struct Chunks {
	chunks: Vec<Chunk>,
}

impl Chunks {
	pub fn new() -> Self {
		Self { chunks: Vec::new() }
	}

	pub fn ids(&self) -> impl Iterator<Item = ChunkId> {
		(0..self.chunks.len()).map(ChunkId::from_index)
	}

	pub fn get(&self, id: ChunkId) -> &Chunk {
		&self.chunks[id.index()]
	}

	pub fn add(&mut self, chunk: Chunk) -> ChunkId {
		let id = ChunkId::from_index(self.chunks.len());
		self.chunks.push(chunk);
		id
	}
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ChunkId(u32);

impl ChunkId {
	fn from_index(index: usize) -> Self {
		Self(index as u32)
	}

	pub fn index(&self) -> usize {
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
		(0..self.type_items.len()).map(TypeItemId::from_index)
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
		(0..self.proto_items.len()).map(ProtoItemId::from_index)
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
		&self.blocks[block_id.index()]
	}

	pub fn add_block(&mut self, block: Block) -> BlockId {
		let id = BlockId::from_index(self.blocks.len());
		self.blocks.push(block);
		id
	}

	pub fn add_to_top(&mut self, item_id: ModuleItemId) {
		self.top.push(item_id);
	}
}
