use std::fmt::{self, Display, Formatter};
use std::ops::{Index, Range};

use crate::intern::{Interner, Sym};

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
pub struct Package {
	srcs: Vec<Source>,
	chunks: Vec<Chunk>,
}

impl Package {
	pub fn new() -> Self {
		Self {
			srcs: Vec::new(),
			chunks: Vec::new(),
		}
	}

	pub fn get_src(&self, id: SourceId) -> &Source {
		&self.srcs[id.0 as usize]
	}

	pub fn add_src(&mut self, file: String, text: String) -> SourceId {
		let id = SourceId(self.srcs.len() as u32);
		let src = Source { id, file, text };
		self.srcs.push(src);
		id
	}

	pub fn get_chunk(&self, id: ChunkId) -> &Chunk {
		&self.chunks[id.0 as usize]
	}

	pub fn add_chunk(&mut self, chunk: Chunk) -> ChunkId {
		let id = ChunkId(self.chunks.len() as u32);
		self.chunks.push(chunk);
		id
	}
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SourceId(u32);

#[derive(Debug)]
pub struct Source {
	id: SourceId,
	file: String,
	text: String,
}

impl Source {
	pub fn len(&self) -> usize {
		self.text.len()
	}

	pub fn loc(&self, pos: usize) -> Location {
		let mut i = 0;
		let mut lin = 1;
		let mut col = 1;
		while i < pos && i < self.text.len() {
			if self.text.as_bytes()[i] == b'\n' {
				lin += 1;
				col = 1;
			} else {
				col += 1;
			}
			i += 1;
		}
		Location {
			file: self.file.clone(),
			lin,
			col,
		}
	}
}

impl Index<usize> for Source {
	type Output = u8;

	fn index(&self, idx: usize) -> &Self::Output {
		&self.text.as_bytes()[idx]
	}
}

impl Index<Range<usize>> for Source {
	type Output = str;

