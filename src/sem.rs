use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};

use crate::intern::{Interner, Sym};
use crate::rt::CORE_TYPE_NAMES;
use crate::syn::{
	Access, Arg, Assign, Binary, Block, BlockId, Break, Builtin, Call, Chunk, Def, Each, Expr,
	ExprId, Lit, Location, Loop, Member, Mention, Method, ModuleItem, Param, Place, Return, Source,
	Span, Type, TypeItem, Unary, When,
};

#[derive(Debug)]
pub enum Error {
	PreludeShadowed(Location, String),
	RequiredAfterDefault(Location, String, String),
	BreakOutsideLoop(Location),
}

impl Error {
	pub fn loc(&self) -> &Location {
		match self {
			Self::PreludeShadowed(loc, _) => loc,
			Self::RequiredAfterDefault(loc, ..) => loc,
			Self::BreakOutsideLoop(loc) => loc,
		}
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		write!(f, "{}: semantic error: ", self.loc())?;
		match self {
			Self::PreludeShadowed(_, name) => {
				write!(f, "name '{}' shadows a name in the prelude", name)
			}
			Self::RequiredAfterDefault(_, name, defaulted) => {
				write!(
					f,
					"required param '{}' follows defaulted param '{}'",
					name, defaulted
				)
			}
			Self::BreakOutsideLoop(_) => write!(f, "'break' outside a loop"),
		}
	}
}

pub fn check(syms: &mut Interner, chunk: &Chunk, src: &Source) -> Result<(), Error> {
	let mut prelude = HashSet::new();
	for name in CORE_TYPE_NAMES {
		prelude.insert(syms.intern(name));
	}
	for item_id in &chunk.top {
		let span = chunk.get_module_item_span(*item_id);
		let name = match chunk.get_module_item(*item_id) {
			ModuleItem::Type(type_) => {
				check_type(syms, chunk, src, span, type_)?;
				type_.0
			}
			ModuleItem::Def(def) => {
				check_def(syms, chunk, src, span, def)?;
				def.0
			}
			ModuleItem::Expr(expr_id) => {
				check_expr(chunk, src, *expr_id, 0)?;
				continue;
			}
		};
		if prelude.contains(&name) {
			let loc = src.loc(span.start);
			return Err(Error::PreludeShadowed(loc, syms.resolve(name).to_string()));
		}
	}
	Ok(())
}

fn check_type(
	syms: &Interner,
	chunk: &Chunk,
	src: &Source,
	span: Span,
	Type(_, fields, items): &Type,
) -> Result<(), Error> {
	ensure_required_precede_defaults(syms, src, span, fields)?;
	for item_id in items {
		let span = chunk.get_type_item_span(*item_id);
		match chunk.get_type_item(*item_id) {
			TypeItem::Field(_) => continue,
			TypeItem::Type(inner_type) => check_type(syms, chunk, src, span, inner_type)?,
			TypeItem::Method(Method::Instance(def)) => {
				check_def(syms, chunk, src, span, def)?;
			}
			TypeItem::Method(Method::Static(def)) => {
				check_def(syms, chunk, src, span, def)?;
			}
		}
	}
	Ok(())
}

fn check_def(
	syms: &Interner,
	chunk: &Chunk,
	src: &Source,
	span: Span,
	Def(_, params, body): &Def,
) -> Result<(), Error> {
	ensure_required_precede_defaults(syms, src, span, params)?;
	// A proc body resets the loop-depth counter. break inside a proc can't
	// reach an outer loop, even if the proc itself is lexically nested in one.
	check_block(chunk, src, *body, 0)
}

fn check_block(chunk: &Chunk, src: &Source, block_id: BlockId, depth: u32) -> Result<(), Error> {
	let Block(exprs) = chunk.get_block(block_id);
	for expr_id in exprs {
		check_expr(chunk, src, *expr_id, depth)?;
	}
	Ok(())
}

