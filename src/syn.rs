use std::fmt::{self, Display, Formatter};
use std::ops::{Index, Range};

use crate::intern::{Interner, Sym};

#[derive(Debug)]
pub enum Error {
	UnexpectedChar(Location, char),
	UnexpectedToken(Location, Token),
	UnterminatedStrLit(Location),
	UnsupportedStrEsc(Location, char),
	UnknownBuiltin(Location, String),
}

impl Error {
	fn loc(&self) -> &Location {
		match self {
			Self::UnexpectedChar(loc, _) => loc,
			Self::UnexpectedToken(loc, _) => loc,
			Self::UnterminatedStrLit(loc) => loc,
			Self::UnsupportedStrEsc(loc, _) => loc,
			Self::UnknownBuiltin(loc, _) => loc,
		}
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		write!(f, "{}: syntax error: ", self.loc())?;
		match self {
			Self::UnexpectedChar(_, char) => write!(f, "unexpected char '{}'", char),
			Self::UnexpectedToken(_, tok) => write!(f, "unexpected token {}", tok.tag.name()),
			Self::UnterminatedStrLit(_) => write!(f, "unterminated string literal"),
			Self::UnsupportedStrEsc(_, esc) => write!(f, "unsupported escape sequence '\\{}'", esc),
			Self::UnknownBuiltin(_, builtin) => write!(f, "unknown builtin '{}'", builtin),
		}
	}
}

#[derive(Debug)]
pub struct Package {
	srcs: Vec<Source>,
}

