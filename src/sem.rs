use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display, Formatter};

use crate::intern::{Interner, Sym};
use crate::rt::CORE_TYPE_NAMES;
use crate::syn::{
	Access, Arg, Arm, Assign, Binary, Block, BlockId, Break, Builtin, Call, Chunk, ChunkId, Def,
	Each, Expr, ExprId, Field, Lit, Location, Loop, Match, Member, Mention, Method, ModuleItem,
	Package, Param, Place, Proto, ProtoItem, Return, Span, Type, TypeItem, TypeItemId, Unary, When,
};

#[derive(Debug)]
pub enum Error {
	PreludeShadowed(Location, String),
	RequiredAfterDefault(Location, String, String),
	BreakOutsideLoop(Location),
	ParamsOnCaseParent(Location, String),
	FieldOnCaseParent(Location, String, String),
	ProvidedWithoutRequired(Location, String),
	UnknownProtocol(Location, String),
	MissingMember(Location, String, String, String),
	SignatureMismatch(Location, String, String, String),
	ProtocolConflict(Location, String, String, String),
	MemberCollision(Location, String, String, String),
	ProtocolReach(Location, String, String),
}

impl Error {
	pub fn loc(&self) -> &Location {
		match self {
			Self::PreludeShadowed(loc, _) => loc,
			Self::RequiredAfterDefault(loc, ..) => loc,
			Self::BreakOutsideLoop(loc) => loc,
			Self::ParamsOnCaseParent(loc, _) => loc,
			Self::FieldOnCaseParent(loc, ..) => loc,
			Self::ProvidedWithoutRequired(loc, _) => loc,
			Self::UnknownProtocol(loc, _) => loc,
			Self::MissingMember(loc, ..) => loc,
			Self::SignatureMismatch(loc, ..) => loc,
			Self::ProtocolConflict(loc, ..) => loc,
			Self::MemberCollision(loc, ..) => loc,
			Self::ProtocolReach(loc, ..) => loc,
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
			Self::ParamsOnCaseParent(_, name) => {
				write!(f, "case type '{}' cannot declare params", name)
			}
			Self::FieldOnCaseParent(_, name, field) => {
				write!(f, "case type '{}' cannot declare field '{}'", name, field)
			}
			Self::ProvidedWithoutRequired(_, proto) => {
				write!(f, "protocol '{}' provides members but requires none", proto)
			}
			Self::UnknownProtocol(_, proto) => write!(f, "unknown protocol '{}'", proto),
			Self::MissingMember(_, name, proto, member) => {
				write!(
					f,
					"type '{}' does not implement '{}', required by protocol '{}'",
					name, member, proto
				)
			}
			Self::SignatureMismatch(_, name, proto, member) => {
				write!(
					f,
					"'{}' on type '{}' does not match the signature declared by protocol '{}'",
					member, name, proto
				)
			}
			Self::ProtocolConflict(_, first, second, member) => {
				write!(
					f,
					"protocols '{}' and '{}' both provide '{}'",
					first, second, member
				)
			}
			Self::MemberCollision(_, name, proto, member) => {
				write!(
					f,
					"member '{}' provided by protocol '{}' collides with a member of type '{}'",
					member, proto, name
				)
			}
			Self::ProtocolReach(_, proto, member) => {
				write!(
					f,
					"provided body cannot reference 'self.{}', which protocol '{}' does not declare",
					member, proto
				)
			}
		}
	}
}