	fn index(&self, idx: Range<usize>) -> &Self::Output {
		&self.text[idx]
	}
}

#[derive(Debug)]
pub struct Location {
	file: String,
	lin: usize,
	col: usize,
}

impl Display for Location {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		write!(f, "{}:{},{}", self.file, self.lin, self.col)
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Token {
	pub src: SourceId,
	pub tag: TokenTag,
	pub sym: Option<Sym>,
	pub pos: usize,
	pub end: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Span {
	pub src: SourceId,
	pub start: usize,
	pub end: usize,
}

impl Span {
	pub fn loc(&self, pkg: &Package) -> Location {
		pkg.get_src(self.src).loc(self.start)
	}
}

impl From<Token> for Span {
	fn from(tok: Token) -> Span {
		Span {
			src: tok.src,
			start: tok.pos,
			end: tok.end,
		}
	}
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TokenTag {
	Eof,

	Type,
	Case,
	Def,
	Each,
	Loop,
	Do,
	In,
	When,
	Then,
	Else,
	End,
	Return,
	Break,

	Self_,
	Ident,
	Builtin,
	Str,
	Char,
	Num,
	Bool,
	Nil,

	Eq,

	Or,
	And,
	Not,

	EqEq,
	NotEq,

	Lt,
	Gt,
	LtEq,
	GtEq,

	Plus,
	Minus,
	Star,
	Slash,

	Amp,
	LtLt,

	LBrace,
	RBrace,
	LBrack,
	RBrack,
	LParen,
	RParen,

	Colon,
	Comma,
	Dot,
}

impl TokenTag {
	fn name(&self) -> &'static str {
		match self {
			TokenTag::Eof => "EOF",

			TokenTag::Type => "TYPE",
			TokenTag::Case => "CASE",
			TokenTag::Def => "DEF",
			TokenTag::Each => "EACH",
			TokenTag::Loop => "LOOP",
			TokenTag::Do => "DO",
			TokenTag::In => "IN",
			TokenTag::When => "WHEN",
			TokenTag::Then => "THEN",
			TokenTag::Else => "ELSE",
			TokenTag::End => "END",
			TokenTag::Return => "RETURN",
			TokenTag::Break => "BREAK",
			TokenTag::Or => "OR",
			TokenTag::And => "AND",
			TokenTag::Not => "NOT",

			TokenTag::Self_ => "SELF",
			TokenTag::Ident => "IDENT",
			TokenTag::Builtin => "BUILTIN",
			TokenTag::Str => "STR",
			TokenTag::Char => "CHAR",
			TokenTag::Num => "NUM",
			TokenTag::Bool => "BOOL",
			TokenTag::Nil => "NIL",

			TokenTag::Eq => "EQ",

			TokenTag::EqEq => "EQEQ",
			TokenTag::NotEq => "NOTEQ",

			TokenTag::Lt => "LT",
			TokenTag::Gt => "GT",
			TokenTag::LtEq => "LTEQ",
			TokenTag::GtEq => "GTEQ",

			TokenTag::Plus => "PLUS",
			TokenTag::Minus => "MINUS",
			TokenTag::Star => "STAR",
			TokenTag::Slash => "SLASH",

			TokenTag::Amp => "AMP",
			TokenTag::LtLt => "LTLT",

			TokenTag::LBrace => "LBRACE",
			TokenTag::RBrace => "RBRACE",
			TokenTag::LBrack => "LBRACK",
			TokenTag::RBrack => "RBRACK",
			TokenTag::LParen => "LPAREN",
			TokenTag::RParen => "RPAREN",

			TokenTag::Colon => "COLON",
			TokenTag::Comma => "COMMA",
			TokenTag::Dot => "DOT",
		}
	}
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct TokenId(u32);

impl TokenId {
	fn index(self) -> usize {
		self.0 as usize
	}

	fn next(self) -> Self {
		Self(self.0 + 1)
	}

	fn prev(self) -> Self {
		Self(self.0 - 1)
	}
}

#[derive(Debug)]
pub struct Tokens {
	tags: Vec<TokenTag>,
	syms: Vec<Option<Sym>>,
	starts: Vec<usize>,
	ends: Vec<usize>,
	nl_befores: Vec<bool>,
}

impl Tokens {
	fn new() -> Self {
		Self {
			tags: Vec::new(),
			syms: Vec::new(),
			starts: Vec::new(),
			ends: Vec::new(),
			nl_befores: Vec::new(),
		}
	}

	fn len(&self) -> usize {
		self.tags.len()
	}

	fn push(
		&mut self,
		tag: TokenTag,
		sym: Option<Sym>,
		start: usize,
		end: usize,
		nl_before: bool,
	) -> TokenId {
		let id = TokenId(self.tags.len() as u32);
		self.tags.push(tag);
		self.syms.push(sym);
		self.starts.push(start);
		self.ends.push(end);
		self.nl_befores.push(nl_before);
		id
	}

	fn tag(&self, id: TokenId) -> TokenTag {
		self.tags[id.index()]
	}

	fn sym(&self, id: TokenId) -> Option<Sym> {
		self.syms[id.index()]
	}

	fn start(&self, id: TokenId) -> usize {
		self.starts[id.index()]
	}

	fn end(&self, id: TokenId) -> usize {
		self.ends[id.index()]
	}

	fn nl_before(&self, id: TokenId) -> bool {
		self.nl_befores[id.index()]
	}
}

pub struct Lexer<'syms, 'src> {
	syms: &'syms mut Interner,
	src: &'src Source,
	pos: usize,
}

impl<'syms, 'src> Lexer<'syms, 'src> {
	pub fn new(syms: &'syms mut Interner, src: &'src Source) -> Self {
		Self { syms, src, pos: 0 }
	}

	pub fn lex(mut self) -> Result<Tokens, Error> {
		let mut toks = Tokens::new();
		loop {
			let (tok, nl_before) = self.lex_next_skip_space()?;
			toks.push(tok.tag, tok.sym, tok.pos, tok.end, nl_before);
			if tok.tag == TokenTag::Eof {
				break;
			}
		}
		Ok(toks)
	}

	fn lex_next_skip_space(&mut self) -> Result<(Token, bool), Error> {
		let mut nl_before = false;
		while self.pos < self.src.len() {
			if self.src[self.pos] == b'\n' {
				nl_before = true;
				self.pos += 1;
			} else if self.src[self.pos].is_ascii_whitespace() {
				self.pos += 1;
			} else if self.src[self.pos] == b'#' {
				while self.pos < self.src.len() && self.src[self.pos] != b'\n' {
					self.pos += 1;
				}
			} else {
				break;
			}
		}
		let tok = self.lex_next()?;
		Ok((tok, nl_before))
	}

	fn lex_next(&mut self) -> Result<Token, Error> {
		if self.pos == self.src.len() {
			return Ok(Token {
				src: self.src.id,
				tag: TokenTag::Eof,
				sym: None,
				pos: self.pos,
				end: self.pos,
			});
		}

		match self.src[self.pos] {
			b':' => {
				if self.pos + 1 < self.src.len() && self.src[self.pos + 1] == b'=' {
					self.pos += 2;
					Ok(Token {
						src: self.src.id,
						tag: TokenTag::Eq,
						sym: None,
						pos: self.pos - 2,
						end: self.pos,
					})
				} else {
					self.pos += 1;
					Ok(Token {
						src: self.src.id,
						tag: TokenTag::Colon,
						sym: None,
						pos: self.pos - 1,
						end: self.pos,
					})
				}
			}
			b'=' => {
				if self.pos + 1 < self.src.len() && self.src[self.pos + 1] == b'=' {
					self.pos += 2;
					Ok(Token {
						src: self.src.id,
						tag: TokenTag::EqEq,
						sym: None,
						pos: self.pos - 2,
						end: self.pos,
					})
				} else {
					Err(Error::UnexpectedChar(self.src.loc(self.pos), '='))
				}
			}
			b'!' => {
				if self.pos + 1 < self.src.len() && self.src[self.pos + 1] == b'=' {
					self.pos += 2;
					Ok(Token {
						src: self.src.id,
						tag: TokenTag::NotEq,
						sym: None,
						pos: self.pos - 2,
						end: self.pos,
					})
				} else {
					Err(Error::UnexpectedChar(self.src.loc(self.pos), '!'))
				}
			}
			b'<' => {
				if self.pos + 1 < self.src.len() && self.src[self.pos + 1] == b'=' {
					self.pos += 2;
					Ok(Token {
						src: self.src.id,
						tag: TokenTag::LtEq,
						sym: None,
						pos: self.pos - 2,
						end: self.pos,
					})
				} else if self.pos + 1 < self.src.len() && self.src[self.pos + 1] == b'<' {
					self.pos += 2;
					Ok(Token {
						src: self.src.id,
						tag: TokenTag::LtLt,
						sym: None,
						pos: self.pos - 2,
						end: self.pos,
					})
				} else {
					self.pos += 1;
					Ok(Token {
						src: self.src.id,
						tag: TokenTag::Lt,
						sym: None,
						pos: self.pos - 1,
						end: self.pos,
					})
				}
			}
			b'>' => {
				if self.pos + 1 < self.src.len() && self.src[self.pos + 1] == b'=' {
					self.pos += 2;
					Ok(Token {
						src: self.src.id,
						tag: TokenTag::GtEq,
						sym: None,
						pos: self.pos - 2,
						end: self.pos,
					})
				} else {
					self.pos += 1;
					Ok(Token {
						src: self.src.id,
						tag: TokenTag::Gt,
						sym: None,
						pos: self.pos - 1,
						end: self.pos,
					})
				}
			}
			b'+' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::Plus,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'-' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::Minus,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'*' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::Star,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'/' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::Slash,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'&' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::Amp,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'{' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::LBrace,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'}' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::RBrace,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'[' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::LBrack,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b']' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::RBrack,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'(' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::LParen,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b')' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::RParen,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b',' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::Comma,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'.' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::Dot,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'"' => self.lex_str(),
			b'\'' => self.lex_char(),
			b'$' => self.lex_builtin(),
			char => {
				if char.is_ascii_alphabetic() || char == b'_' {
					self.lex_ident()
				} else if char.is_ascii_digit() {
					self.lex_num()
				} else {
					let char = self.src[self.pos..self.src.len()].chars().next().unwrap();
					Err(Error::UnexpectedChar(self.src.loc(self.pos), char))
				}
			}
		}
	}

	fn lex_ident(&mut self) -> Result<Token, Error> {
		let pos = self.pos;
		while self.pos < self.src.len()
			&& (self.src[self.pos].is_ascii_alphabetic()
				|| self.src[self.pos].is_ascii_digit()
				|| self.src[self.pos] == b'_')
		{
			self.pos += 1;
		}
		if self.pos < self.src.len() {
			if self.src[self.pos] == b'!'
				&& !(self.pos + 1 < self.src.len()
					&& self.src[self.pos + 1] == b'='
					&& !(self.pos + 2 < self.src.len() && self.src[self.pos + 2] == b'='))
			{
				self.pos += 1;
			} else if self.src[self.pos] == b'?' {
				self.pos += 1;
			}
		}
		let span = &self.src[pos..self.pos];
		let sym = self.syms.intern(span);
		let tag = match sym {
			Sym::TYPE => TokenTag::Type,
			Sym::CASE => TokenTag::Case,
			Sym::DEF => TokenTag::Def,
			Sym::EACH => TokenTag::Each,
			Sym::LOOP => TokenTag::Loop,
			Sym::DO => TokenTag::Do,
			Sym::IN => TokenTag::In,
			Sym::WHEN => TokenTag::When,
			Sym::THEN => TokenTag::Then,
			Sym::ELSE => TokenTag::Else,
			Sym::END => TokenTag::End,
			Sym::RETURN => TokenTag::Return,
			Sym::BREAK => TokenTag::Break,
			Sym::AND => TokenTag::And,
			Sym::OR => TokenTag::Or,
			Sym::NOT => TokenTag::Not,
			Sym::SELF => TokenTag::Self_,
			Sym::TRUE | Sym::FALSE => TokenTag::Bool,
			Sym::NIL => TokenTag::Nil,
			_ => TokenTag::Ident,
		};
		Ok(Token {
			src: self.src.id,
			tag,
			sym: Some(sym),
			pos,
			end: self.pos,
		})
	}

	fn lex_builtin(&mut self) -> Result<Token, Error> {
		let pos = self.pos;
		self.pos += 1;
		let ident = self.lex_ident()?;
		Ok(Token {
			src: self.src.id,
			tag: TokenTag::Builtin,
			sym: ident.sym,
			pos,
			end: ident.end,
		})
	}

	fn lex_str(&mut self) -> Result<Token, Error> {
		let pos = self.pos;
		self.pos += 1;
		while self.pos < self.src.len() && self.src[self.pos] != b'"' {
			if self.src[self.pos] == b'\\' && self.pos + 1 < self.src.len() {
				self.pos += 2;
			} else {
				self.pos += 1;
			}
		}
		if self.pos == self.src.len() {
			return Err(Error::UnterminatedStrLit(self.src.loc(pos)));
		}
		self.pos += 1;
		Ok(Token {
			src: self.src.id,
			tag: TokenTag::Str,
			sym: None,
			pos,
			end: self.pos,
		})
	}

	fn lex_char(&mut self) -> Result<Token, Error> {
		let pos = self.pos;
		self.pos += 1;
		while self.pos < self.src.len() && self.src[self.pos] != b'\'' {
			if self.src[self.pos] == b'\\' && self.pos + 1 < self.src.len() {
				self.pos += 2;
			} else {
				self.pos += 1;
			}
		}
		if self.pos == self.src.len() {
			return Err(Error::UnterminatedCharLit(self.src.loc(pos)));
		}
		self.pos += 1;
		Ok(Token {
			src: self.src.id,
			tag: TokenTag::Char,
			sym: None,
			pos,
			end: self.pos,
		})
	}

	fn lex_num(&mut self) -> Result<Token, Error> {
		let pos = self.pos;
		while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
			self.pos += 1;
		}
		if self.pos < self.src.len() && self.src[self.pos] == b'.' {
			self.pos += 1;
			while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
				self.pos += 1;
			}
		}
		Ok(Token {
			src: self.src.id,
			tag: TokenTag::Num,
			sym: None,
			pos,
			end: self.pos,
		})
	}
}

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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
	Type(Type),
	Def(Def),
	Expr(ExprId),
}

#[derive(Debug)]
pub struct Type(pub Sym, pub Vec<Param>, pub Vec<TypeItemId>);

#[derive(Debug)]
pub struct Def(pub Sym, pub Vec<Param>, pub BlockId);

#[derive(Debug, Clone, Copy)]
pub struct Param(pub Sym, pub Option<ExprId>);

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
pub struct Field(pub Sym, pub ExprId);

#[derive(Debug)]
pub enum Method {
	Instance(Def),
	Static(Def),
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
pub struct Each(pub Sym, pub ExprId, pub BlockId);

#[derive(Debug)]
pub struct Loop(pub BlockId);

#[derive(Debug)]
pub struct When(pub ExprId, pub BlockId, pub Option<BlockId>);

#[derive(Debug)]
pub struct Return(pub Option<ExprId>);

#[derive(Debug)]
pub struct Break(pub Option<ExprId>);

#[derive(Debug)]
pub struct Call(pub ExprId, pub Vec<Arg>);

#[derive(Debug, Clone, Copy)]
pub struct Arg(pub Option<Sym>, pub ExprId);

#[derive(Debug, Clone)]
pub struct Member(pub ExprId, pub Sym);

#[derive(Debug, Clone)]
pub struct Access(pub ExprId, pub ExprId);

#[derive(Debug)]
pub struct Mention(pub ExprId);

#[derive(Debug)]
pub struct Assign(pub Place, pub ExprId);

#[derive(Debug, Clone)]
pub enum Place {
	Name(Name),
	Member(Member),
	Access(Access),
}

#[derive(Debug, Clone)]
pub struct Name(pub Sym);

#[derive(Debug, Clone)]
pub enum Builtin {
	Print(ExprId),
}

#[derive(Debug)]
pub struct Binary(pub BinaryOp, pub ExprId, pub ExprId);

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
pub struct Unary(pub UnaryOp, pub ExprId);

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
pub struct Block(pub Vec<ExprId>);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ChunkId(u32);

#[derive(Debug)]
pub struct Chunk {
	pub src: SourceId,
	pub top: Vec<ModuleItemId>,
	module_items: Nodes<ModuleItem, ModuleItemId>,
	type_items: Nodes<TypeItem, TypeItemId>,
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

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
struct Precedence(u8);

impl Precedence {
	const NONE: Precedence = Precedence(0);
	const ASSIGN: Precedence = Precedence(1);
	const OR: Precedence = Precedence(2);
	const AND: Precedence = Precedence(3);
	const NOT: Precedence = Precedence(4);
	const COMPARE: Precedence = Precedence(5);
	const APPEND: Precedence = Precedence(6);
	const ADD: Precedence = Precedence(7);
	const SUB: Precedence = Precedence(7);
	const MUL: Precedence = Precedence(8);
	const DIV: Precedence = Precedence(8);
	const NEG: Precedence = Precedence(9);
	const MENTION: Precedence = Precedence(9);
	const CALL: Precedence = Precedence(10);
	const MEMBER: Precedence = Precedence(10);
	const ACCESS: Precedence = Precedence(10);

