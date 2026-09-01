use crate::intern::{Interner, Sym};
use crate::src::{Source, SourceId, Span};
use crate::syn::Error;

#[derive(Debug, Clone, Copy)]
pub struct Token {
	pub tag: TokenTag,
	pub sym: Option<Sym>,
	pub start: usize,
	pub end: usize,
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
	Rescue,
	End,
	Return,
	Break,
	Raise,

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
			TokenTag::Rescue => "RESCUE",
			TokenTag::End => "END",
			TokenTag::Return => "RETURN",
			TokenTag::Break => "BREAK",
			TokenTag::Raise => "RAISE",
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

#[derive(Debug, Clone, Copy)]
pub struct TokenRange {
	pub start: TokenId,
	pub end: TokenId,
}

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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Trivia {
	Comment(usize, usize),
	Newlines(u32),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TriviaId(u32);

impl TriviaId {
	fn from_index(index: usize) -> Self {
		Self(index as u32)
	}

	fn index(self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug)]
pub struct Trivias {
	trivias: Vec<Trivia>,
}

impl Trivias {
	fn new() -> Self {
		Self {
			trivias: Vec::new(),
		}
	}

	fn add(&mut self, trivia: Trivia) {
		self.trivias.push(trivia);
	}

	pub fn ids(&self) -> impl ExactSizeIterator<Item = TriviaId> {
		(0..self.trivias.len()).map(TriviaId::from_index)
	}

	pub fn get(&self, id: TriviaId) -> Trivia {
		self.trivias[id.index()]
	}

	pub fn end(&self) -> TriviaId {
		TriviaId::from_index(self.trivias.len())
	}

	pub fn range(&self, start: TriviaId, end: TriviaId) -> &[Trivia] {
		&self.trivias[start.index()..end.index()]
	}
}

#[derive(Debug)]
pub struct Tokens {
	tags: Vec<TokenTag>,
	syms: Vec<Option<Sym>>,
	starts: Vec<usize>,
	ends: Vec<usize>,
	trivia_starts: Vec<TriviaId>,
}

impl Tokens {
	fn new() -> Self {
		Self {
			tags: Vec::new(),
			syms: Vec::new(),
			starts: Vec::new(),
			ends: Vec::new(),
			trivia_starts: Vec::new(),
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
		trivia_start: TriviaId,
	) -> TokenId {
		let id = TokenId(self.tags.len() as u32);
		self.tags.push(tag);
		self.syms.push(sym);
		self.starts.push(start);
		self.ends.push(end);
		self.trivia_starts.push(trivia_start);
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

	pub fn trivia_start(&self, id: TokenId) -> TriviaId {
		self.trivia_starts[id.index()]
	}

	pub fn span(&self, source: SourceId, range: TokenRange) -> Span {
		Span {
			source,
			start: self.start(range.start),
			end: self.end(range.end),
		}
	}
}

pub struct Lexer<'syms, 'source> {
	syms: &'syms mut Interner,
	source: &'source Source,
	pos: usize,
	toks: Tokens,
	trivias: Trivias,
}

impl<'syms, 'source> Lexer<'syms, 'source> {
	pub fn new(syms: &'syms mut Interner, source: &'source Source) -> Self {
		Self {
			syms,
			source,
			pos: 0,
			toks: Tokens::new(),
			trivias: Trivias::new(),
		}
	}

	pub fn lex(mut self) -> Result<(Tokens, Trivias), Error> {
		loop {
			let trivia_start = self.trivias.end();
			self.skip_trivia();
			let tok = self.lex_next()?;
			self.toks
				.push(tok.tag, tok.sym, tok.start, tok.end, trivia_start);
			if tok.tag == TokenTag::Eof {
				break;
			}
		}
		Ok((self.toks, self.trivias))
	}

	fn skip_trivia(&mut self) {
		let mut newlines = 0;
		while self.pos < self.source.len() {
			if self.source[self.pos] == b'\n' {
				newlines += 1;
				self.pos += 1;
			} else if self.source[self.pos].is_ascii_whitespace() {
				self.pos += 1;
			} else if self.source[self.pos] == b'#' {
				if newlines > 0 {
					self.trivias.add(Trivia::Newlines(newlines));
					newlines = 0;
				}
				let start = self.pos;
				while self.pos < self.source.len() && self.source[self.pos] != b'\n' {
					self.pos += 1;
				}
				self.trivias.add(Trivia::Comment(start, self.pos));
			} else {
				break;
			}
		}
		if newlines > 0 {
			self.trivias.add(Trivia::Newlines(newlines));
		}
	}

	fn lex_next(&mut self) -> Result<Token, Error> {
		if self.pos == self.source.len() {
			return Ok(Token {
				tag: TokenTag::Eof,
				sym: None,
				start: self.pos,
				end: self.pos,
			});
		}

		match self.source[self.pos] {
			b':' => {
				if self.pos + 1 < self.source.len() && self.source[self.pos + 1] == b'=' {
					self.pos += 2;
					Ok(Token {
						tag: TokenTag::Eq,
						sym: None,
						start: self.pos - 2,
						end: self.pos,
					})
				} else {
					self.pos += 1;
					Ok(Token {
						tag: TokenTag::Colon,
						sym: None,
						start: self.pos - 1,
						end: self.pos,
					})
				}
			}
			b'=' => {
				if self.pos + 1 < self.source.len() && self.source[self.pos + 1] == b'=' {
					self.pos += 2;
					Ok(Token {
						tag: TokenTag::EqEq,
						sym: None,
						start: self.pos - 2,
						end: self.pos,
					})
				} else {
					Err(Error::UnexpectedChar(self.source.loc(self.pos), '='))
				}
			}
			b'!' => {
				if self.pos + 1 < self.source.len() && self.source[self.pos + 1] == b'=' {
					self.pos += 2;
					Ok(Token {
						tag: TokenTag::NotEq,
						sym: None,
						start: self.pos - 2,
						end: self.pos,
					})
				} else {
					Err(Error::UnexpectedChar(self.source.loc(self.pos), '!'))
				}
			}
			b'<' => {
				if self.pos + 1 < self.source.len() && self.source[self.pos + 1] == b'=' {
					self.pos += 2;
					Ok(Token {
						tag: TokenTag::LtEq,
						sym: None,
						start: self.pos - 2,
						end: self.pos,
					})
				} else if self.pos + 1 < self.source.len() && self.source[self.pos + 1] == b'<' {
					self.pos += 2;
					Ok(Token {
						tag: TokenTag::LtLt,
						sym: None,
						start: self.pos - 2,
						end: self.pos,
					})
				} else {
					self.pos += 1;
					Ok(Token {
						tag: TokenTag::Lt,
						sym: None,
						start: self.pos - 1,
						end: self.pos,
					})
				}
			}
			b'>' => {
				if self.pos + 1 < self.source.len() && self.source[self.pos + 1] == b'=' {
					self.pos += 2;
					Ok(Token {
						tag: TokenTag::GtEq,
						sym: None,
						start: self.pos - 2,
						end: self.pos,
					})
				} else {
					self.pos += 1;
					Ok(Token {
						tag: TokenTag::Gt,
						sym: None,
						start: self.pos - 1,
						end: self.pos,
					})
				}
			}
			b'+' => {
				self.pos += 1;
				Ok(Token {
					tag: TokenTag::Plus,
					sym: None,
					start: self.pos - 1,
					end: self.pos,
				})
			}
			b'-' => {
				self.pos += 1;
				Ok(Token {
					tag: TokenTag::Minus,
					sym: None,
					start: self.pos - 1,
					end: self.pos,
				})
			}
			b'*' => {
				self.pos += 1;
				Ok(Token {
					tag: TokenTag::Star,
					sym: None,
					start: self.pos - 1,
					end: self.pos,
				})
			}
			b'/' => {
				self.pos += 1;
				Ok(Token {
					tag: TokenTag::Slash,
					sym: None,
					start: self.pos - 1,
					end: self.pos,
				})
			}
			b'&' => {
				self.pos += 1;
				Ok(Token {
					tag: TokenTag::Amp,
					sym: None,
					start: self.pos - 1,
					end: self.pos,
				})
			}
			b'{' => {
				self.pos += 1;
				Ok(Token {
					tag: TokenTag::LBrace,
					sym: None,
					start: self.pos - 1,
					end: self.pos,
				})
			}
			b'}' => {
				self.pos += 1;
				Ok(Token {
					tag: TokenTag::RBrace,
					sym: None,
					start: self.pos - 1,
					end: self.pos,
				})
			}
			b'[' => {
				self.pos += 1;
				Ok(Token {
					tag: TokenTag::LBrack,
					sym: None,
					start: self.pos - 1,
					end: self.pos,
				})
			}
			b']' => {
				self.pos += 1;
				Ok(Token {
					tag: TokenTag::RBrack,
					sym: None,
					start: self.pos - 1,
					end: self.pos,
				})
			}
			b'(' => {
				self.pos += 1;
				Ok(Token {
					tag: TokenTag::LParen,
					sym: None,
					start: self.pos - 1,
					end: self.pos,
				})
			}
			b')' => {
				self.pos += 1;
				Ok(Token {
					tag: TokenTag::RParen,
					sym: None,
					start: self.pos - 1,
					end: self.pos,
				})
			}
			b',' => {
				self.pos += 1;
				Ok(Token {
					tag: TokenTag::Comma,
					sym: None,
					start: self.pos - 1,
					end: self.pos,
				})
			}
			b'.' => {
				self.pos += 1;
				Ok(Token {
					tag: TokenTag::Dot,
					sym: None,
					start: self.pos - 1,
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
					let char = self.source[self.pos..self.source.len()]
						.chars()
						.next()
						.unwrap();
					Err(Error::UnexpectedChar(self.source.loc(self.pos), char))
				}
			}
		}
	}

	fn lex_ident(&mut self) -> Result<Token, Error> {
		let start = self.pos;
		while self.pos < self.source.len()
			&& (self.source[self.pos].is_ascii_alphabetic()
				|| self.source[self.pos].is_ascii_digit()
				|| self.source[self.pos] == b'_')
		{
			self.pos += 1;
		}
		if self.pos < self.source.len() {
			if self.source[self.pos] == b'!'
				&& !(self.pos + 1 < self.source.len()
					&& self.source[self.pos + 1] == b'='
					&& !(self.pos + 2 < self.source.len() && self.source[self.pos + 2] == b'='))
			{
				self.pos += 1;
			} else if self.source[self.pos] == b'?' {
				self.pos += 1;
			}
		}
		let span = &self.source[start..self.pos];
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
			Sym::RESCUE => TokenTag::Rescue,
			Sym::END => TokenTag::End,
			Sym::RETURN => TokenTag::Return,
			Sym::BREAK => TokenTag::Break,
			Sym::RAISE => TokenTag::Raise,
			Sym::AND => TokenTag::And,
			Sym::OR => TokenTag::Or,
			Sym::NOT => TokenTag::Not,
			Sym::SELF => TokenTag::Self_,
			Sym::TRUE | Sym::FALSE => TokenTag::Bool,
			Sym::NIL => TokenTag::Nil,
			_ => TokenTag::Ident,
		};
		Ok(Token {
			tag,
			sym: Some(sym),
			start,
			end: self.pos,
		})
	}

	fn lex_builtin(&mut self) -> Result<Token, Error> {
		let start = self.pos;
		self.pos += 1;
		let ident = self.lex_ident()?;
		Ok(Token {
			tag: TokenTag::Builtin,
			sym: ident.sym,
			start,
			end: ident.end,
		})
	}

	fn lex_str(&mut self) -> Result<Token, Error> {
		let start = self.pos;
		self.pos += 1;
		while self.pos < self.source.len() && self.source[self.pos] != b'"' {
			if self.source[self.pos] == b'\\' && self.pos + 1 < self.source.len() {
				self.pos += 2;
			} else {
				self.pos += 1;
			}
		}
		if self.pos == self.source.len() {
			return Err(Error::UnterminatedStrLit(self.source.loc(start)));
		}
		self.pos += 1;
		Ok(Token {
			tag: TokenTag::Str,
			sym: None,
			start,
			end: self.pos,
		})
	}

	fn lex_char(&mut self) -> Result<Token, Error> {
		let start = self.pos;
		self.pos += 1;
		while self.pos < self.source.len() && self.source[self.pos] != b'\'' {
			if self.source[self.pos] == b'\\' && self.pos + 1 < self.source.len() {
				self.pos += 2;
			} else {
				self.pos += 1;
			}
		}
		if self.pos == self.source.len() {
			return Err(Error::UnterminatedCharLit(self.source.loc(start)));
		}
		self.pos += 1;
		Ok(Token {
			tag: TokenTag::Char,
			sym: None,
			start,
			end: self.pos,
		})
	}

	fn lex_num(&mut self) -> Result<Token, Error> {
		let start = self.pos;
		while self.pos < self.source.len() && self.source[self.pos].is_ascii_digit() {
			self.pos += 1;
		}
		if self.pos < self.source.len() && self.source[self.pos] == b'.' {
			self.pos += 1;
			while self.pos < self.source.len() && self.source[self.pos].is_ascii_digit() {
				self.pos += 1;
			}
		}
		Ok(Token {
			tag: TokenTag::Num,
			sym: None,
			start,
			end: self.pos,
		})
	}
}