pub fn check(syms: &mut Interner, pkg: &Package, chunk_id: ChunkId) -> Result<(), Error> {
	let chunk = pkg.get_chunk(chunk_id);
	let mut prelude = HashSet::new();
	for name in CORE_TYPE_NAMES {
		prelude.insert(syms.intern(name));
	}
	// Iterate all declarations first so that they can be checked
	// order-independently. A protocol is kept with the chunk it was declared
	// in, since its member ids only index that chunk's arena.
	let mut protos = HashMap::new();
	for item_id in &chunk.top {
		let span = chunk.get_module_item_span(*item_id);
		let name = match chunk.get_module_item(*item_id) {
			ModuleItem::Type(Type(name, ..)) => *name,
			ModuleItem::Proto(proto) => {
				protos.insert(proto.0, (chunk_id, proto));
				proto.0
			}
			ModuleItem::Def(Def(name, ..)) => *name,
			ModuleItem::Expr(_) => continue,
		};
		if prelude.contains(&name) {
			let loc = span.loc(pkg);
			return Err(Error::PreludeShadowed(loc, syms.resolve(name).to_string()));
		}
	}
	for item_id in &chunk.top {
		let span = chunk.get_module_item_span(*item_id);
		match chunk.get_module_item(*item_id) {
			ModuleItem::Type(type_) => check_type(syms, chunk, pkg, span, type_, None, &protos)?,
			ModuleItem::Proto(proto) => check_proto(syms, chunk, pkg, span, proto)?,
			ModuleItem::Def(def) => check_def(syms, chunk, pkg, span, def)?,
			ModuleItem::Expr(expr_id) => check_expr(chunk, pkg, *expr_id, 0)?,
		}
	}
	Ok(())
}

fn check_type(
	syms: &Interner,
	chunk: &Chunk,
	pkg: &Package,
	span: Span,
	type_: &Type,
	inherited: Option<&HashMap<Sym, (Span, &Def)>>,
	protos: &HashMap<Sym, (ChunkId, &Proto)>,
) -> Result<(), Error> {
	let Type(name, fields, _, items) = type_;
	ensure_required_precede_defaults(syms, pkg, span, fields)?;

	// A case type's parent cannot have any fields.
	let has_cases = items
		.iter()
		.any(|item_id| match chunk.get_type_item(*item_id) {
			TypeItem::Case(..) => true,
			_ => false,
		});
	if has_cases && !fields.is_empty() {
		let loc = span.loc(pkg);
		return Err(Error::ParamsOnCaseParent(
			loc,
			syms.resolve(*name).to_string(),
		));
	}

	let methods = collect_methods(chunk, items, inherited);
	check_conformance(syms, chunk, pkg, span, type_, &methods, protos)?;

	for item_id in items {
		let span = chunk.get_type_item_span(*item_id);
		match chunk.get_type_item(*item_id) {
			TypeItem::Case(variant) => {
				check_type(syms, chunk, pkg, span, variant, Some(&methods), protos)?
			}
			TypeItem::Field(Field(field, _)) => {
				if has_cases {
					let loc = span.loc(pkg);
					return Err(Error::FieldOnCaseParent(
						loc,
						syms.resolve(*name).to_string(),
						syms.resolve(*field).to_string(),
					));
				}
			}
			TypeItem::Type(inner_type) => {
				check_type(syms, chunk, pkg, span, inner_type, None, protos)?
			}
			TypeItem::Method(Method::Instance(def)) => {
				check_def(syms, chunk, pkg, span, def)?;
			}
			TypeItem::Method(Method::Static(def)) => {
				check_def(syms, chunk, pkg, span, def)?;
			}
		}
	}
	Ok(())
}

fn check_proto(
	syms: &Interner,
	chunk: &Chunk,
	pkg: &Package,
	span: Span,
	Proto(name, items): &Proto,
) -> Result<(), Error> {
	let mut members = HashSet::new();
	for item_id in items {
		let ProtoItem(Def(member, ..)) = chunk.get_proto_item(*item_id);
		members.insert(*member);
	}
	let mut requires = false;
	let mut provides = false;
	for item_id in items {
		let item_span = chunk.get_proto_item_span(*item_id);
		let ProtoItem(def) = chunk.get_proto_item(*item_id);
		check_def(syms, chunk, pkg, item_span, def)?;
		if is_provided(chunk, def) {
			provides = true;
			let Def(_, _, body) = def;
			check_reach_block(syms, chunk, pkg, *body, *name, &members)?;
		} else {
			requires = true;
		}
	}
	// A protocol must either provide a method or only require methods.
	if provides && !requires {
		let loc = span.loc(pkg);
		return Err(Error::ProvidedWithoutRequired(
			loc,
			syms.resolve(*name).to_string(),
		));
	}
	Ok(())
}

