#![allow(unused)]

use std::ops::{Index, Range};

#[derive(Debug)]
struct Package {
	srcs: Vec<Source>,
}

impl Package {
	fn new() -> Self {
		Self { srcs: Vec::new() }
	}

	fn get_src(&self, id: SourceId) -> &Source {
		&self.srcs[id.0]
	}

	fn add_src(&mut self, file: String, chars: Vec<char>) -> SourceId {
		let id = SourceId(self.srcs.len());
		let src = Source { id, file, chars };
		self.srcs.push(src);
		id
	}
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct SourceId(usize);

#[derive(Debug)]
struct Source {
	id: SourceId,
	file: String,
	chars: Vec<char>,
}

impl Source {
	fn len(&self) -> usize {
		self.chars.len()
	}

	fn loc(&self, idx: usize) -> Location {
		let mut i = 0;
		let mut lin = 1;
		let mut col = 1;
		while i < idx && i < self.chars.len() {
			if self.chars[i] == '\n' {
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
	type Output = char;

	fn index(&self, idx: usize) -> &Self::Output {
		&self.chars[idx]
	}
}

impl Index<Range<usize>> for Source {
	type Output = [char];

	fn index(&self, idx: Range<usize>) -> &Self::Output {
		&self.chars[idx.start..idx.end]
	}
}

use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
struct Location {
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
struct Token {
	src: SourceId,
	tag: TokenTag,
	idx: usize,
	len: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum TokenTag {
	Eof,

	Def,
	Each,
	Do,
	In,
	End,
	Return,
	Print,

	Ident,
	Str,
	Num,
	Bool,
	Nil,

	Eq,

	LBrace,
	RBrace,
	LParen,
	RParen,

	Comma,
}

impl TokenTag {
	fn name(&self) -> &'static str {
		match self {
			TokenTag::Eof => "EOF",

			TokenTag::Def => "DEF",
			TokenTag::Each => "EACH",
			TokenTag::Do => "DO",
			TokenTag::In => "IN",
			TokenTag::End => "END",
			TokenTag::Return => "RETURN",
			TokenTag::Print => "PRINT",

			TokenTag::Ident => "IDENT",
			TokenTag::Str => "STR",
			TokenTag::Num => "NUM",
			TokenTag::Bool => "BOOL",
			TokenTag::Nil => "NIL",

			TokenTag::Eq => "EQ",

			TokenTag::LBrace => "LBRACE",
			TokenTag::RBrace => "RBRACE",
			TokenTag::LParen => "LPAREN",
			TokenTag::RParen => "RPAREN",

			TokenTag::Comma => "COMMA",
		}
	}
}

struct Lexer<'src> {
	src: &'src Source,
	idx: usize,
}

impl<'src> Lexer<'src> {
	fn new(src: &'src Source) -> Self {
		Self { src, idx: 0 }
	}

	fn lex(mut self) -> Result<Vec<Token>, Error> {
		let mut toks = Vec::new();
		while self.idx < self.src.len() {
			let tok = self.lex_next()?;
			toks.push(tok);
		}
		Ok(toks)
	}

	fn lex_next(&mut self) -> Result<Token, Error> {
		while self.idx < self.src.len() && self.src[self.idx].is_whitespace() {
			self.idx += 1;
		}
		if self.idx == self.src.len() {
			return Ok(Token {
				src: self.src.id,
				tag: TokenTag::Eof,
				idx: self.idx,
				len: 0,
			});
		}
		match self.src[self.idx] {
			':' => {
				if self.idx + 1 < self.src.len() && self.src[self.idx + 1] == '=' {
					self.idx += 2;
					Ok(Token {
						src: self.src.id,
						tag: TokenTag::Eq,
						idx: self.idx - 2,
						len: 2,
					})
				} else {
					let ch = self.src[self.idx];
					Err(Error::Syntax(
						self.src.loc(self.idx),
						SyntaxError::UnexpectedChar(ch),
					))
				}
			}
			'[' => {
				self.idx += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::LBrace,
					idx: self.idx - 1,
					len: 1,
				})
			}
			']' => {
				self.idx += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::RBrace,
					idx: self.idx - 1,
					len: 1,
				})
			}
			'(' => {
				self.idx += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::LParen,
					idx: self.idx - 1,
					len: 1,
				})
			}
			')' => {
				self.idx += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::RParen,
					idx: self.idx - 1,
					len: 1,
				})
			}
			',' => {
				self.idx += 1;
				Ok(Token {
					src: self.src.id,
					tag: TokenTag::Comma,
					idx: self.idx - 1,
					len: 1,
				})
			}
			'"' => self.lex_str(),
			ch => {
				if ch.is_alphabetic() || ch == '_' {
					self.lex_ident()
				} else if ch.is_numeric() {
					self.lex_num()
				} else {
					Err(Error::Syntax(
						self.src.loc(self.idx),
						SyntaxError::UnexpectedChar(ch),
					))
				}
			}
		}
	}

	fn lex_ident(&mut self) -> Result<Token, Error> {
		let idx = self.idx;
		while self.idx < self.src.len()
			&& (self.src[self.idx].is_alphabetic()
				|| self.src[self.idx].is_numeric()
				|| self.src[self.idx] == '_')
		{
			self.idx += 1;
		}
		let span = self.src[idx..self.idx].iter().collect::<String>();
		let tag = match span.as_str() {
			"def" => TokenTag::Def,
			"each" => TokenTag::Each,
			"do" => TokenTag::Do,
			"in" => TokenTag::In,
			"end" => TokenTag::End,
			"return" => TokenTag::Return,
			"print" => TokenTag::Print,
			"true" | "false" => TokenTag::Bool,
			"nil" => TokenTag::Nil,
			_ => TokenTag::Ident,
		};
		Ok(Token {
			src: self.src.id,
			tag,
			idx,
			len: self.idx - idx,
		})
	}

	fn lex_str(&mut self) -> Result<Token, Error> {
		let idx = self.idx;
		self.idx += 1;
		while self.idx < self.src.len() && self.src[self.idx] != '"' {
			if self.src[self.idx] == '\\' && self.idx + 1 < self.src.len() {
				self.idx += 2;
			} else {
				self.idx += 1;
			}
		}
		if self.idx == self.src.len() {
			return Err(Error::Syntax(
				self.src.loc(idx),
				SyntaxError::UnterminatedStrLit,
			));
		}
		self.idx += 1;
		Ok(Token {
			src: self.src.id,
			tag: TokenTag::Str,
			idx,
			len: self.idx - idx,
		})
	}

	fn lex_num(&mut self) -> Result<Token, Error> {
		let idx = self.idx;
		while self.idx < self.src.len() && self.src[self.idx].is_numeric() {
			self.idx += 1;
		}
		if self.idx < self.src.len() && self.src[self.idx] == '.' {
			self.idx += 1;
			while self.idx < self.src.len() && self.src[self.idx].is_numeric() {
				self.idx += 1;
			}
		}
		Ok(Token {
			src: self.src.id,
			tag: TokenTag::Num,
			idx,
			len: self.idx - idx,
		})
	}
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct StmtId(usize);

