use crate::intern::{Interner, Sym};
use crate::src::{Source, SourceId, Span};
use crate::syn::Error;

#[derive(Debug, Clone, Copy)]
pub struct Token {
	pub src: SourceId,
	pub tag: TokenTag,
	pub sym: Option<Sym>,
	pub pos: usize,
	pub end: usize,
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

	Module,
	Import,
	Export,
	Type,
	Extern,
	Case,
	Proto,
	Impl,
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
	pub fn name(&self) -> &'static str {
		match self {
			TokenTag::Eof => "EOF",

			TokenTag::Module => "MODULE",
			TokenTag::Import => "IMPORT",
			TokenTag::Export => "EXPORT",
			TokenTag::Type => "TYPE",
			TokenTag::Extern => "EXTERN",
			TokenTag::Case => "CASE",
			TokenTag::Proto => "PROTO",
			TokenTag::Impl => "IMPL",
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
pub struct TokenId(pub u32);

impl TokenId {
	pub fn index(self) -> usize {
		self.0 as usize
	}

	pub fn next(self) -> Self {
		Self(self.0 + 1)
	}

	pub fn prev(self) -> Self {
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

	pub fn len(&self) -> usize {
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

	pub fn tag(&self, id: TokenId) -> TokenTag {
		self.tags[id.index()]
	}

	pub fn sym(&self, id: TokenId) -> Option<Sym> {
		self.syms[id.index()]
	}

	pub fn start(&self, id: TokenId) -> usize {
		self.starts[id.index()]
	}

	pub fn end(&self, id: TokenId) -> usize {
		self.ends[id.index()]
	}

	pub fn nl_before(&self, id: TokenId) -> bool {
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
				src: self.src.id(),
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
						src: self.src.id(),
						tag: TokenTag::Eq,
						sym: None,
						pos: self.pos - 2,
						end: self.pos,
					})
				} else {
					self.pos += 1;
					Ok(Token {
						src: self.src.id(),
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
						src: self.src.id(),
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
						src: self.src.id(),
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
						src: self.src.id(),
						tag: TokenTag::LtEq,
						sym: None,
						pos: self.pos - 2,
						end: self.pos,
					})
				} else if self.pos + 1 < self.src.len() && self.src[self.pos + 1] == b'<' {
					self.pos += 2;
					Ok(Token {
						src: self.src.id(),
						tag: TokenTag::LtLt,
						sym: None,
						pos: self.pos - 2,
						end: self.pos,
					})
				} else {
					self.pos += 1;
					Ok(Token {
						src: self.src.id(),
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
						src: self.src.id(),
						tag: TokenTag::GtEq,
						sym: None,
						pos: self.pos - 2,
						end: self.pos,
					})
				} else {
					self.pos += 1;
					Ok(Token {
						src: self.src.id(),
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
					src: self.src.id(),
					tag: TokenTag::Plus,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'-' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id(),
					tag: TokenTag::Minus,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'*' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id(),
					tag: TokenTag::Star,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'/' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id(),
					tag: TokenTag::Slash,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'&' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id(),
					tag: TokenTag::Amp,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'{' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id(),
					tag: TokenTag::LBrace,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'}' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id(),
					tag: TokenTag::RBrace,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'[' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id(),
					tag: TokenTag::LBrack,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b']' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id(),
					tag: TokenTag::RBrack,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'(' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id(),
					tag: TokenTag::LParen,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b')' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id(),
					tag: TokenTag::RParen,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b',' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id(),
					tag: TokenTag::Comma,
					sym: None,
					pos: self.pos - 1,
					end: self.pos,
				})
			}
			b'.' => {
				self.pos += 1;
				Ok(Token {
					src: self.src.id(),
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
			Sym::MODULE => TokenTag::Module,
			Sym::IMPORT => TokenTag::Import,
			Sym::EXPORT => TokenTag::Export,
			Sym::TYPE => TokenTag::Type,
			Sym::EXTERN => TokenTag::Extern,
			Sym::CASE => TokenTag::Case,
			Sym::PROTO => TokenTag::Proto,
			Sym::IMPL => TokenTag::Impl,
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
			src: self.src.id(),
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
			src: self.src.id(),
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
			src: self.src.id(),
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
			src: self.src.id(),
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
			src: self.src.id(),
			tag: TokenTag::Num,
			sym: None,
			pos,
			end: self.pos,
		})
	}
}