	fn of(tag: TokenTag) -> Self {
		match tag {
			TokenTag::Eq => Precedence::ASSIGN,
			TokenTag::Or => Precedence::OR,
			TokenTag::And => Precedence::AND,
			TokenTag::EqEq => Precedence::COMPARE,
			TokenTag::NotEq => Precedence::COMPARE,
			TokenTag::Lt => Precedence::COMPARE,
			TokenTag::Gt => Precedence::COMPARE,
			TokenTag::LtEq => Precedence::COMPARE,
			TokenTag::GtEq => Precedence::COMPARE,
			TokenTag::LtLt => Precedence::APPEND,
			TokenTag::Plus => Precedence::ADD,
			TokenTag::Minus => Precedence::SUB,
			TokenTag::Star => Precedence::MUL,
			TokenTag::Slash => Precedence::DIV,
			TokenTag::LParen => Precedence::CALL,
			TokenTag::Dot => Precedence::MEMBER,
			TokenTag::LBrack => Precedence::ACCESS,
			_ => Precedence::NONE,
		}
	}
}

fn terminates_expr(tag: TokenTag) -> bool {
	match tag {
		TokenTag::End => true,
		TokenTag::Ident => true,
		TokenTag::Str => true,
		TokenTag::Char => true,
		TokenTag::Num => true,
		TokenTag::Bool => true,
		TokenTag::Nil => true,
		TokenTag::RBrace => true,
		TokenTag::RBrack => true,
		TokenTag::RParen => true,
		_ => false,
	}
}

fn starts_expr(tag: TokenTag) -> bool {
	match tag {
		TokenTag::LParen => true,
		TokenTag::When => true,
		TokenTag::Each => true,
		TokenTag::Loop => true,
		TokenTag::Return => true,
		TokenTag::Break => true,
		TokenTag::Self_ => true,
		TokenTag::Not => true,
		TokenTag::Minus => true,
		TokenTag::Amp => true,
		TokenTag::Ident => true,
		TokenTag::Builtin => true,
		TokenTag::Str => true,
		TokenTag::Char => true,
		TokenTag::Num => true,
		TokenTag::Bool => true,
		TokenTag::LBrack => true,
		TokenTag::LBrace => true,
		TokenTag::Nil => true,
		_ => false,
	}
}

fn continues_expr(tag: TokenTag) -> bool {
	match tag {
		TokenTag::Or => true,
		TokenTag::And => true,
		TokenTag::EqEq => true,
		TokenTag::NotEq => true,
		TokenTag::Lt => true,
		TokenTag::Gt => true,
		TokenTag::LtEq => true,
		TokenTag::GtEq => true,
		TokenTag::LtLt => true,
		TokenTag::Plus => true,
		TokenTag::Star => true,
		TokenTag::Slash => true,
		TokenTag::Dot => true,
		_ => false,
	}
}

pub struct Parser<'src> {
	src: &'src Source,
	toks: Tokens,
	cur: TokenId,
	chunk: Chunk,
}

impl<'src> Parser<'src> {
	pub fn new(src: &'src Source, toks: Tokens) -> Self {
		Self {
			src,
			toks,
			cur: TokenId(0),
			chunk: Chunk::new(src.id),
		}
	}