#[derive(Debug)]
enum Stmt {
	Def(Def),
	Each(Each),
	Return(Return),
	Print(Print),
	Expr(ExprId),
}

#[derive(Debug)]
struct Def(Token, SymId, Vec<SymId>, Vec<StmtId>);

#[derive(Debug)]
struct Each(Token, SymId, ExprId, Vec<StmtId>);

#[derive(Debug)]
enum Place {
	Ident(Ident),
}

#[derive(Debug)]
struct Return(Token, ExprId);

#[derive(Debug)]
struct Print(Token, ExprId);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct ExprId(usize);

#[derive(Debug)]
enum Expr {
	Call(Call),
	Assign(Assign),
	Ident(Ident),
	Lit(Lit),
}

#[derive(Debug)]
struct Call(Token, ExprId, Vec<ExprId>);

#[derive(Debug)]
struct Assign(Token, Place, ExprId);

#[derive(Debug, Clone)]
struct Ident(Token, SymId);

#[derive(Debug)]
enum Lit {
	Str(Token, String),
	Num(Token, f64),
	Bool(Token, bool),
	List(Token, Vec<ExprId>),
	Nil(Token),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct ChunkId(usize);

#[derive(Debug)]
struct Chunk {
	src: SourceId,
	stmts: Vec<Stmt>,
	exprs: Vec<Expr>,
	top: Vec<StmtId>,
}

impl Chunk {
	fn new(src: SourceId) -> Self {
		Self {
			src,
			stmts: Vec::new(),
			exprs: Vec::new(),
			top: Vec::new(),
		}
	}