// Check that a provided body only reaches the implementing instance through the
// members its own protocol declares.
fn check_reach_block(
	syms: &Interner,
	chunk: &Chunk,
	pkg: &Package,
	block_id: BlockId,
	proto: Sym,
	members: &HashSet<Sym>,
) -> Result<(), Error> {
	let Block(exprs) = chunk.get_block(block_id);
	for expr_id in exprs {
		check_reach_expr(syms, chunk, pkg, *expr_id, proto, members)?;
	}
	Ok(())
}

fn check_reach_expr(
	syms: &Interner,
	chunk: &Chunk,
	pkg: &Package,
	expr_id: ExprId,
	proto: Sym,
	members: &HashSet<Sym>,
) -> Result<(), Error> {
	match chunk.get_expr(expr_id) {
		Expr::Each(Each(_, iter, body)) => {
			check_reach_expr(syms, chunk, pkg, *iter, proto, members)?;
			check_reach_block(syms, chunk, pkg, *body, proto, members)
		}
		Expr::Loop(Loop(body)) => check_reach_block(syms, chunk, pkg, *body, proto, members),
		Expr::When(When(cond, then_branch, else_branch)) => {
			check_reach_expr(syms, chunk, pkg, *cond, proto, members)?;
			check_reach_block(syms, chunk, pkg, *then_branch, proto, members)?;
			if let Some(else_branch) = else_branch {
				check_reach_block(syms, chunk, pkg, *else_branch, proto, members)?;
			}
			Ok(())
		}
		Expr::Match(Match(scrutinee, arms, else_branch)) => {
			check_reach_expr(syms, chunk, pkg, *scrutinee, proto, members)?;
			for Arm(path, body) in arms {
				check_reach_expr(syms, chunk, pkg, *path, proto, members)?;
				check_reach_block(syms, chunk, pkg, *body, proto, members)?;
			}
			if let Some(else_branch) = else_branch {
				check_reach_block(syms, chunk, pkg, *else_branch, proto, members)?;
			}
			Ok(())
		}
		Expr::Return(Return(val)) => match val {
			Some(val) => check_reach_expr(syms, chunk, pkg, *val, proto, members),
			None => Ok(()),
		},
		Expr::Break(Break(val)) => match val {
			Some(val) => check_reach_expr(syms, chunk, pkg, *val, proto, members),
			None => Ok(()),
		},
		Expr::Self_ => Ok(()),
		Expr::Call(Call(callee, args)) => {
			check_reach_expr(syms, chunk, pkg, *callee, proto, members)?;
			for Arg(_, arg_id) in args {
				check_reach_expr(syms, chunk, pkg, *arg_id, proto, members)?;
			}
			Ok(())
		}
		Expr::Member(member) => {
			ensure_reaches_member(syms, chunk, pkg, expr_id, proto, members, member)?;
			let Member(recv, _) = member;
			check_reach_expr(syms, chunk, pkg, *recv, proto, members)
		}
		Expr::Access(Access(recv, idx)) => {
			check_reach_expr(syms, chunk, pkg, *recv, proto, members)?;
			check_reach_expr(syms, chunk, pkg, *idx, proto, members)
		}
		Expr::Mention(Mention(inner)) => check_reach_expr(syms, chunk, pkg, *inner, proto, members),
		Expr::Assign(Assign(place, val)) => {
			match place {
				Place::Name(_) => {}
				Place::Member(member) => {
					ensure_reaches_member(syms, chunk, pkg, expr_id, proto, members, member)?;
					let Member(recv, _) = member;
					check_reach_expr(syms, chunk, pkg, *recv, proto, members)?;
				}
				Place::Access(Access(recv, idx)) => {
					check_reach_expr(syms, chunk, pkg, *recv, proto, members)?;
					check_reach_expr(syms, chunk, pkg, *idx, proto, members)?;
				}
			}
			check_reach_expr(syms, chunk, pkg, *val, proto, members)
		}
		Expr::Binary(Binary(_, lhs, rhs)) => {
			check_reach_expr(syms, chunk, pkg, *lhs, proto, members)?;
			check_reach_expr(syms, chunk, pkg, *rhs, proto, members)
		}
		Expr::Unary(Unary(_, inner)) => check_reach_expr(syms, chunk, pkg, *inner, proto, members),
		Expr::Name(_) => Ok(()),
		Expr::Builtin(Builtin::Print(inner)) => {
			check_reach_expr(syms, chunk, pkg, *inner, proto, members)
		}
		Expr::Lit(lit) => match lit {
			Lit::List(items) => {
				for item in items {
					check_reach_expr(syms, chunk, pkg, *item, proto, members)?;
				}
				Ok(())
			}
			Lit::Dict(pairs) => {
				for (key, val) in pairs {
					check_reach_expr(syms, chunk, pkg, *key, proto, members)?;
					check_reach_expr(syms, chunk, pkg, *val, proto, members)?;
				}
				Ok(())
			}
			Lit::Str(_) | Lit::Char(_) | Lit::Num(_) | Lit::Bool(_) | Lit::Nil => Ok(()),
		},
	}
}