	fn tag(&self) -> TokenTag {
		self.toks.tag(self.cur)
	}

	fn tok(&self, id: TokenId) -> Token {
		Token {
			src: self.src.id,
			tag: self.toks.tag(id),
			sym: self.toks.sym(id),
			pos: self.toks.start(id),
			end: self.toks.end(id),
		}
	}

	fn unexpected(&self) -> Error {
		Error::UnexpectedToken(
			self.src.loc(self.toks.start(self.cur)),
			self.toks.tag(self.cur),
		)
	}

	pub fn parse(mut self) -> Result<Chunk, Error> {
		while self.cur.index() < self.toks.len() && self.tag() != TokenTag::Eof {
			let item_id = self.parse_module_item()?;
			self.chunk.add_to_top(item_id);
		}
		Ok(self.chunk)
	}

	fn parse_module_item(&mut self) -> Result<ModuleItemId, Error> {
		match self.tag() {
			TokenTag::Type => self.parse_type_in_module(),
			TokenTag::Def => self.parse_def_decl(),
			_ => {
				let expr_id = self.parse_expr()?;
				let span = self.chunk.get_expr_span(expr_id);
				let item = ModuleItem::Expr(expr_id);
				let item_id = self.chunk.add_module_item(span, item);
				Ok(item_id)
			}
		}
	}