	fn get_stmt(&self, stmt_id: StmtId) -> &Stmt {
		&self.stmts[stmt_id.0]
	}

	fn add_stmt(&mut self, stmt: Stmt) -> StmtId {
		self.stmts.push(stmt);
		StmtId(self.stmts.len() - 1)
	}

	fn get_expr(&self, expr_id: ExprId) -> &Expr {
		&self.exprs[expr_id.0]
	}

	fn add_expr(&mut self, expr: Expr) -> ExprId {
		self.exprs.push(expr);
		ExprId(self.exprs.len() - 1)
	}

	fn add_to_top(&mut self, stmt_id: StmtId) {
		self.top.push(stmt_id);
	}
}

struct Parser<'syms, 'src> {
	syms: &'syms mut Interner,
	src: &'src Source,
	toks: Vec<Token>,
	idx: usize,
	chunk: Chunk,
}

impl<'syms, 'src> Parser<'syms, 'src> {
	fn new(syms: &'syms mut Interner, src: &'src Source, toks: Vec<Token>) -> Self {
		Self {
			syms,
			src,
			toks,
			idx: 0,
			chunk: Chunk::new(src.id),
		}
	}

	fn parse(mut self) -> Result<Chunk, Error> {
		while self.idx < self.toks.len() && self.toks[self.idx].tag != TokenTag::Eof {
			let stmt_id = self.parse_stmt()?;
			self.chunk.add_to_top(stmt_id);
		}
		Ok(self.chunk)
	}

	fn parse_stmt(&mut self) -> Result<StmtId, Error> {
		let stmt = match self.toks[self.idx].tag {
			TokenTag::Def => self.parse_def_stmt()?,
			TokenTag::Each => self.parse_each_stmt()?,
			TokenTag::Return => self.parse_return_stmt()?,
			TokenTag::Print => self.parse_print_stmt()?,
			_ => {
				let expr_id = self.parse_expr()?;
				Stmt::Expr(expr_id)
			}
		};
		Ok(self.chunk.add_stmt(stmt))
	}

	fn parse_def_stmt(&mut self) -> Result<Stmt, Error> {
		let tok = self.take(TokenTag::Def)?;
		let ident = self.take(TokenTag::Ident)?;
		let span = self.src[ident.idx..ident.idx + ident.len]
			.iter()
			.collect::<String>();
		let name = self.syms.get_or_add(&span);
		let mut params = Vec::new();
		self.take(TokenTag::LParen)?;
		while self.idx < self.toks.len() && self.toks[self.idx].tag != TokenTag::RParen {
			let tok = self.take(TokenTag::Ident)?;
			let span = self.src[tok.idx..tok.idx + tok.len]
				.iter()
				.collect::<String>();
			let param = self.syms.get_or_add(&span);
			params.push(param);
			match self.toks[self.idx].tag {
				TokenTag::Comma => {
					self.idx += 1;
				}
				TokenTag::RParen => {}
				_ => {
					let tok = &self.toks[self.idx];
					return Err(Error::Syntax(
						self.src.loc(tok.idx),
						SyntaxError::UnexpectedToken(*tok),
					));
				}
			}
		}
		self.take(TokenTag::RParen)?;
		let mut body = Vec::new();
		while self.idx < self.toks.len() && self.toks[self.idx].tag != TokenTag::End {
			let stmt_id = self.parse_stmt()?;
			body.push(stmt_id);
		}
		self.take(TokenTag::End)?;
		Ok(Stmt::Def(Def(tok, name, params, body)))
	}