fn check_expr(chunk: &Chunk, src: &Source, expr_id: ExprId, depth: u32) -> Result<(), Error> {
	match chunk.get_expr(expr_id) {
		Expr::Each(Each(_, iter, body)) => {
			check_expr(chunk, src, *iter, depth)?;
			check_block(chunk, src, *body, depth + 1)
		}
		Expr::Loop(Loop(body)) => check_block(chunk, src, *body, depth + 1),
		Expr::When(When(cond, then_branch, else_branch)) => {
			check_expr(chunk, src, *cond, depth)?;
			check_block(chunk, src, *then_branch, depth)?;
			if let Some(else_branch) = else_branch {
				check_block(chunk, src, *else_branch, depth)?;
			}
			Ok(())
		}
		Expr::Return(Return(val)) => match val {
			Some(val) => check_expr(chunk, src, *val, depth),
			None => Ok(()),
		},
		Expr::Break(Break(val)) => {
			if depth == 0 {
				let span = chunk.get_expr_span(expr_id);
				return Err(Error::BreakOutsideLoop(src.loc(span.start)));
			}
			match val {
				Some(val) => check_expr(chunk, src, *val, depth),
				None => Ok(()),
			}
		}
		Expr::Self_ => Ok(()),
		Expr::Call(Call(callee, args)) => {
			check_expr(chunk, src, *callee, depth)?;
			for Arg(_, arg_id) in args {
				check_expr(chunk, src, *arg_id, depth)?;
			}
			Ok(())
		}
		Expr::Member(Member(recv, _)) => check_expr(chunk, src, *recv, depth),
		Expr::Access(Access(recv, idx)) => {
			check_expr(chunk, src, *recv, depth)?;
			check_expr(chunk, src, *idx, depth)
		}
		Expr::Mention(Mention(inner)) => check_expr(chunk, src, *inner, depth),
		Expr::Assign(Assign(place, val)) => {
			match place {
				Place::Name(_) => {}
				Place::Member(Member(recv, _)) => check_expr(chunk, src, *recv, depth)?,
				Place::Access(Access(recv, idx)) => {
					check_expr(chunk, src, *recv, depth)?;
					check_expr(chunk, src, *idx, depth)?;
				}
			}
			check_expr(chunk, src, *val, depth)
		}
		Expr::Binary(Binary(_, lhs, rhs)) => {
			check_expr(chunk, src, *lhs, depth)?;
			check_expr(chunk, src, *rhs, depth)
		}
		Expr::Unary(Unary(_, inner)) => check_expr(chunk, src, *inner, depth),
		Expr::Name(_) => Ok(()),
		Expr::Builtin(Builtin::Print(inner)) => check_expr(chunk, src, *inner, depth),
		Expr::Lit(lit) => match lit {
			Lit::List(items) => {
				for item in items {
					check_expr(chunk, src, *item, depth)?;
				}
				Ok(())
			}
			Lit::Dict(pairs) => {
				for (key, val) in pairs {
					check_expr(chunk, src, *key, depth)?;
					check_expr(chunk, src, *val, depth)?;
				}
				Ok(())
			}
			Lit::Str(_) | Lit::Char(_) | Lit::Num(_) | Lit::Bool(_) | Lit::Nil => Ok(()),
		},
	}
}

// Ensure that required params precede defaulted ones. This is a load-bearing
// invariant for virtually all arg/param handling.
fn ensure_required_precede_defaults(
	syms: &Interner,
	src: &Source,
	span: Span,
	params: &[Param],
) -> Result<(), Error> {
	let mut defaulted: Option<Sym> = None;
	for Param(name, default) in params {
		if default.is_some() {
			defaulted = Some(*name);
		} else if let Some(earlier) = defaulted {
			// A defaulted param came earlier, so this required one breaks the
			// invariant.
			let loc = src.loc(span.start);
			return Err(Error::RequiredAfterDefault(
				loc,
				syms.resolve(*name).to_string(),
				syms.resolve(earlier).to_string(),
			));
		}
	}
	Ok(())
}