	fn parse_type_in_module(&mut self) -> Result<ModuleItemId, Error> {
		let (type_, span) = self.parse_type(TokenTag::Type, true)?;
		let item = ModuleItem::Type(type_);
		let item_id = self.chunk.add_module_item(span, item);
		Ok(item_id)
	}

	fn parse_type_in_type(&mut self) -> Result<TypeItemId, Error> {
		let (type_, span) = self.parse_type(TokenTag::Type, true)?;
		let item = TypeItem::Type(type_);
		let item_id = self.chunk.add_type_item(span, item);
		Ok(item_id)
	}

	fn parse_case_in_type(&mut self) -> Result<TypeItemId, Error> {
		let (type_, span) = self.parse_type(TokenTag::Case, false)?;
		let item = TypeItem::Case(type_);
		let item_id = self.chunk.add_type_item(span, item);
		Ok(item_id)
	}

	// A case clause is field-for-field a type, so both parse here. Cases nest
	// one level only: inside a case, `case` falls through to parse_type_item's
	// catch-all.
	fn parse_type(&mut self, open: TokenTag, allow_cases: bool) -> Result<(Type, Span), Error> {
		let tok = self.take(open)?;
		let ident = self.take(TokenTag::Ident)?;
		let name = ident.sym.unwrap();
		let last = self.src[ident.end - 1];
		if last == b'!' || last == b'?' {
			return Err(Error::UnexpectedChar(
				self.src.loc(ident.end - 1),
				last as char,
			));
		}
		let fields = self.parse_params()?;
		let mut items = Vec::new();
		let mut seen_field = false;
		let mut seen_method = false;
		while self.cur.index() < self.toks.len() && self.tag() != TokenTag::End {
			let item_id = self.parse_type_item(allow_cases)?;
			let span = self.chunk.get_type_item_span(item_id);
			match self.chunk.get_type_item(item_id) {
				TypeItem::Case(..) => {
					if seen_field || seen_method {
						return Err(Error::UnexpectedToken(
							self.src.loc(span.start),
							TokenTag::Case,
						));
					}
				}
				TypeItem::Field(..) => {
					if seen_method {
						return Err(Error::UnexpectedToken(
							self.src.loc(span.start),
							TokenTag::Ident,
						));
					}
					seen_field = true;
				}
				TypeItem::Type(..) | TypeItem::Method(..) => seen_method = true,
			}
			items.push(item_id);
		}
		self.take(TokenTag::End)?;
		Ok((Type(name, fields, items), tok.into()))
	}

	fn parse_type_item(&mut self, allow_cases: bool) -> Result<TypeItemId, Error> {
		match self.tag() {
			TokenTag::Case if allow_cases => self.parse_case_in_type(),
			TokenTag::Type => self.parse_type_in_type(),
			TokenTag::Def => self.parse_method_decl(),
			TokenTag::Ident => self.parse_field_decl(),
			_ => Err(self.unexpected()),
		}
	}

	fn parse_field_decl(&mut self) -> Result<TypeItemId, Error> {
		let tok = self.take(TokenTag::Ident)?;
		let name = tok.sym.unwrap();
		self.take(TokenTag::Eq)?;
		let expr_id = self.parse_expr()?;
		let item = TypeItem::Field(Field(name, expr_id));
		let item_id = self.chunk.add_type_item(tok.into(), item);
		Ok(item_id)
	}