	fn parse_each_stmt(&mut self) -> Result<Stmt, Error> {
		let tok = self.take(TokenTag::Each)?;
		let ident = self.take(TokenTag::Ident)?;
		let span = self.src[ident.idx..ident.idx + ident.len]
			.iter()
			.collect::<String>();
		let item = self.syms.get_or_add(&span);
		self.take(TokenTag::In)?;
		let iter = self.parse_expr()?;
		self.take(TokenTag::Do)?;
		let mut body = Vec::new();
		while self.idx < self.toks.len() && self.toks[self.idx].tag != TokenTag::End {
			let stmt_id = self.parse_stmt()?;
			body.push(stmt_id);
		}
		self.take(TokenTag::End)?;
		Ok(Stmt::Each(Each(tok, item, iter, body)))
	}

	fn parse_return_stmt(&mut self) -> Result<Stmt, Error> {
		let tok = self.take(TokenTag::Return)?;
		let expr_id = self.parse_expr()?;
		Ok(Stmt::Return(Return(tok, expr_id)))
	}

	fn parse_print_stmt(&mut self) -> Result<Stmt, Error> {
		let tok = self.take(TokenTag::Print)?;
		let expr_id = self.parse_expr()?;
		Ok(Stmt::Print(Print(tok, expr_id)))
	}

	fn parse_expr(&mut self) -> Result<ExprId, Error> {
		let expr = self.parse_expr_unit()?;
		let expr_id = self.chunk.add_expr(expr);
		let tok = self.toks[self.idx];
		match tok.tag {
			TokenTag::Eq => {
				self.idx += 1;
				let place = match self.chunk.get_expr(expr_id) {
					Expr::Ident(ident) => Place::Ident(ident.clone()),
					_ => {
						return Err(Error::Syntax(
							self.src.loc(tok.idx),
							SyntaxError::UnexpectedToken(tok),
						));
					}
				};
				let val_expr_id = self.parse_expr()?;
				let expr = Expr::Assign(Assign(tok, place, val_expr_id));
				let expr_id = self.chunk.add_expr(expr);
				Ok(expr_id)
			}
			TokenTag::LParen => {
				let mut args = Vec::new();
				self.take(TokenTag::LParen)?;
				while self.idx < self.toks.len() && self.toks[self.idx].tag != TokenTag::RParen {
					let arg = self.parse_expr()?;
					args.push(arg);
					match self.toks[self.idx].tag {
						TokenTag::Comma => {
							self.idx += 1;
						}
						TokenTag::RParen => {}
						_ => {
							let tok = self.toks[self.idx];
							return Err(Error::Syntax(
								self.src.loc(tok.idx),
								SyntaxError::UnexpectedToken(tok),
							));
						}
					}
				}
				self.take(TokenTag::RParen)?;
				let expr = Expr::Call(Call(tok, expr_id, args));
				let expr_id = self.chunk.add_expr(expr);
				Ok(expr_id)
			}
			_ => Ok(expr_id),
		}
	}

	fn parse_expr_unit(&mut self) -> Result<Expr, Error> {
		match self.toks[self.idx].tag {
			TokenTag::Ident => self.parse_ident_expr(),
			TokenTag::Str => self.parse_str_lit_expr(),
			TokenTag::Num => self.parse_num_lit_expr(),
			TokenTag::Bool => self.parse_bool_lit_expr(),
			TokenTag::LBrace => self.parse_list_lit_expr(),
			TokenTag::Nil => self.parse_nil_lit_expr(),
			_ => {
				let tok = &self.toks[self.idx];
				Err(Error::Syntax(
					self.src.loc(tok.idx),
					SyntaxError::UnexpectedToken(*tok),
				))
			}
		}
	}

