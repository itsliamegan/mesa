use crate::intern::Sym;
use crate::src::{Source, Span};
use crate::syn::lex::{Token, TokenId, TokenTag, Tokens};
use crate::syn::nodes::*;
use crate::syn::{Chunk, Error};

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
		TokenTag::Self_ => true,
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
		TokenTag::Do => true,
		TokenTag::Return => true,
		TokenTag::Break => true,
		TokenTag::Raise => true,
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
			chunk: Chunk::new(src.id()),
		}
	}

	fn tag(&self) -> TokenTag {
		self.toks.tag(self.cur)
	}

	fn tok(&self, id: TokenId) -> Token {
		Token {
			src: self.src.id(),
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
			TokenTag::Module => self.parse_module_decl(),
			TokenTag::Import => self.parse_import_decl(),
			TokenTag::Export => self.parse_export_decl(),
			TokenTag::Type => self.parse_type_in_module(),
			TokenTag::Extern => self.parse_extern_decl(),
			TokenTag::Proto => self.parse_proto_decl(),
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

	fn parse_module_decl(&mut self) -> Result<ModuleItemId, Error> {
		let tok = self.take(TokenTag::Module)?;
		let path = self.parse_dotted_path()?;
		let item = ModuleItem::Module(Module { path });
		let item_id = self.chunk.add_module_item(tok.into(), item);
		Ok(item_id)
	}

	fn parse_import_decl(&mut self) -> Result<ModuleItemId, Error> {
		let tok = self.take(TokenTag::Import)?;
		let path = self.parse_dotted_path()?;
		let item = ModuleItem::Import(Import { path });
		let item_id = self.chunk.add_module_item(tok.into(), item);
		Ok(item_id)
	}

	fn parse_export_decl(&mut self) -> Result<ModuleItemId, Error> {
		let tok = self.take(TokenTag::Export)?;
		let mut names = Vec::new();
		names.push(self.take(TokenTag::Ident)?.sym.unwrap());
		while self.tag() == TokenTag::Comma {
			self.cur = self.cur.next();
			names.push(self.take(TokenTag::Ident)?.sym.unwrap());
		}
		let item = ModuleItem::Export(Export { names });
		let item_id = self.chunk.add_module_item(tok.into(), item);
		Ok(item_id)
	}

	fn parse_dotted_path(&mut self) -> Result<Vec<Sym>, Error> {
		let mut path = Vec::new();
		path.push(self.take_type_name()?);
		while self.tag() == TokenTag::Dot {
			self.cur = self.cur.next();
			path.push(self.take_type_name()?);
		}
		Ok(path)
	}

	fn parse_type_in_module(&mut self) -> Result<ModuleItemId, Error> {
		let (type_, span) = self.parse_type(TokenTag::Type, true)?;
		let item = ModuleItem::Type(type_);
		let item_id = self.chunk.add_module_item(span, item);
		Ok(item_id)
	}

	fn parse_extern_decl(&mut self) -> Result<ModuleItemId, Error> {
		let tok = self.take(TokenTag::Extern)?;
		self.take(TokenTag::Type)?;
		let name = self.take_type_name()?;
		let impls = self.parse_impl_line()?;
		let mut items = Vec::new();
		while self.cur.index() < self.toks.len() && self.tag() != TokenTag::End {
			items.push(match self.tag() {
				TokenTag::Extern => self.parse_extern_method_decl()?,
				TokenTag::Def => self.parse_method_decl()?,
				_ => return Err(self.unexpected()),
			});
		}
		self.take(TokenTag::End)?;
		let item = ModuleItem::Extern(Extern { name, impls, items });
		let item_id = self.chunk.add_module_item(tok.into(), item);
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

	fn parse_type(&mut self, open: TokenTag, allow_cases: bool) -> Result<(Type, Span), Error> {
		let tok = self.take(open)?;
		let name = self.take_type_name()?;
		let params = self.parse_params()?;
		let impls = self.parse_impl_line()?;
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
				TypeItem::Type(..) | TypeItem::Method(..) | TypeItem::Extern(..) => {
					seen_method = true
				}
			}
			items.push(item_id);
		}
		self.take(TokenTag::End)?;
		Ok((
			Type {
				name,
				params,
				impls,
				items,
			},
			tok.into(),
		))
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
		let item = TypeItem::Field(Field {
			name,
			init: expr_id,
		});
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
		let block = self.parse_body_block()?;
		let def = Def {
			name,
			params,
			body: block,
		};
		let method = if is_static {
			Method::Static(def)
		} else {
			Method::Instance(def)
		};
		let item = TypeItem::Method(method);
		let item_id = self.chunk.add_type_item(tok.into(), item);
		Ok(item_id)
	}

	// A native member declaration. It takes no body block at all rather than
	// taking and discarding an empty one, so that 'ExternDef' has no body field
	// downstream consumers would have to ignore.
	fn parse_extern_method_decl(&mut self) -> Result<TypeItemId, Error> {
		let tok = self.take(TokenTag::Extern)?;
		self.take(TokenTag::Def)?;
		let is_static = self.tag() == TokenTag::Self_;
		let name = if is_static {
			self.take(TokenTag::Self_)?;
			self.take(TokenTag::Dot)?;
			self.take(TokenTag::Ident)?.sym.unwrap()
		} else {
			self.take(TokenTag::Ident)?.sym.unwrap()
		};
		let params = self.parse_params()?;
		self.take(TokenTag::End)?;
		let def = ExternDef { name, params };
		let method = if is_static {
			ExternMethod::Static(def)
		} else {
			ExternMethod::Instance(def)
		};
		let item = TypeItem::Extern(method);
		let item_id = self.chunk.add_type_item(tok.into(), item);
		Ok(item_id)
	}

	fn parse_proto_decl(&mut self) -> Result<ModuleItemId, Error> {
		let tok = self.take(TokenTag::Proto)?;
		let name = self.take_type_name()?;
		let mut items = Vec::new();
		while self.cur.index() < self.toks.len() && self.tag() != TokenTag::End {
			let item_id = self.parse_proto_item()?;
			items.push(item_id);
		}
		self.take(TokenTag::End)?;
		let item = ModuleItem::Proto(Proto { name, items });
		let item_id = self.chunk.add_module_item(tok.into(), item);
		Ok(item_id)
	}

	fn parse_proto_item(&mut self) -> Result<ProtoItemId, Error> {
		let tok = self.take(TokenTag::Def)?;
		let name = self.take(TokenTag::Ident)?.sym.unwrap();
		let params = self.parse_params()?;
		let block = self.parse_body_block()?;
		let item = ProtoItem {
			def: Def {
				name,
				params,
				body: block,
			},
		};
		let item_id = self.chunk.add_proto_item(tok.into(), item);
		Ok(item_id)
	}

	fn parse_def_decl(&mut self) -> Result<ModuleItemId, Error> {
		let tok = self.take(TokenTag::Def)?;
		let name = self.take(TokenTag::Ident)?.sym.unwrap();
		let params = self.parse_params()?;
		let block = self.parse_body_block()?;
		let item = ModuleItem::Def(Def {
			name,
			params,
			body: block,
		});
		let item_id = self.chunk.add_module_item(tok.into(), item);
		Ok(item_id)
	}

	fn take_type_name(&mut self) -> Result<Sym, Error> {
		let ident = self.take(TokenTag::Ident)?;
		let last = self.src[ident.end - 1];
		// Only locals, fields, and procs can have ! and ? in their names.
		if last == b'!' || last == b'?' {
			return Err(Error::UnexpectedChar(
				self.src.loc(ident.end - 1),
				last as char,
			));
		}
		Ok(ident.sym.unwrap())
	}

	fn parse_body_block(&mut self) -> Result<BlockId, Error> {
		let mut body = Vec::new();
		while self.cur.index() < self.toks.len() && self.tag() != TokenTag::End {
			let expr_id = self.parse_expr()?;
			body.push(expr_id);
		}
		self.take(TokenTag::End)?;
		Ok(self.chunk.add_block(Block { exprs: body }))
	}

	fn parse_impl_line(&mut self) -> Result<Vec<Vec<Sym>>, Error> {
		let mut impls = Vec::new();
		if self.tag() != TokenTag::Impl {
			return Ok(impls);
		}
		self.take(TokenTag::Impl)?;
		impls.push(self.parse_dotted_path()?);
		while self.tag() == TokenTag::Comma {
			self.take(TokenTag::Comma)?;
			impls.push(self.parse_dotted_path()?);
		}
		Ok(impls)
	}

	fn parse_params(&mut self) -> Result<Vec<Param>, Error> {
		let mut params = Vec::new();
		if self.tag() != TokenTag::LParen || self.toks.nl_before(self.cur) {
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
			params.push(Param { name, default });
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
		let expr = Expr::Member(Member {
			receiver: val_id,
			name: field,
		});
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_access_expr(&mut self, val_id: ExprId) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::LBrack)?;
		let key_id = self.parse_expr()?;
		self.take(TokenTag::RBrack)?;
		let expr = Expr::Access(Access {
			receiver: val_id,
			key: key_id,
		});
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
			args.push(Arg { name, val: arg });
			match self.tag() {
				TokenTag::Comma => {
					self.cur = self.cur.next();
				}
				TokenTag::RParen => {}
				_ => return Err(self.unexpected()),
			}
		}
		self.take(TokenTag::RParen)?;
		let expr = Expr::Call(Call {
			callee: val_id,
			args,
		});
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
		let expr = Expr::Assign(Assign {
			place,
			val: val_expr_id,
		});
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
		let expr = Expr::Binary(Binary {
			op,
			lhs: val_id,
			rhs,
		});
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
		let expr = Expr::Unary(Unary { op, val: val_id });
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_mention_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Amp)?;
		let val_id = self.parse_expr_prec(Precedence::MENTION)?;
		let expr = Expr::Mention(Mention { val: val_id });
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_expr_unit(&mut self) -> Result<ExprId, Error> {
		match self.tag() {
			TokenTag::LParen => self.parse_group_expr(),
			TokenTag::When => self.parse_when_expr(),
			TokenTag::Each => self.parse_each_expr(),
			TokenTag::Loop => self.parse_loop_expr(),
			TokenTag::Do => self.parse_do_expr(),
			TokenTag::Return => self.parse_return_expr(),
			TokenTag::Break => self.parse_break_expr(),
			TokenTag::Raise => self.parse_raise_expr(),
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
		let block = self.chunk.add_block(Block { exprs: body });
		let expr = Expr::Each(Each {
			item,
			iter,
			body: block,
		});
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
		let block = self.chunk.add_block(Block { exprs: body });
		let expr = Expr::Loop(Loop { body: block });
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
		let expr = Expr::Return(Return { val: val_expr_id });
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
		let expr = Expr::Break(Break { val: val_expr_id });
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_raise_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Raise)?;
		let val = self.parse_expr()?;
		let expr = Expr::Raise(Raise { val });
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_when_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::When)?;
		let cond = self.parse_expr()?;
		if self.tag() == TokenTag::Case {
			return self.parse_match_arms(tok, cond);
		}
		self.take(TokenTag::Then)?;
		let mut then_branch = Vec::new();
		while self.cur.index() < self.toks.len()
			&& self.tag() != TokenTag::End
			&& self.tag() != TokenTag::Else
		{
			let expr_id = self.parse_expr()?;
			then_branch.push(expr_id);
		}
		let then_branch = self.chunk.add_block(Block { exprs: then_branch });
		let (else_branch, consume_end) = if self.tag() == TokenTag::Else {
			self.take(TokenTag::Else)?;
			if self.tag() == TokenTag::When {
				let nested_when = self.parse_when_expr()?;
				let else_branch = self.chunk.add_block(Block {
					exprs: vec![nested_when],
				});
				(Some(else_branch), false)
			} else {
				let mut else_branch = Vec::new();
				while self.cur.index() < self.toks.len() && self.tag() != TokenTag::End {
					let expr_id = self.parse_expr()?;
					else_branch.push(expr_id);
				}
				let else_branch = self.chunk.add_block(Block { exprs: else_branch });
				(Some(else_branch), true)
			}
		} else {
			(None, true)
		};
		if consume_end {
			self.take(TokenTag::End)?;
		}
		let expr = Expr::When(When {
			cond,
			then_branch,
			else_branch,
		});
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_match_arms(&mut self, tok: Token, scrutinee: ExprId) -> Result<ExprId, Error> {
		let (arms, else_branch) = self.parse_case_arms()?;
		let expr = Expr::Match(Match {
			scrutinee,
			arms,
			else_branch,
		});
		let expr_id = self.chunk.add_expr(tok.into(), expr);
		Ok(expr_id)
	}

	fn parse_case_arms(&mut self) -> Result<(Vec<Arm>, Option<BlockId>), Error> {
		let mut arms = Vec::new();
		while self.tag() == TokenTag::Case {
			self.take(TokenTag::Case)?;
			let mut path = self.parse_name_expr()?;
			while self.tag() == TokenTag::Dot {
				path = self.parse_member_expr(path)?;
			}
			self.take(TokenTag::Then)?;
			let mut body = Vec::new();
			while self.cur.index() < self.toks.len()
				&& self.tag() != TokenTag::Case
				&& self.tag() != TokenTag::Else
				&& self.tag() != TokenTag::End
			{
				let expr_id = self.parse_expr()?;
				body.push(expr_id);
			}
			let body = self.chunk.add_block(Block { exprs: body });
			arms.push(Arm { path, body });
		}
		let else_branch = if self.tag() == TokenTag::Else {
			self.take(TokenTag::Else)?;
			let mut else_branch = Vec::new();
			while self.cur.index() < self.toks.len() && self.tag() != TokenTag::End {
				let expr_id = self.parse_expr()?;
				else_branch.push(expr_id);
			}
			Some(self.chunk.add_block(Block { exprs: else_branch }))
		} else {
			None
		};
		self.take(TokenTag::End)?;
		Ok((arms, else_branch))
	}

	fn parse_do_expr(&mut self) -> Result<ExprId, Error> {
		let tok = self.take(TokenTag::Do)?;
		let mut body = Vec::new();
		while self.cur.index() < self.toks.len()
			&& self.tag() != TokenTag::Rescue
			&& self.tag() != TokenTag::End
		{
			let expr_id = self.parse_expr()?;
			body.push(expr_id);
		}
		let body = self.chunk.add_block(Block { exprs: body });

		let (binding, arms, else_branch) = if self.tag() == TokenTag::Rescue {
			self.take(TokenTag::Rescue)?;
			let binding = self.take(TokenTag::Ident)?.sym.unwrap();
			let (arms, else_branch) = self.parse_case_arms()?;
			(Some(binding), arms, else_branch)
		} else {
			self.take(TokenTag::End)?;
			(None, Vec::new(), None)
		};

		let expr = Expr::Do(Do {
			body,
			binding,
			arms,
			else_branch,
		});
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
		let expr = Expr::Name(Name { sym });
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
				let expr = Expr::Builtin(Builtin::Print { val });
				let expr_id = self.chunk.add_expr(tok.into(), expr);
				Ok(expr_id)
			}
			"$type" => {
				self.take(TokenTag::LParen)?;
				let val = self.parse_expr()?;
				self.take(TokenTag::RParen)?;
				let expr = Expr::Builtin(Builtin::Type { val });
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