	fn parse_method_decl(&mut self) -> Result<TypeItemId, Error> {
		let tok = self.take(TokenTag::Def)?;
		let is_static = self.tag() == TokenTag::Self_;
		let name = if is_static {
			self.take(TokenTag::Self_)?;
			self.take(TokenTag::Dot)?;
			self.take(TokenTag::Ident)?.sym.unwrap()
		} else {
			self.take(TokenTag::Ident)?.sym.unwrap()
		};
		let params = self.parse_params()?;
		let mut body = Vec::new();
		while self.cur.index() < self.toks.len() && self.tag() != TokenTag::End {
			let expr_id = self.parse_expr()?;
			body.push(expr_id);
		}
		self.take(TokenTag::End)?;
		let block = self.chunk.add_block(Block(body));
		let def = Def(name, params, block);
		let method = if is_static {
			Method::Static(def)
		} else {
			Method::Instance(def)
		};
		let item = TypeItem::Method(method);
		let item_id = self.chunk.add_type_item(tok.into(), item);
		Ok(item_id)
	}

	fn parse_def_decl(&mut self) -> Result<ModuleItemId, Error> {
		let tok = self.take(TokenTag::Def)?;
		let name = self.take(TokenTag::Ident)?.sym.unwrap();
		let params = self.parse_params()?;
		let mut body = Vec::new();
		while self.cur.index() < self.toks.len() && self.tag() != TokenTag::End {
			let expr_id = self.parse_expr()?;
			body.push(expr_id);
		}
		self.take(TokenTag::End)?;
		let block = self.chunk.add_block(Block(body));
		let item = ModuleItem::Def(Def(name, params, block));
		let item_id = self.chunk.add_module_item(tok.into(), item);
		Ok(item_id)
	}

	fn parse_params(&mut self) -> Result<Vec<Param>, Error> {
		let mut params = Vec::new();
		if self.tag() != TokenTag::LParen {
			return Ok(params);
		}
		self.take(TokenTag::LParen)?;
		while self.cur.index() < self.toks.len() && self.tag() != TokenTag::RParen {
			let name = self.take(TokenTag::Ident)?.sym.unwrap();
			let default = if self.tag() == TokenTag::Colon {
				self.take(TokenTag::Colon)?;
				Some(self.parse_expr()?)
			} else {
				None
			};
			params.push(Param(name, default));
			match self.tag() {
				TokenTag::Comma => {
					self.cur = self.cur.next();
				}
				TokenTag::RParen => {}
				_ => return Err(self.unexpected()),
			}
		}
		self.take(TokenTag::RParen)?;
		Ok(params)
	}

	fn parse_expr(&mut self) -> Result<ExprId, Error> {
		self.parse_expr_prec(Precedence::NONE)
	}

	fn parse_expr_prec(&mut self, min_prec: Precedence) -> Result<ExprId, Error> {
		let mut expr_id = self.parse_expr_unit()?;
		while Precedence::of(self.tag()) > min_prec {
			if self.toks.nl_before(self.cur)
				&& terminates_expr(self.toks.tag(self.cur.prev()))
				&& !continues_expr(self.tag())
			{
				break;
			}
			expr_id = match self.tag() {
				TokenTag::Eq => self.parse_assign_expr(expr_id)?,
				TokenTag::Or => self.parse_binary_expr(expr_id, BinaryOp::Or, Precedence::OR)?,
				TokenTag::And => self.parse_binary_expr(expr_id, BinaryOp::And, Precedence::AND)?,
				TokenTag::EqEq => {
					self.parse_binary_expr(expr_id, BinaryOp::Eq, Precedence::COMPARE)?
				}
				TokenTag::NotEq => {
					self.parse_binary_expr(expr_id, BinaryOp::NotEq, Precedence::COMPARE)?
				}
				TokenTag::Lt => {
					self.parse_binary_expr(expr_id, BinaryOp::Lt, Precedence::COMPARE)?
				}
				TokenTag::Gt => {
					self.parse_binary_expr(expr_id, BinaryOp::Gt, Precedence::COMPARE)?
				}
				TokenTag::LtEq => {
					self.parse_binary_expr(expr_id, BinaryOp::LtEq, Precedence::COMPARE)?
				}
				TokenTag::GtEq => {
					self.parse_binary_expr(expr_id, BinaryOp::GtEq, Precedence::COMPARE)?
				}
				TokenTag::LtLt => {
					self.parse_binary_expr(expr_id, BinaryOp::Append, Precedence::APPEND)?
				}
				TokenTag::Plus => {
					self.parse_binary_expr(expr_id, BinaryOp::Add, Precedence::ADD)?
				}
				TokenTag::Minus => {
					self.parse_binary_expr(expr_id, BinaryOp::Sub, Precedence::SUB)?
				}
				TokenTag::Star => {
					self.parse_binary_expr(expr_id, BinaryOp::Mul, Precedence::MUL)?
				}
				TokenTag::Slash => {
					self.parse_binary_expr(expr_id, BinaryOp::Div, Precedence::DIV)?
				}
				TokenTag::LParen => self.parse_call_expr(expr_id)?,
				TokenTag::Dot => self.parse_member_expr(expr_id)?,
				TokenTag::LBrack => self.parse_access_expr(expr_id)?,
				_ => break,
			};
		}
		Ok(expr_id)
	}