	fn parse_ident_expr(&mut self) -> Result<Expr, Error> {
		let tok = self.take(TokenTag::Ident)?;
		let span = self.src[tok.idx..tok.idx + tok.len]
			.iter()
			.collect::<String>();
		let sym_id = self.syms.get_or_add(&span);
		Ok(Expr::Ident(Ident(tok, sym_id)))
	}

	fn parse_str_lit_expr(&mut self) -> Result<Expr, Error> {
		let tok = self.take(TokenTag::Str)?;
		let mut idx = tok.idx + 1;
		let mut str = String::with_capacity(tok.len);
		while idx < self.src.len() && idx < tok.idx + tok.len - 1 {
			if self.src[idx] == '\\' {
				match self.src[idx + 1] {
					'n' => str.push('\n'),
					't' => str.push('\t'),
					'"' => str.push('"'),
					'\\' => str.push('\\'),
					char => {
						return Err(Error::Syntax(
							self.src.loc(idx),
							SyntaxError::UnsupportedStrEsc(char),
						));
					}
				}
				idx += 2;
			} else {
				str.push(self.src[idx]);
				idx += 1;
			}
		}
		Ok(Expr::Lit(Lit::Str(tok, str)))
	}

	fn parse_num_lit_expr(&mut self) -> Result<Expr, Error> {
		let tok = self.take(TokenTag::Num)?;
		let span = self.src[tok.idx..tok.idx + tok.len]
			.iter()
			.collect::<String>();
		let num = span.parse().unwrap();
		Ok(Expr::Lit(Lit::Num(tok, num)))
	}

	fn parse_bool_lit_expr(&mut self) -> Result<Expr, Error> {
		let tok = self.take(TokenTag::Bool)?;
		let span = self.src[tok.idx..tok.idx + tok.len]
			.iter()
			.collect::<String>();
		let bool = span.parse().unwrap();
		Ok(Expr::Lit(Lit::Bool(tok, bool)))
	}

	fn parse_list_lit_expr(&mut self) -> Result<Expr, Error> {
		let tok = self.take(TokenTag::LBrace)?;
		let mut items = Vec::new();
		while self.idx < self.toks.len() && self.toks[self.idx].tag != TokenTag::RBrace {
			let item = self.parse_expr()?;
			items.push(item);
			match self.toks[self.idx].tag {
				TokenTag::Comma => {
					self.idx += 1;
				}
				TokenTag::RBrace => {}
				_ => {
					let tok = self.toks[self.idx];
					return Err(Error::Syntax(
						self.src.loc(tok.idx),
						SyntaxError::UnexpectedToken(tok),
					));
				}
			}
		}
		self.take(TokenTag::RBrace)?;
		Ok(Expr::Lit(Lit::List(tok, items)))
	}

	fn parse_nil_lit_expr(&mut self) -> Result<Expr, Error> {
		let tok = self.take(TokenTag::Nil)?;
		Ok(Expr::Lit(Lit::Nil(tok)))
	}

	fn take(&mut self, tag: TokenTag) -> Result<Token, Error> {
		let tok = &self.toks[self.idx];
		if tok.tag == tag {
			self.idx += 1;
			Ok(*tok)
		} else {
			Err(Error::Syntax(
				self.src.loc(tok.idx),
				SyntaxError::UnexpectedToken(*tok),
			))
		}
	}
}

#[derive(Debug)]
enum Error {
	Syntax(Location, SyntaxError),
	Runtime(Location, RuntimeError),
}

#[derive(Debug)]
enum SyntaxError {
	UnexpectedChar(char),
	UnexpectedToken(Token),
	UnterminatedStrLit,
	UnsupportedStrEsc(char),
}

#[derive(Debug)]
enum RuntimeError {
	WrongArgCount(usize, usize),
	CallNonCallable(String),
	IterNonIterable(String),
	UnboundIdent(String),
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::Syntax(loc, err) => write!(f, "{}: syntax error: {}", loc, err),
			Self::Runtime(loc, err) => write!(f, "{}: runtime error: {}", loc, err),
		}
	}
}