impl Package {
	pub fn new() -> Self {
		Self { srcs: Vec::new() }
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TokenTag {
	Eof,

	Type,
	Def,
	Each,
	Do,
	In,
	When,
	Then,
	Else,
	End,
	Return,

	Ident,
	Builtin,
	Str,
	Num,
	Bool,
	Nil,

	Eq,

	Plus,
	Minus,
	Star,
	Slash,

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
			TokenTag::Def => "DEF",
			TokenTag::Each => "EACH",
			TokenTag::Do => "DO",
			TokenTag::In => "IN",
			TokenTag::When => "WHEN",
			TokenTag::Then => "THEN",
			TokenTag::Else => "ELSE",
			TokenTag::End => "END",
			TokenTag::Return => "RETURN",

			TokenTag::Ident => "IDENT",
			TokenTag::Builtin => "BUILTIN",
			TokenTag::Str => "STR",
			TokenTag::Num => "NUM",
			TokenTag::Bool => "BOOL",
			TokenTag::Nil => "NIL",

			TokenTag::Eq => "EQ",

			TokenTag::Plus => "PLUS",
			TokenTag::Minus => "MINUS",
			TokenTag::Star => "STAR",
			TokenTag::Slash => "SLASH",

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

pub struct Lexer<'syms, 'src> {
	syms: &'syms mut Interner,
	src: &'src Source,
	pos: usize,
}

impl<'syms, 'src> Lexer<'syms, 'src> {
	pub fn new(syms: &'syms mut Interner, src: &'src Source) -> Self {
		Self { syms, src, pos: 0 }
	}

	pub fn lex(mut self) -> Result<Vec<Token>, Error> {
		let mut toks = Vec::new();
		loop {
			let tok = self.lex_next()?;
			toks.push(tok);
			if tok.tag == TokenTag::Eof {
				break;
			}
		}
		Ok(toks)
	}

	fn lex_next(&mut self) -> Result<Token, Error> {
		while self.pos < self.src.len() {
			if self.src[self.pos].is_ascii_whitespace() {
				self.pos += 1;
			} else if self.src[self.pos] == b'#' {
				while self.pos < self.src.len() && self.src[self.pos] != b'\n' {
					self.pos += 1;
				}
			} else {
				break;
			}
		}

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
		let span = &self.src[pos..self.pos];
		let sym = self.syms.intern(span);
		let tag = match sym {
			Sym::TYPE => TokenTag::Type,
			Sym::DEF => TokenTag::Def,
			Sym::EACH => TokenTag::Each,
			Sym::DO => TokenTag::Do,
			Sym::IN => TokenTag::In,
			Sym::WHEN => TokenTag::When,
			Sym::THEN => TokenTag::Then,
			Sym::ELSE => TokenTag::Else,
			Sym::END => TokenTag::End,
			Sym::RETURN => TokenTag::Return,
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DeclId(u32);

#[derive(Debug)]
pub enum Decl {
	Type(Type),
	Def(Def),
	Expr(ExprId),
}

#[derive(Debug)]
pub struct Type(pub Sym, pub Vec<Sym>, pub Vec<DeclId>);

#[derive(Debug)]
pub struct Def(pub Sym, pub Vec<Sym>, pub BlockId);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ExprId(u32);

#[derive(Debug)]
pub enum Expr {
	Each(Each),
	When(When),
	Return(Return),
	Call(Call),
	Access(Access),
	Script(Script),
	Assign(Assign),
	Binary(Binary),
	Ident(Ident),
	Builtin(Builtin),
	Lit(Lit),
}

#[derive(Debug)]
pub struct Each(pub Sym, pub ExprId, pub BlockId);

#[derive(Debug)]
pub struct When(pub ExprId, pub BlockId, pub Option<BlockId>);

#[derive(Debug)]
pub struct Return(pub ExprId);

#[derive(Debug)]
pub struct Call(pub ExprId, pub Vec<ExprId>);

#[derive(Debug)]
pub struct Access(pub ExprId, pub Sym);

#[derive(Debug)]
pub struct Script(pub ExprId, pub ExprId);

#[derive(Debug)]
pub struct Assign(pub Place, pub ExprId);

#[derive(Debug, Clone)]
pub enum Place {
	Ident(Ident),
	Member(Member),
}

#[derive(Debug, Clone)]
pub struct Member(pub ExprId, pub Sym);

#[derive(Debug, Clone)]
pub struct Ident(pub Sym);

#[derive(Debug, Clone)]
pub enum Builtin {
	Print(ExprId),
}

#[derive(Debug)]
pub struct Binary(pub BinaryOp, pub ExprId, pub ExprId);

#[derive(Debug)]
pub enum BinaryOp {
	Add,
	Sub,
	Mul,
	Div,
}

#[derive(Debug)]
pub enum Lit {
	Str(String),
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
	pub top: Vec<DeclId>,
	decls: Vec<Decl>,
	decl_toks: Vec<Token>,
	exprs: Vec<Expr>,
	expr_toks: Vec<Token>,
	blocks: Vec<Block>,
}

impl Chunk {
	pub fn new(src: SourceId) -> Self {
		Self {
			src,
			top: Vec::new(),
			decls: Vec::new(),
			decl_toks: Vec::new(),
			exprs: Vec::new(),
			expr_toks: Vec::new(),
			blocks: Vec::new(),
		}
	}

	pub fn get_decl(&self, decl_id: DeclId) -> &Decl {
		&self.decls[decl_id.0 as usize]
	}

	pub fn get_decl_tok(&self, decl_id: DeclId) -> Token {
		self.decl_toks[decl_id.0 as usize]
	}

	pub fn add_decl(&mut self, tok: Token, decl: Decl) -> DeclId {
		let id = DeclId(self.decls.len() as u32);
		self.decls.push(decl);
		self.decl_toks.push(tok);
		id
	}

	pub fn get_expr(&self, expr_id: ExprId) -> &Expr {
		&self.exprs[expr_id.0 as usize]
	}

	pub fn get_expr_tok(&self, expr_id: ExprId) -> Token {
		self.expr_toks[expr_id.0 as usize]
	}

	pub fn add_expr(&mut self, tok: Token, expr: Expr) -> ExprId {
		let id = ExprId(self.exprs.len() as u32);
		self.exprs.push(expr);
		self.expr_toks.push(tok);
		id
	}

	pub fn get_block(&self, block_id: BlockId) -> &Block {
		&self.blocks[block_id.0 as usize]
	}

	pub fn add_block(&mut self, block: Block) -> BlockId {
		let id = BlockId(self.blocks.len() as u32);
		self.blocks.push(block);
		id
	}

	pub fn add_to_top(&mut self, decl_id: DeclId) {
		self.top.push(decl_id);
	}
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
struct Precedence(u8);

impl Precedence {
	const NONE: Precedence = Precedence(0);
	const ASSIGN: Precedence = Precedence(1);
	const ADD: Precedence = Precedence(2);
	const SUB: Precedence = Precedence(2);
	const MUL: Precedence = Precedence(3);
	const DIV: Precedence = Precedence(3);
	const CALL: Precedence = Precedence(4);
	const ACCESS: Precedence = Precedence(4);
	const SCRIPT: Precedence = Precedence(4);

	fn of(tag: TokenTag) -> Self {
		match tag {
			TokenTag::Eq => Precedence::ASSIGN,
			TokenTag::Plus => Precedence::ADD,
			TokenTag::Minus => Precedence::SUB,
			TokenTag::Star => Precedence::MUL,
			TokenTag::Slash => Precedence::DIV,
			TokenTag::LParen => Precedence::CALL,
			TokenTag::Dot => Precedence::ACCESS,
			TokenTag::LBrack => Precedence::SCRIPT,
			_ => Precedence::NONE,
		}
	}
}

pub struct Parser<'src> {
	src: &'src Source,
	toks: Vec<Token>,
	pos: usize,
	chunk: Chunk,
}

impl<'src> Parser<'src> {
	pub fn new(src: &'src Source, toks: Vec<Token>) -> Self {
		Self {
			src,
			toks,
			pos: 0,
			chunk: Chunk::new(src.id),
		}
	}

	pub fn parse(mut self) -> Result<Chunk, Error> {
		while self.pos < self.toks.len() && self.toks[self.pos].tag != TokenTag::Eof {
			let decl_id = self.parse_decl()?;
			self.chunk.add_to_top(decl_id);
		}
		Ok(self.chunk)
	}

	fn parse_decl(&mut self) -> Result<DeclId, Error> {
		match self.toks[self.pos].tag {
			TokenTag::Type => self.parse_type_decl(),
			TokenTag::Def => self.parse_def_decl(),
			_ => {
				let expr_id = self.parse_expr()?;
				let tok = self.chunk.get_expr_tok(expr_id);
				let decl = Decl::Expr(expr_id);
				let decl_id = self.chunk.add_decl(tok, decl);
				Ok(decl_id)
			}
		}
	}

	fn parse_type_decl(&mut self) -> Result<DeclId, Error> {
		let tok = self.take(TokenTag::Type)?;
		let name = self.take(TokenTag::Ident)?.sym.unwrap();
		let mut fields = Vec::new();
		if self.toks[self.pos].tag == TokenTag::LParen {
			self.take(TokenTag::LParen)?;
			while self.pos < self.toks.len() && self.toks[self.pos].tag != TokenTag::RParen {
				let field = self.take(TokenTag::Ident)?.sym.unwrap();
				fields.push(field);
				match self.toks[self.pos].tag {
					TokenTag::Comma => {
						self.pos += 1;
					}
					TokenTag::RParen => {}
					_ => {
						let tok = &self.toks[self.pos];
						return Err(Error::UnexpectedToken(self.src.loc(tok.pos), *tok));
					}
				}
			}
			self.take(TokenTag::RParen)?;
		}
		let mut items = Vec::new();
		while self.pos < self.toks.len() && self.toks[self.pos].tag != TokenTag::End {
			let item_id = self.parse_decl()?;
			items.push(item_id);
		}
		self.take(TokenTag::End)?;
		let decl = Decl::Type(Type(name, fields, items));
		let decl_id = self.chunk.add_decl(tok, decl);
		Ok(decl_id)
	}

	fn parse_def_decl(&mut self) -> Result<DeclId, Error> {
		let tok = self.take(TokenTag::Def)?;
		let name = self.take(TokenTag::Ident)?.sym.unwrap();
		let mut params = Vec::new();
		self.take(TokenTag::LParen)?;
		while self.pos < self.toks.len() && self.toks[self.pos].tag != TokenTag::RParen {
			let param = self.take(TokenTag::Ident)?.sym.unwrap();
			params.push(param);
			match self.toks[self.pos].tag {
				TokenTag::Comma => {
					self.pos += 1;
				}
				TokenTag::RParen => {}
				_ => {
					let tok = &self.toks[self.pos];
					return Err(Error::UnexpectedToken(self.src.loc(tok.pos), *tok));
				}
			}
		}
		self.take(TokenTag::RParen)?;
		let mut body = Vec::new();
		while self.pos < self.toks.len() && self.toks[self.pos].tag != TokenTag::End {
			let expr_id = self.parse_expr()?;
			body.push(expr_id);
		}
		self.take(TokenTag::End)?;
		let block = self.chunk.add_block(Block(body));
		let decl = Decl::Def(Def(name, params, block));
		let decl_id = self.chunk.add_decl(tok, decl);
		Ok(decl_id)
	}

	fn parse_expr(&mut self) -> Result<ExprId, Error> {
		self.parse_expr_prec(Precedence::NONE)
	}

	fn parse_expr_prec(&mut self, min_prec: Precedence) -> Result<ExprId, Error> {
		let mut expr_id = self.parse_expr_unit()?;
		while Precedence::of(self.toks[self.pos].tag) > min_prec {
			expr_id = match self.toks[self.pos].tag {
				TokenTag::Eq => self.parse_assign_expr(expr_id)?,
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
				TokenTag::Dot => self.parse_access_expr(expr_id)?,
				TokenTag::LBrack => self.parse_script_expr(expr_id)?,
				_ => break,
			};
		}
		Ok(expr_id)
	}

	fn parse_access_expr(&mut self, val_id: ExprId) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Dot)?;
		let field = self.take(TokenTag::Ident)?.sym.unwrap();
		let expr = Expr::Access(Access(val_id, field));
		let expr_id = self.chunk.add_expr(tok, expr);
		Ok(expr_id)
	}

	fn parse_script_expr(&mut self, val_id: ExprId) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::LBrack)?;
		let key_id = self.parse_expr()?;
		self.take(TokenTag::RBrack)?;
		let expr = Expr::Script(Script(val_id, key_id));
		let expr_id = self.chunk.add_expr(tok, expr);
		Ok(expr_id)
	}

	fn parse_call_expr(&mut self, val_id: ExprId) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::LParen)?;
		let mut args = Vec::new();
		while self.pos < self.toks.len() && self.toks[self.pos].tag != TokenTag::RParen {
			let arg = self.parse_expr()?;
			args.push(arg);
			match self.toks[self.pos].tag {
				TokenTag::Comma => {
					self.pos += 1;
				}
				TokenTag::RParen => {}
				_ => {
					let tok = self.toks[self.pos];
					return Err(Error::UnexpectedToken(self.src.loc(tok.pos), tok));
				}
			}
		}
		self.take(TokenTag::RParen)?;
		let expr = Expr::Call(Call(val_id, args));
		let expr_id = self.chunk.add_expr(tok, expr);
		Ok(expr_id)
	}

