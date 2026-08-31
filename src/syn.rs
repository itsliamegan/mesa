pub mod lex;
pub mod nodes;
pub mod parse;

use std::fmt::{self, Display, Formatter};

use crate::intern::Interner;
use crate::src::{Location, SourceId, Sources, Span};
use crate::syn::lex::{Comments, Lexer, TokenRange, TokenTag, Tokens};
use crate::syn::nodes::*;
use crate::syn::parse::Parser;

pub fn parse(syms: &mut Interner, sources: &Sources) -> Result<Chunks, Vec<Error>> {
	let mut chunks = Chunks::new();
	let mut errs = Vec::new();

	for id in sources.ids() {
		let source = sources.get(id);
		let (toks, comments) = match Lexer::new(syms, source).lex() {
			Ok(lexed) => lexed,
			Err(err) => {
				errs.push(err);
				continue;
			}
		};
		let chunk = match Parser::new(source, toks, comments).parse() {
			Ok(chunk) => chunk,
			Err(err) => {
				errs.push(err);
				continue;
			}
		};
		chunks.add(chunk);
	}

	if !errs.is_empty() {
		return Err(errs);
	}

	Ok(chunks)
}

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
	tokens: Tokens,
	comments: Comments,
	module_items: Nodes<ModuleItem, ModuleItemId>,
	type_items: Nodes<TypeItem, TypeItemId>,
	extern_items: Nodes<ExternItem, ExternItemId>,
	defs: Nodes<Def, DefId>,
	exprs: Nodes<Expr, ExprId>,
	blocks: Vec<Block>,
}

impl Chunk {
	pub(crate) fn new(src: SourceId, tokens: Tokens, comments: Comments) -> Self {
		Self {
			src,
			top: Vec::new(),
			tokens,
			comments,
			module_items: Nodes::new(),
			type_items: Nodes::new(),
			extern_items: Nodes::new(),
			defs: Nodes::new(),
			exprs: Nodes::new(),
			blocks: Vec::new(),
		}
	}

	pub fn tokens(&self) -> &Tokens {
		&self.tokens
	}

	pub fn comments(&self) -> &Comments {
		&self.comments
	}

	pub fn get_module_item(&self, item_id: ModuleItemId) -> &ModuleItem {
		self.module_items.get(item_id)
	}

	pub fn get_module_item_span(&self, item_id: ModuleItemId) -> Span {
		self.tokens
			.span(self.src, self.module_items.get_token_range(item_id))
	}

	pub fn add_module_item(&mut self, range: TokenRange, item: ModuleItem) -> ModuleItemId {
		self.module_items.add(range, item)
	}

	pub fn get_type_item(&self, item_id: TypeItemId) -> &TypeItem {
		self.type_items.get(item_id)
	}

	pub fn get_type_item_span(&self, item_id: TypeItemId) -> Span {
		self.tokens
			.span(self.src, self.type_items.get_token_range(item_id))
	}

	pub fn add_type_item(&mut self, range: TokenRange, item: TypeItem) -> TypeItemId {
		self.type_items.add(range, item)
	}

	// Every type item in the chunk, cases and nested types included, at any
	// nesting depth, in the order they were added.
	pub fn type_item_ids(&self) -> impl ExactSizeIterator<Item = TypeItemId> {
		(0..self.type_items.len()).map(TypeItemId::from_index)
	}

	pub fn get_extern_item(&self, item_id: ExternItemId) -> &ExternItem {
		self.extern_items.get(item_id)
	}

	pub fn get_extern_item_span(&self, item_id: ExternItemId) -> Span {
		self.tokens
			.span(self.src, self.extern_items.get_token_range(item_id))
	}

	pub fn add_extern_item(&mut self, range: TokenRange, item: ExternItem) -> ExternItemId {
		self.extern_items.add(range, item)
	}

	pub fn get_def(&self, def_id: DefId) -> &Def {
		self.defs.get(def_id)
	}

	pub fn get_def_span(&self, def_id: DefId) -> Span {
		self.tokens
			.span(self.src, self.defs.get_token_range(def_id))
	}

	pub fn add_def(&mut self, range: TokenRange, def: Def) -> DefId {
		self.defs.add(range, def)
	}

	pub fn def_ids(&self) -> impl ExactSizeIterator<Item = DefId> {
		(0..self.defs.len()).map(DefId::from_index)
	}

	pub fn get_expr(&self, expr_id: ExprId) -> &Expr {
		self.exprs.get(expr_id)
	}

	pub fn get_expr_span(&self, expr_id: ExprId) -> Span {
		self.tokens
			.span(self.src, self.exprs.get_token_range(expr_id))
	}

	pub fn get_expr_token_range(&self, expr_id: ExprId) -> TokenRange {
		self.exprs.get_token_range(expr_id)
	}

	pub fn set_expr_token_range(&mut self, expr_id: ExprId, range: TokenRange) {
		self.exprs.set_token_range(expr_id, range);
	}

	pub fn add_expr(&mut self, range: TokenRange, expr: Expr) -> ExprId {
		self.exprs.add(range, expr)
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