impl Display for SyntaxError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::UnexpectedChar(char) => write!(f, "unexpected char '{}'", char),
			Self::UnexpectedToken(tok) => write!(f, "unexpected token {}", tok.tag.name()),
			Self::UnterminatedStrLit => write!(f, "unterminated string literal"),
			Self::UnsupportedStrEsc(esc) => write!(f, "unsupported escape sequence '\\{}'", esc),
		}
	}
}

impl Display for RuntimeError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::WrongArgCount(have, want) => {
				write!(f, "wrong number of args; have {}, want {}", have, want)
			}
			Self::CallNonCallable(val) => write!(f, "call of non-callable {}", val),
			Self::IterNonIterable(val) => write!(f, "iter of non-iterable {}", val),
			Self::UnboundIdent(ident) => write!(f, "unbound ident '{}'", ident),
		}
	}
}

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct SymId(usize);

#[derive(Debug)]
struct Sym(SymId, String);

struct Interner {
	syms: Vec<Sym>,
	ids: HashMap<String, SymId>,
}

impl Interner {
	fn new() -> Self {
		Self {
			syms: Vec::new(),
			ids: HashMap::new(),
		}
	}

	fn get_or_add(&mut self, name: &str) -> SymId {
		match self.get_by_name(name) {
			Some(Sym(id, _)) => *id,
			None => {
				let id = SymId(self.syms.len());
				let sym = Sym(id, String::from(name));
				self.syms.push(sym);
				self.ids.insert(String::from(name), id);
				id
			}
		}
	}

	fn get_by_name(&self, name: &str) -> Option<&Sym> {
		match self.ids.get(name) {
			Some(id) => Some(&self.syms[id.0]),
			None => None,
		}
	}

	fn get_by_id(&self, id: SymId) -> &Sym {
		&self.syms[id.0]
	}
}

#[derive(Debug, Clone)]
enum Val {
	Num(f64),
	Bool(bool),
	Ref(Ref),
	Nil,
}

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug, Clone)]
struct Ref(Rc<RefCell<Obj>>);

impl Ref {
	fn new(obj: Obj) -> Self {
		Self(Rc::new(RefCell::new(obj)))
	}

	fn get(&self) -> impl Deref<Target = Obj> {
		self.0.borrow()
	}
}

#[derive(Debug)]
enum Obj {
	Str(String),
	List(Vec<Val>),
	Proc(Proc),
}

#[derive(Debug)]
struct Proc {
	name: SymId,
	params: Vec<SymId>,
	body: Vec<StmtId>,
	scope: Rc<RefCell<Scope>>,
}

#[derive(Debug)]
struct Scope {
	vals: HashMap<SymId, Val>,
	outer: Option<Rc<RefCell<Scope>>>,
}

impl Scope {
	fn root() -> Self {
		Self {
			vals: HashMap::new(),
			outer: None,
		}
	}

	fn within(outer: Rc<RefCell<Self>>) -> Self {
		Self {
			vals: HashMap::new(),
			outer: Some(outer),
		}
	}

	fn lookup(&self, name: SymId) -> Option<Val> {
		match self.vals.get(&name) {
			Some(val) => Some(val.clone()),
			None => match &self.outer {
				Some(outer) => outer.borrow().lookup(name),
				None => None,
			},
		}
	}

	fn assign(&mut self, name: SymId, val: Val) {
		self.vals.insert(name, val);
	}
}

struct Interpreter<'syms, 'pkg> {
	syms: &'syms Interner,
	pkg: &'pkg Package,
	scope: Rc<RefCell<Scope>>,
}

impl<'syms, 'pkg> Interpreter<'syms, 'pkg> {
	fn new(syms: &'syms Interner, pkg: &'pkg Package) -> Self {
		Self {
			syms,
			pkg,
			scope: Rc::new(RefCell::new(Scope::root())),
		}
	}