	fn parse_member_expr(&mut self, val_id: ExprId) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Dot)?;
		let field = self.take(TokenTag::Ident)?.sym.unwrap();
		let expr = Expr::Member(Member(val_id, field));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_access_expr(&mut self, val_id: ExprId) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::LBrack)?;
		let key_id = self.parse_expr()?;
		self.take(TokenTag::RBrack)?;
		let expr = Expr::Access(Access(val_id, key_id));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_call_expr(&mut self, val_id: ExprId) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::LParen)?;
		let mut args = Vec::new();
		let mut seen_keyword = false;
		while self.cur.index() < self.toks.len() && self.tag() != TokenTag::RParen {
			let name = if self.tag() == TokenTag::Ident
				&& self.toks.tag(self.cur.next()) == TokenTag::Colon
			{
				let name = self.take(TokenTag::Ident)?.sym.unwrap();
				self.take(TokenTag::Colon)?;
				seen_keyword = true;
				Some(name)
			} else if seen_keyword {
				let loc = self.src.loc(self.toks.start(self.cur));
				return Err(Error::PositionalAfterKeyword(loc));
			} else {
				None
			};
			let arg = self.parse_expr()?;
			args.push(Arg(name, arg));
			match self.tag() {
				TokenTag::Comma => {
					self.cur = self.cur.next();
				}
				TokenTag::RParen => {}
				_ => return Err(self.unexpected()),
			}
		}
		self.take(TokenTag::RParen)?;
		let expr = Expr::Call(Call(val_id, args));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_assign_expr(&mut self, val_id: ExprId) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Eq)?;
		let place = match self.chunk.get_expr(val_id) {
			Expr::Name(name) => Place::Name(name.clone()),
			Expr::Member(member) => Place::Member(member.clone()),
			Expr::Access(access) => Place::Access(access.clone()),
			_ => return Err(Error::UnexpectedToken(self.src.loc(tok.pos), tok.tag)),
		};
		let val_expr_id = self.parse_expr_prec(Precedence::NONE)?;
		let expr = Expr::Assign(Assign(place, val_expr_id));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_binary_expr(
		&mut self,
		val_id: ExprId,
		op: BinaryOp,
		prec: Precedence,
	) -> Result<ExprId, Error> {
		let tok = self.tok(self.cur);
		self.cur = self.cur.next();
		let rhs = self.parse_expr_prec(prec)?;
		let expr = Expr::Binary(Binary(op, val_id, rhs));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_unary_expr(
		&mut self,
		tag: TokenTag,
		op: UnaryOp,
		prec: Precedence,
	) -> Result<ExprId, Error> {
		let tok = self.take(tag)?;
		let val_id = self.parse_expr_prec(prec)?;
		let expr = Expr::Unary(Unary(op, val_id));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_mention_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Amp)?;
		let val_id = self.parse_expr_prec(Precedence::MENTION)?;
		let expr = Expr::Mention(Mention(val_id));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_expr_unit(&mut self) -> Result<ExprId, Error> {
		match self.tag() {
			TokenTag::LParen => self.parse_group_expr(),
			TokenTag::When => self.parse_when_expr(),
			TokenTag::Each => self.parse_each_expr(),
			TokenTag::Loop => self.parse_loop_expr(),
			TokenTag::Return => self.parse_return_expr(),
			TokenTag::Break => self.parse_break_expr(),
			TokenTag::Self_ => self.parse_self_expr(),
			TokenTag::Not => self.parse_unary_expr(TokenTag::Not, UnaryOp::Not, Precedence::NOT),
			TokenTag::Minus => {
				self.parse_unary_expr(TokenTag::Minus, UnaryOp::Neg, Precedence::NEG)
			}
			TokenTag::Amp => self.parse_mention_expr(),
			TokenTag::Ident => self.parse_name_expr(),
			TokenTag::Builtin => self.parse_builtin_expr(),
			TokenTag::Str => self.parse_str_lit_expr(),
			TokenTag::Char => self.parse_char_lit_expr(),
			TokenTag::Num => self.parse_num_lit_expr(),
			TokenTag::Bool => self.parse_bool_lit_expr(),
			TokenTag::LBrack => self.parse_list_lit_expr(),
			TokenTag::LBrace => self.parse_dict_lit_expr(),
			TokenTag::Nil => self.parse_nil_lit_expr(),
			_ => Err(self.unexpected()),
		}
	}

	fn parse_group_expr(&mut self) -> Result<ExprId, Error> {
		self.take(TokenTag::LParen)?;
		let expr_id = self.parse_expr_prec(Precedence::NONE)?;
		self.take(TokenTag::RParen)?;
		Ok(expr_id)
	}

	fn parse_each_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Each)?;
		let item = self.take(TokenTag::Ident)?.sym.unwrap();
		self.take(TokenTag::In)?;
		let iter = self.parse_expr()?;
		self.take(TokenTag::Do)?;
		let mut body = Vec::new();
		while self.cur.index() < self.toks.len() && self.tag() != TokenTag::End {
			let expr_id = self.parse_expr()?;
			body.push(expr_id);
		}
		self.take(TokenTag::End)?;
		let block = self.chunk.add_block(Block(body));
		let expr = Expr::Each(Each(item, iter, block));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_loop_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Loop)?;
		self.take(TokenTag::Do)?;
		let mut body = Vec::new();
		while self.cur.index() < self.toks.len() && self.tag() != TokenTag::End {
			let expr_id = self.parse_expr()?;
			body.push(expr_id);
		}
		self.take(TokenTag::End)?;
		let block = self.chunk.add_block(Block(body));
		let expr = Expr::Loop(Loop(block));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_return_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Return)?;
		let val_expr_id = if self.toks.nl_before(self.cur) || !starts_expr(self.tag()) {
			None
		} else {
			Some(self.parse_expr()?)
		};
		let expr = Expr::Return(Return(val_expr_id));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_break_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Break)?;
		let val_expr_id = if self.toks.nl_before(self.cur) || !starts_expr(self.tag()) {
			None
		} else {
			Some(self.parse_expr()?)
		};
		let expr = Expr::Break(Break(val_expr_id));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_when_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::When)?;
		let cond = self.parse_expr()?;
		self.take(TokenTag::Then)?;
		let mut then_branch = Vec::new();
		while self.cur.index() < self.toks.len()
			&& self.tag() != TokenTag::End
			&& self.tag() != TokenTag::Else
		{
			let expr_id = self.parse_expr()?;
			then_branch.push(expr_id);
		}
		let then_branch = self.chunk.add_block(Block(then_branch));
		let (else_branch, consume_end) = if self.tag() == TokenTag::Else {
			self.take(TokenTag::Else)?;
			if self.tag() == TokenTag::When {
				let nested_when = self.parse_when_expr()?;
				let else_branch = self.chunk.add_block(Block(vec![nested_when]));
				(Some(else_branch), false)
			} else {
				let mut else_branch = Vec::new();
				while self.cur.index() < self.toks.len() && self.tag() != TokenTag::End {
					let expr_id = self.parse_expr()?;
					else_branch.push(expr_id);
				}
				let else_branch = self.chunk.add_block(Block(else_branch));
				(Some(else_branch), true)
			}
		} else {
			(None, true)
		};
		if consume_end {
			self.take(TokenTag::End)?;
		}
		let expr = Expr::When(When(cond, then_branch, else_branch));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_self_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Self_)?;
		let expr_id = self.chunk.add_expr(tok.into(), Expr::Self_);
		Ok(expr_id)
	}

	fn parse_name_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Ident)?;
		let sym = tok.sym.unwrap();
		let expr = Expr::Name(Name(sym));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_builtin_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Builtin)?;
		let span = &self.src[tok.pos..tok.end];
		match span {
			"$print" => {
				self.take(TokenTag::LParen)?;
				let val = self.parse_expr()?;
				self.take(TokenTag::RParen)?;
				let expr = Expr::Builtin(Builtin::Print(val));
				let expr_id = self.chunk.add_expr(tok.into(), expr);
				Ok(expr_id)
			}
			_ => Err(Error::UnknownBuiltin(
				self.src.loc(tok.pos),
				span.to_string(),
			)),
		}
	}

	fn parse_str_lit_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Str)?;
		let str = self.unescape_quoted_lit(&tok, b'"')?;
		let expr = Expr::Lit(Lit::Str(str));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_char_lit_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Char)?;
		let str = self.unescape_quoted_lit(&tok, b'\'')?;
		let mut chars = str.chars();
		let char = match chars.next() {
			Some(char) if chars.next().is_none() => char,
			_ => return Err(Error::MultiCharLit(self.src.loc(tok.pos))),
		};
		let expr = Expr::Lit(Lit::Char(char));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn unescape_quoted_lit(&self, tok: &Token, quote: u8) -> Result<String, Error> {
		let mut pos = tok.pos + 1;
		let mut str = String::with_capacity(tok.end - tok.pos);
		let mut chunk_pos = pos;
		while pos < tok.end - 1 {
			if self.src[pos] == b'\\' {
				str.push_str(&self.src[chunk_pos..pos]);
				let esc = self.src[pos + 1];
				match esc {
					b'n' => str.push('\n'),
					b't' => str.push('\t'),
					b'\\' => str.push('\\'),
					esc if esc == quote => str.push(quote as char),
					_ => {
						let char = self.src[pos..self.src.len()].chars().next().unwrap();
						return Err(Error::UnknownEsc(self.src.loc(pos), char));
					}
				}
				pos += 2;
				chunk_pos = pos;
			} else {
				pos += 1;
			}
		}
		str.push_str(&self.src[chunk_pos..pos]);
		Ok(str)
	}

	fn parse_num_lit_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Num)?;
		let span = &self.src[tok.pos..tok.end];
		let num = span.parse().unwrap();
		let expr = Expr::Lit(Lit::Num(num));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_bool_lit_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Bool)?;
		let span = &self.src[tok.pos..tok.end];
		let bool = span.parse().unwrap();
		let expr = Expr::Lit(Lit::Bool(bool));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_list_lit_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::LBrack)?;
		let mut items = Vec::new();
		while self.cur.index() < self.toks.len() && self.tag() != TokenTag::RBrack {
			let item = self.parse_expr()?;
			items.push(item);
			match self.tag() {
				TokenTag::Comma => {
					self.cur = self.cur.next();
				}
				TokenTag::RBrack => {}
				_ => return Err(self.unexpected()),
			}
		}
		self.take(TokenTag::RBrack)?;
		let expr = Expr::Lit(Lit::List(items));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_dict_lit_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::LBrace)?;
		let mut pairs = Vec::new();
		while self.cur.index() < self.toks.len() && self.tag() != TokenTag::RBrace {
			let key = self.parse_expr()?;
			self.take(TokenTag::Colon)?;
			let val = self.parse_expr()?;
			pairs.push((key, val));
			match self.tag() {
				TokenTag::Comma => {
					self.cur = self.cur.next();
				}
				TokenTag::RBrace => {}
				_ => return Err(self.unexpected()),
			}
		}
		self.take(TokenTag::RBrace)?;
		let expr = Expr::Lit(Lit::Dict(pairs));
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_nil_lit_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Nil)?;
		let expr = Expr::Lit(Lit::Nil);
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn take(&mut self, tag: TokenTag) -> Result<Token, Error> {
		if self.tag() == tag {
			let tok = self.tok(self.cur);
			self.cur = self.cur.next();
			Ok(tok)
		} else {
			Err(self.unexpected())
		}
	}
}