	fn parse_assign_expr(&mut self, val_id: ExprId) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Eq)?;
		let place = match self.chunk.get_expr(val_id) {
			Expr::Ident(ident) => Place::Ident(ident.clone()),
			Expr::Access(Access(target_id, name)) => Place::Member(Member(*target_id, *name)),
			_ => return Err(Error::UnexpectedToken(self.src.loc(tok.pos), tok)),
		};
		let val_expr_id = self.parse_expr_prec(Precedence::NONE)?;
		let expr = Expr::Assign(Assign(place, val_expr_id));
		let expr_id = self.chunk.add_expr(tok, expr);
		Ok(expr_id)
	}

	fn parse_binary_expr(
		&mut self,
		val_id: ExprId,
		op: BinaryOp,
		prec: Precedence,
	) -> Result<ExprId, Error> {
		let tok = self.toks[self.pos];
		self.pos += 1;
		let rhs = self.parse_expr_prec(prec)?;
		let expr = Expr::Binary(Binary(op, val_id, rhs));
		let expr_id = self.chunk.add_expr(tok, expr);
		Ok(expr_id)
	}

	fn parse_expr_unit(&mut self) -> Result<ExprId, Error> {
		match self.toks[self.pos].tag {
			TokenTag::When => self.parse_when_expr(),
			TokenTag::Each => self.parse_each_expr(),
			TokenTag::Return => self.parse_return_expr(),
			TokenTag::Ident => self.parse_ident_expr(),
			TokenTag::Builtin => self.parse_builtin_expr(),
			TokenTag::Str => self.parse_str_lit_expr(),
			TokenTag::Num => self.parse_num_lit_expr(),
			TokenTag::Bool => self.parse_bool_lit_expr(),
			TokenTag::LBrack => self.parse_list_lit_expr(),
			TokenTag::LBrace => self.parse_dict_lit_expr(),
			TokenTag::Nil => self.parse_nil_lit_expr(),
			_ => {
				let tok = &self.toks[self.pos];
				Err(Error::UnexpectedToken(self.src.loc(tok.pos), *tok))
			}
		}
	}

	fn parse_each_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Each)?;
		let item = self.take(TokenTag::Ident)?.sym.unwrap();
		self.take(TokenTag::In)?;
		let iter = self.parse_expr()?;
		self.take(TokenTag::Do)?;
		let mut body = Vec::new();
		while self.pos < self.toks.len() && self.toks[self.pos].tag != TokenTag::End {
			let expr_id = self.parse_expr()?;
			body.push(expr_id);
		}
		self.take(TokenTag::End)?;
		let block = self.chunk.add_block(Block(body));
		let expr = Expr::Each(Each(item, iter, block));
		let expr_id = self.chunk.add_expr(tok, expr);
		Ok(expr_id)
	}

	fn parse_return_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Return)?;
		let val_expr_id = self.parse_expr()?;
		let expr = Expr::Return(Return(val_expr_id));
		let expr_id = self.chunk.add_expr(tok, expr);
		Ok(expr_id)
	}

	fn parse_when_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::When)?;
		let cond = self.parse_expr()?;
		self.take(TokenTag::Then)?;
		let mut then_branch = Vec::new();
		while self.pos < self.toks.len()
			&& self.toks[self.pos].tag != TokenTag::End
			&& self.toks[self.pos].tag != TokenTag::Else
		{
			let expr_id = self.parse_expr()?;
			then_branch.push(expr_id);
		}
		let then_branch = self.chunk.add_block(Block(then_branch));
		let else_branch = if self.toks[self.pos].tag == TokenTag::Else {
			self.take(TokenTag::Else)?;
			let mut else_branch = Vec::new();
			while self.pos < self.toks.len() && self.toks[self.pos].tag != TokenTag::End {
				let expr_id = self.parse_expr()?;
				else_branch.push(expr_id);
			}
			let else_branch = self.chunk.add_block(Block(else_branch));
			Some(else_branch)
		} else {
			None
		};
		self.take(TokenTag::End)?;
		let expr = Expr::When(When(cond, then_branch, else_branch));
		let expr_id = self.chunk.add_expr(tok, expr);
		Ok(expr_id)
	}

	fn parse_ident_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Ident)?;
		let sym = tok.sym.unwrap();
		let expr = Expr::Ident(Ident(sym));
		let expr_id = self.chunk.add_expr(tok, expr);
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
				let expr_id = self.chunk.add_expr(tok, expr);
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
		let mut pos = tok.pos + 1;
		let mut str = String::with_capacity(tok.end - tok.pos);
		let mut chunk_pos = pos;
		while pos < tok.end - 1 {
			if self.src[pos] == b'\\' {
				str.push_str(&self.src[chunk_pos..pos]);
				match self.src[pos + 1] {
					b'n' => str.push('\n'),
					b't' => str.push('\t'),
					b'"' => str.push('"'),
					b'\\' => str.push('\\'),
					_ => {
						let char = self.src[pos..self.src.len()].chars().next().unwrap();
						return Err(Error::UnsupportedStrEsc(self.src.loc(pos), char));
					}
				}
				pos += 2;
				chunk_pos = pos;
			} else {
				pos += 1;
			}
		}
		str.push_str(&self.src[chunk_pos..pos]);
		let expr = Expr::Lit(Lit::Str(str));
		let expr_id = self.chunk.add_expr(tok, expr);
		Ok(expr_id)
	}

	fn parse_num_lit_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Num)?;
		let span = &self.src[tok.pos..tok.end];
		let num = span.parse().unwrap();
		let expr = Expr::Lit(Lit::Num(num));
		let expr_id = self.chunk.add_expr(tok, expr);
		Ok(expr_id)
	}

	fn parse_bool_lit_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Bool)?;
		let span = &self.src[tok.pos..tok.end];
		let bool = span.parse().unwrap();
		let expr = Expr::Lit(Lit::Bool(bool));
		let expr_id = self.chunk.add_expr(tok, expr);
		Ok(expr_id)
	}

	fn parse_list_lit_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::LBrack)?;
		let mut items = Vec::new();
		while self.pos < self.toks.len() && self.toks[self.pos].tag != TokenTag::RBrack {
			let item = self.parse_expr()?;
			items.push(item);
			match self.toks[self.pos].tag {
				TokenTag::Comma => {
					self.pos += 1;
				}
				TokenTag::RBrack => {}
				_ => {
					let tok = self.toks[self.pos];
					return Err(Error::UnexpectedToken(self.src.loc(tok.pos), tok));
				}
			}
		}
		self.take(TokenTag::RBrack)?;
		let expr = Expr::Lit(Lit::List(items));
		let expr_id = self.chunk.add_expr(tok, expr);
		Ok(expr_id)
	}

	fn parse_dict_lit_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::LBrace)?;
		let mut pairs = Vec::new();
		while self.pos < self.toks.len() && self.toks[self.pos].tag != TokenTag::RBrace {
			let key = self.parse_expr()?;
			self.take(TokenTag::Colon)?;
			let val = self.parse_expr()?;
			pairs.push((key, val));
			match self.toks[self.pos].tag {
				TokenTag::Comma => {
					self.pos += 1;
				}
				TokenTag::RBrace => {}
				_ => {
					let tok = self.toks[self.pos];
					return Err(Error::UnexpectedToken(self.src.loc(tok.pos), tok));
				}
			}
		}
		self.take(TokenTag::RBrace)?;
		let expr = Expr::Lit(Lit::Dict(pairs));
		let expr_id = self.chunk.add_expr(tok, expr);
		Ok(expr_id)
	}

	fn parse_nil_lit_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Nil)?;
		let expr = Expr::Lit(Lit::Nil);
		let expr_id = self.chunk.add_expr(tok, expr);
		Ok(expr_id)
	}

	fn take(&mut self, tag: TokenTag) -> Result<Token, Error> {
		let tok = &self.toks[self.pos];
		if tok.tag == tag {
			self.pos += 1;
			Ok(*tok)
		} else {
			Err(Error::UnexpectedToken(self.src.loc(tok.pos), *tok))
		}
	}
}