	fn eval(mut self, chunk: Chunk) -> Result<(), Error> {
		for stmt_id in &chunk.top {
			if let Some(val) = self.eval_stmt(&chunk, *stmt_id)? {
				return Ok(());
			}
		}
		Ok(())
	}

	fn eval_stmt(&mut self, chunk: &Chunk, stmt_id: StmtId) -> Result<Option<Val>, Error> {
		match chunk.get_stmt(stmt_id) {
			Stmt::Def(Def(_, name, params, body)) => {
				let obj = Obj::Proc(Proc {
					name: *name,
					params: params.to_vec(),
					body: body.to_vec(),
					scope: self.scope.clone(),
				});
				let val = Val::Ref(Ref::new(obj));
				self.scope.borrow_mut().assign(*name, val);
				Ok(None)
			}
			Stmt::Each(Each(tok, name, iter, body)) => {
				let iter = self.eval_expr(chunk, *iter)?;
				match iter {
					Val::Ref(rf) => match &*rf.get() {
						Obj::List(items) => {
							let outer_scope = self.scope.clone();
							let inner_scope = Scope::within(outer_scope.clone());
							self.scope = Rc::new(RefCell::new(inner_scope));
							for item in items {
								self.scope.borrow_mut().assign(*name, item.clone());
								for stmt_id in body {
									match self.eval_stmt(chunk, *stmt_id) {
										Ok(Some(val)) => {
											self.scope = outer_scope;
											return Ok(Some(val));
										}
										Ok(None) => {},
										Err(err) => {
											self.scope = outer_scope;
											return Err(err)
										}
									}
								}
							}
							self.scope = outer_scope;
							Ok(None)
						}
						obj => {
							let src = self.pkg.get_src(chunk.src);
							let loc = src.loc(tok.idx);
							Err(Error::Runtime(loc, RuntimeError::IterNonIterable(format!("{:?}", obj))))
						}
					}
					val => {
						let src = self.pkg.get_src(chunk.src);
						let loc = src.loc(tok.idx);
						Err(Error::Runtime(loc, RuntimeError::IterNonIterable(format!("{:?}", val))))
					}

				}
			}
			Stmt::Return(Return(_, val_expr_id)) => {
				let val = self.eval_expr(chunk, *val_expr_id)?;
				Ok(Some(val))
			}
			Stmt::Print(Print(_, val_expr_id)) => {
				let val = self.eval_expr(chunk, *val_expr_id)?;
				println!("{}", rt_print(self.syms, &val));
				Ok(None)
			}
			Stmt::Expr(expr_id) => {
				self.eval_expr(chunk, *expr_id)?;
				Ok(None)
			}
		}
	}