fn ensure_reaches_member(
	syms: &Interner,
	chunk: &Chunk,
	pkg: &Package,
	expr_id: ExprId,
	proto: Sym,
	members: &HashSet<Sym>,
	Member(recv, name): &Member,
) -> Result<(), Error> {
	if let Expr::Self_ = chunk.get_expr(*recv) {
		if !members.contains(name) {
			let span = chunk.get_expr_span(expr_id);
			return Err(Error::ProtocolReach(
				span.loc(pkg),
				syms.resolve(proto).to_string(),
				syms.resolve(*name).to_string(),
			));
		}
	}
	Ok(())
}

fn is_provided(chunk: &Chunk, Def(_, _, body): &Def) -> bool {
	let Block(exprs) = chunk.get_block(*body);
	!exprs.is_empty()
}

fn check_conformance(
	syms: &Interner,
	chunk: &Chunk,
	pkg: &Package,
	span: Span,
	Type(name, _, impls, items): &Type,
	methods: &HashMap<Sym, (Span, &Def)>,
	protos: &HashMap<Sym, (ChunkId, &Proto)>,
) -> Result<(), Error> {
	if impls.is_empty() {
		return Ok(());
	}
	let statics = collect_statics(chunk, items);
	// Provided members this type acquires, against the protocol each came from.
	let mut acquired: HashMap<Sym, Sym> = HashMap::new();
	for impl_name in impls {
		let (proto_chunk_id, proto) = match protos.get(impl_name) {
			Some(proto) => proto,
			None => {
				let loc = span.loc(pkg);
				return Err(Error::UnknownProtocol(
					loc,
					syms.resolve(*impl_name).to_string(),
				));
			}
		};
		// A protocol need not share a chunk with the type implementing it, so
		// its member ids are read through its own arena and never this type's.
		let proto_chunk = pkg.get_chunk(*proto_chunk_id);
		let Proto(proto_name, members) = proto;
		for member_id in members {
			let ProtoItem(def) = proto_chunk.get_proto_item(*member_id);
			let Def(member, params, _) = def;
			if let Some((method_span, Def(_, method_params, _))) = methods.get(member) {
				if !signatures_agree(proto_chunk, chunk, params, method_params) {
					let loc = method_span.loc(pkg);
					return Err(Error::SignatureMismatch(
						loc,
						syms.resolve(*name).to_string(),
						syms.resolve(*proto_name).to_string(),
						syms.resolve(*member).to_string(),
					));
				}
				continue;
			}
			if !is_provided(proto_chunk, def) {
				let loc = span.loc(pkg);
				return Err(Error::MissingMember(
					loc,
					syms.resolve(*name).to_string(),
					syms.resolve(*proto_name).to_string(),
					syms.resolve(*member).to_string(),
				));
			}
			if let Some(static_span) = statics.get(member) {
				let loc = static_span.loc(pkg);
				return Err(Error::MemberCollision(
					loc,
					syms.resolve(*name).to_string(),
					syms.resolve(*proto_name).to_string(),
					syms.resolve(*member).to_string(),
				));
			}
			if let Some(first) = acquired.insert(*member, *proto_name) {
				let loc = span.loc(pkg);
				return Err(Error::ProtocolConflict(
					loc,
					syms.resolve(first).to_string(),
					syms.resolve(*proto_name).to_string(),
					syms.resolve(*member).to_string(),
				));
			}
		}
	}
	Ok(())
}

fn collect_methods<'chunk>(
	chunk: &'chunk Chunk,
	items: &[TypeItemId],
	inherited: Option<&HashMap<Sym, (Span, &'chunk Def)>>,
) -> HashMap<Sym, (Span, &'chunk Def)> {
	let mut methods = match inherited {
		Some(inherited) => inherited.clone(),
		None => HashMap::new(),
	};
	for item_id in items {
		let span = chunk.get_type_item_span(*item_id);
		if let TypeItem::Method(Method::Instance(def)) = chunk.get_type_item(*item_id) {
			methods.insert(def.0, (span, def));
		}
	}
	methods
}

fn collect_statics(chunk: &Chunk, items: &[TypeItemId]) -> HashMap<Sym, Span> {
	let mut statics = HashMap::new();
	for item_id in items {
		let span = chunk.get_type_item_span(*item_id);
		let name = match chunk.get_type_item(*item_id) {
			TypeItem::Case(Type(name, ..)) => *name,
			TypeItem::Field(..) => continue,
			TypeItem::Type(Type(name, ..)) => *name,
			TypeItem::Method(Method::Instance(..)) => continue,
			TypeItem::Method(Method::Static(Def(name, ..))) => *name,
		};
		statics.insert(name, span);
	}
	statics
}

fn signatures_agree(
	proto_chunk: &Chunk,
	type_chunk: &Chunk,
	proto: &[Param],
	typ: &[Param],
) -> bool {
	if proto.len() != typ.len() {
		return false;
	}
	for (Param(proto_name, proto_default), Param(name, default)) in proto.iter().zip(typ) {
		if proto_name != name {
			return false;
		}
		if !defaults_agree(proto_chunk, type_chunk, *proto_default, *default) {
			return false;
		}
	}
	true
}

fn defaults_agree(
	proto_chunk: &Chunk,
	type_chunk: &Chunk,
	proto: Option<ExprId>,
	typ: Option<ExprId>,
) -> bool {
	let (proto, typ) = match (proto, typ) {
		(Some(proto), Some(typ)) => (proto, typ),
		(None, None) => return true,
		_ => return false,
	};
	// Only literals are compared; anything else is taken to agree until there
	// is a structural comparison over the arena.
	match (proto_chunk.get_expr(proto), type_chunk.get_expr(typ)) {
		(Expr::Lit(proto), Expr::Lit(typ)) => lits_agree(proto, typ),
		_ => true,
	}
}

fn lits_agree(proto: &Lit, typ: &Lit) -> bool {
	match (proto, typ) {
		(Lit::Str(proto), Lit::Str(typ)) => proto == typ,
		(Lit::Char(proto), Lit::Char(typ)) => proto == typ,
		(Lit::Num(proto), Lit::Num(typ)) => proto == typ,
		(Lit::Bool(proto), Lit::Bool(typ)) => proto == typ,
		(Lit::List(..), _) | (_, Lit::List(..)) => true,
		(Lit::Dict(..), _) | (_, Lit::Dict(..)) => true,
		(Lit::Nil, Lit::Nil) => true,
		_ => false,
	}
}

fn check_def(
	syms: &Interner,
	chunk: &Chunk,
	pkg: &Package,
	span: Span,
	Def(_, params, body): &Def,
) -> Result<(), Error> {
	ensure_required_precede_defaults(syms, pkg, span, params)?;
	// A proc body resets the loop-depth counter. break inside a proc can't
	// reach an outer loop, even if the proc itself is lexically nested in one.
	check_block(chunk, pkg, *body, 0)
}