	fn eval_expr(&mut self, chunk: &Chunk, expr_id: ExprId) -> Result<Val, Error> {
		match chunk.get_expr(expr_id) {
			Expr::Call(Call(tok, val_id, arg_ids)) => {
				let mut args = Vec::with_capacity(arg_ids.len());
				for arg_id in arg_ids {
					let arg = self.eval_expr(chunk, *arg_id)?;
					args.push(arg);
				}
				match self.eval_expr(chunk, *val_id)? {
					Val::Ref(rf) => match &*rf.get() {
						Obj::Proc(proc) => {
							if args.len() != proc.params.len() {
								let src = self.pkg.get_src(chunk.src);
								let loc = src.loc(tok.idx);
								return Err(Error::Runtime(
									loc,
									RuntimeError::WrongArgCount(args.len(), proc.params.len()),
								));
							}
							let caller_scope = self.scope.clone();
							let mut callee_scope = Scope::within(proc.scope.clone());
							for (arg, param) in args.into_iter().zip(proc.params.iter()) {
								callee_scope.assign(*param, arg);
							}
							self.scope = Rc::new(RefCell::new(callee_scope));
							for stmt_id in &proc.body {
								match self.eval_stmt(chunk, *stmt_id) {
									Ok(Some(val)) => {
										self.scope = caller_scope;
										return Ok(val);
									}
									Ok(None) => {}
									Err(err) => {
										self.scope = caller_scope;
										return Err(err);
									}
								}
							}
							self.scope = caller_scope;
							Ok(Val::Nil)
						}
						obj => {
							let src = self.pkg.get_src(chunk.src);
							let loc = src.loc(tok.idx);
							Err(Error::Runtime(
								loc,
								RuntimeError::CallNonCallable(format!("{:?}", obj)),
							))
						}
					},
					val => {
						let src = self.pkg.get_src(chunk.src);
						let loc = src.loc(tok.idx);
						Err(Error::Runtime(
							loc,
							RuntimeError::CallNonCallable(format!("{:?}", val)),
						))
					}
				}
			}
			Expr::Assign(Assign(_, place, val_expr_id)) => {
				let val = self.eval_expr(chunk, *val_expr_id)?;
				match place {
					Place::Ident(Ident(_, sym_id)) => {
						self.scope.borrow_mut().assign(*sym_id, val.clone());
						Ok(val)
					}
				}
			}
			Expr::Ident(Ident(tok, sym_id)) => match self.scope.borrow().lookup(*sym_id) {
				Some(val) => Ok(val),
				None => {
					let src = self.pkg.get_src(chunk.src);
					let loc = src.loc(tok.idx);
					let name = self.syms.get_by_id(*sym_id);
					Err(Error::Runtime(
						loc,
						RuntimeError::UnboundIdent(name.1.clone()),
					))
				}
			},
			Expr::Lit(lit) => Ok(match lit {
				Lit::Str(_, str) => Val::Ref(Ref::new(Obj::Str(str.clone()))),
				Lit::Num(_, num) => Val::Num(*num),
				Lit::Bool(_, bool) => Val::Bool(*bool),
				Lit::List(_, item_ids) => {
					let mut items = Vec::with_capacity(item_ids.len());
					for item_id in item_ids {
						let item = self.eval_expr(chunk, *item_id)?;
						items.push(item);
					}
					Val::Ref(Ref::new(Obj::List(items)))
				}
				Lit::Nil(_) => Val::Nil,
			}),
		}
	}
}

fn rt_print(syms: &Interner, val: &Val) -> String {
	match val {
		Val::Num(num) => format!("{}", num),
		Val::Bool(bool) => format!("{}", bool),
		Val::Ref(rf) => match &*rf.get() {
			Obj::Str(str) => String::from(str),
			Obj::List(items) => {
				let mut res = String::new();
				res.push('[');
				for (i, item) in items.iter().enumerate() {
					res.push_str(&rt_print(syms, item));
					if i + 1 != items.len() {
						res.push_str(", ");
					}
				}
				res.push(']');
				res
			}
			Obj::Proc(proc) => {
				let mut res = String::new();
				let name = syms.get_by_id(proc.name);
				res.push_str(&format!("def {}(", name.1));
				for (i, param) in proc.params.iter().enumerate() {
					let param = syms.get_by_id(*param);
					res.push_str(&param.1);
					if i + 1 != proc.params.len() {
						res.push_str(", ");
					}
				}
				res.push(')');
				res
			}
		},
		Val::Nil => String::from("nil"),
	}
}

use std::env;
use std::fs;
use std::process;

fn main() {
	let args = env::args().collect::<Vec<_>>();
	if args.len() != 2 {
		eprintln!("usage: hrm <file>");
		process::exit(1);
	}
	let file = args[1].clone();
	let chars = match fs::read_to_string(&file) {
		Ok(str) => str.chars().collect::<Vec<_>>(),
		Err(_) => {
			eprintln!("error: cannot read file '{}'", file);
			process::exit(1);
		}
	};

	let mut syms = Interner::new();
	let mut pkg = Package::new();
	let src_id = pkg.add_src(file, chars);
	let src = pkg.get_src(src_id);

	match Lexer::new(src)
		.lex()
		.and_then(|toks| Parser::new(&mut syms, src, toks).parse())
		.and_then(|chunk| Interpreter::new(&syms, &pkg).eval(chunk))
	{
		Ok(()) => {}
		Err(err) => eprintln!("{}", err),
	}
}