fn check_block(chunk: &Chunk, pkg: &Package, block_id: BlockId, depth: u32) -> Result<(), Error> {
	let Block(exprs) = chunk.get_block(block_id);
	for expr_id in exprs {
		check_expr(chunk, pkg, *expr_id, depth)?;
	}
	Ok(())
}

fn check_expr(chunk: &Chunk, pkg: &Package, expr_id: ExprId, depth: u32) -> Result<(), Error> {
	match chunk.get_expr(expr_id) {
		Expr::Each(Each(_, iter, body)) => {
			check_expr(chunk, pkg, *iter, depth)?;
			check_block(chunk, pkg, *body, depth + 1)
		}
		Expr::Loop(Loop(body)) => check_block(chunk, pkg, *body, depth + 1),
		Expr::When(When(cond, then_branch, else_branch)) => {
			check_expr(chunk, pkg, *cond, depth)?;
			check_block(chunk, pkg, *then_branch, depth)?;
			if let Some(else_branch) = else_branch {
				check_block(chunk, pkg, *else_branch, depth)?;
			}
			Ok(())
		}
		Expr::Match(Match(scrutinee, arms, else_branch)) => {
			check_expr(chunk, pkg, *scrutinee, depth)?;
			for Arm(path, body) in arms {
				check_expr(chunk, pkg, *path, depth)?;
				check_block(chunk, pkg, *body, depth)?;
			}
			if let Some(else_branch) = else_branch {
				check_block(chunk, pkg, *else_branch, depth)?;
			}
			Ok(())
		}
		Expr::Return(Return(val)) => match val {
			Some(val) => check_expr(chunk, pkg, *val, depth),
			None => Ok(()),
		},
		Expr::Break(Break(val)) => {
			if depth == 0 {
				let span = chunk.get_expr_span(expr_id);
				return Err(Error::BreakOutsideLoop(span.loc(pkg)));
			}
			match val {
				Some(val) => check_expr(chunk, pkg, *val, depth),
				None => Ok(()),
			}
		}
		Expr::Self_ => Ok(()),
		Expr::Call(Call(callee, args)) => {
			check_expr(chunk, pkg, *callee, depth)?;
			for Arg(_, arg_id) in args {
				check_expr(chunk, pkg, *arg_id, depth)?;
			}
			Ok(())
		}
		Expr::Member(Member(recv, _)) => check_expr(chunk, pkg, *recv, depth),
		Expr::Access(Access(recv, idx)) => {
			check_expr(chunk, pkg, *recv, depth)?;
			check_expr(chunk, pkg, *idx, depth)
		}
		Expr::Mention(Mention(inner)) => check_expr(chunk, pkg, *inner, depth),
		Expr::Assign(Assign(place, val)) => {
			match place {
				Place::Name(_) => {}
				Place::Member(Member(recv, _)) => check_expr(chunk, pkg, *recv, depth)?,
				Place::Access(Access(recv, idx)) => {
					check_expr(chunk, pkg, *recv, depth)?;
					check_expr(chunk, pkg, *idx, depth)?;
				}
			}
			check_expr(chunk, pkg, *val, depth)
		}
		Expr::Binary(Binary(_, lhs, rhs)) => {
			check_expr(chunk, pkg, *lhs, depth)?;
			check_expr(chunk, pkg, *rhs, depth)
		}
		Expr::Unary(Unary(_, inner)) => check_expr(chunk, pkg, *inner, depth),
		Expr::Name(_) => Ok(()),
		Expr::Builtin(Builtin::Print(inner)) => check_expr(chunk, pkg, *inner, depth),
		Expr::Lit(lit) => match lit {
			Lit::List(items) => {
				for item in items {
					check_expr(chunk, pkg, *item, depth)?;
				}
				Ok(())
			}
			Lit::Dict(pairs) => {
				for (key, val) in pairs {
					check_expr(chunk, pkg, *key, depth)?;
					check_expr(chunk, pkg, *val, depth)?;
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
	pkg: &Package,
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
			let loc = span.loc(pkg);
			return Err(Error::RequiredAfterDefault(
				loc,
				syms.resolve(*name).to_string(),
				syms.resolve(earlier).to_string(),
			));
		}
	}
	Ok(())
}
