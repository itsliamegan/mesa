use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display, Formatter};

use crate::intern::{Interner, Sym};
use crate::rt::CORE_TYPE_NAMES;
use crate::syn::{
	BlockId, Builtin, Chunk, ChunkId, Def, Expr, ExprId, Lit, Location, Member, Method, ModuleItem,
	Package, Param, Place, Proto, Span, Type, TypeItem, TypeItemId,
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

pub fn check(syms: &mut Interner, pkg: &Package, chunk_id: ChunkId) -> Result<(), Vec<Error>> {
	let chunk = pkg.get_chunk(chunk_id);
	let mut errs = Vec::new();

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
			ModuleItem::Type(type_) => type_.name,
			ModuleItem::Proto(proto) => {
				protos.insert(proto.name, (chunk_id, proto));
				proto.name
			}
			ModuleItem::Def(def) => def.name,
			ModuleItem::Expr(_) => continue,
		};
		if prelude.contains(&name) {
			let loc = span.loc(pkg);
			errs.push(Error::PreludeShadowed(loc, syms.resolve(name).to_string()));
		}
	}

	if !errs.is_empty() {
		return Err(errs);
	}

	for item_id in &chunk.top {
		let span = chunk.get_module_item_span(*item_id);
		let result = match chunk.get_module_item(*item_id) {
			ModuleItem::Type(type_) => check_type(syms, chunk, pkg, span, type_, None, &protos),
			ModuleItem::Proto(proto) => check_proto(syms, chunk, pkg, span, proto),
			ModuleItem::Def(def) => check_def(syms, chunk, pkg, span, def),
			ModuleItem::Expr(expr_id) => check_expr(chunk, pkg, *expr_id, 0),
		};
		if let Err(err) = result {
			errs.push(err);
		}
	}

	if !errs.is_empty() { Err(errs) } else { Ok(()) }
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
	ensure_required_precede_defaults(syms, pkg, span, &type_.params)?;

	// A case type's parent cannot have any fields.
	let has_cases = type_
		.items
		.iter()
		.any(|item_id| match chunk.get_type_item(*item_id) {
			TypeItem::Case(..) => true,
			_ => false,
		});
	if has_cases && !type_.params.is_empty() {
		let loc = span.loc(pkg);
		return Err(Error::ParamsOnCaseParent(
			loc,
			syms.resolve(type_.name).to_string(),
		));
	}

	let methods = collect_methods(chunk, &type_.items, inherited);
	check_conformance(syms, chunk, pkg, span, type_, &methods, protos)?;

	for item_id in &type_.items {
		let span = chunk.get_type_item_span(*item_id);
		match chunk.get_type_item(*item_id) {
			TypeItem::Case(variant) => {
				check_type(syms, chunk, pkg, span, variant, Some(&methods), protos)?
			}
			TypeItem::Field(field) => {
				if has_cases {
					let loc = span.loc(pkg);
					return Err(Error::FieldOnCaseParent(
						loc,
						syms.resolve(type_.name).to_string(),
						syms.resolve(field.name).to_string(),
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
	proto: &Proto,
) -> Result<(), Error> {
	let mut members = HashSet::new();
	for item_id in &proto.items {
		let item = chunk.get_proto_item(*item_id);
		members.insert(item.def.name);
	}
	let mut requires = false;
	let mut provides = false;
	for item_id in &proto.items {
		let item_span = chunk.get_proto_item_span(*item_id);
		let item = chunk.get_proto_item(*item_id);
		let def = &item.def;
		check_def(syms, chunk, pkg, item_span, def)?;
		if is_provided(chunk, def) {
			provides = true;
			check_reach_block(syms, chunk, pkg, def.body, proto.name, &members)?;
		} else {
			requires = true;
		}
	}
	// A protocol must either provide a method or only require methods.
	if provides && !requires {
		let loc = span.loc(pkg);
		return Err(Error::ProvidedWithoutRequired(
			loc,
			syms.resolve(proto.name).to_string(),
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
	let block = chunk.get_block(block_id);
	for expr_id in &block.exprs {
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
		Expr::Each(each) => {
			check_reach_expr(syms, chunk, pkg, each.iter, proto, members)?;
			check_reach_block(syms, chunk, pkg, each.body, proto, members)
		}
		Expr::Loop(loop_) => check_reach_block(syms, chunk, pkg, loop_.body, proto, members),
		Expr::When(when) => {
			check_reach_expr(syms, chunk, pkg, when.cond, proto, members)?;
			check_reach_block(syms, chunk, pkg, when.then_branch, proto, members)?;
			if let Some(else_branch) = when.else_branch {
				check_reach_block(syms, chunk, pkg, else_branch, proto, members)?;
			}
			Ok(())
		}
		Expr::Match(match_) => {
			check_reach_expr(syms, chunk, pkg, match_.scrutinee, proto, members)?;
			for arm in &match_.arms {
				check_reach_expr(syms, chunk, pkg, arm.path, proto, members)?;
				check_reach_block(syms, chunk, pkg, arm.body, proto, members)?;
			}
			if let Some(else_branch) = match_.else_branch {
				check_reach_block(syms, chunk, pkg, else_branch, proto, members)?;
			}
			Ok(())
		}
		Expr::Return(return_) => match return_.val {
			Some(val) => check_reach_expr(syms, chunk, pkg, val, proto, members),
			None => Ok(()),
		},
		Expr::Break(break_) => match break_.val {
			Some(val) => check_reach_expr(syms, chunk, pkg, val, proto, members),
			None => Ok(()),
		},
		Expr::Self_ => Ok(()),
		Expr::Call(call) => {
			check_reach_expr(syms, chunk, pkg, call.callee, proto, members)?;
			for arg in &call.args {
				check_reach_expr(syms, chunk, pkg, arg.val, proto, members)?;
			}
			Ok(())
		}
		Expr::Member(member) => {
			ensure_reaches_member(syms, chunk, pkg, expr_id, proto, members, member)?;
			check_reach_expr(syms, chunk, pkg, member.receiver, proto, members)
		}
		Expr::Access(access) => {
			check_reach_expr(syms, chunk, pkg, access.receiver, proto, members)?;
			check_reach_expr(syms, chunk, pkg, access.key, proto, members)
		}
		Expr::Mention(mention) => check_reach_expr(syms, chunk, pkg, mention.val, proto, members),
		Expr::Assign(assign) => {
			match &assign.place {
				Place::Name(_) => {}
				Place::Member(member) => {
					ensure_reaches_member(syms, chunk, pkg, expr_id, proto, members, member)?;
					check_reach_expr(syms, chunk, pkg, member.receiver, proto, members)?;
				}
				Place::Access(access) => {
					check_reach_expr(syms, chunk, pkg, access.receiver, proto, members)?;
					check_reach_expr(syms, chunk, pkg, access.key, proto, members)?;
				}
			}
			check_reach_expr(syms, chunk, pkg, assign.val, proto, members)
		}
		Expr::Binary(binary) => {
			check_reach_expr(syms, chunk, pkg, binary.lhs, proto, members)?;
			check_reach_expr(syms, chunk, pkg, binary.rhs, proto, members)
		}
		Expr::Unary(unary) => check_reach_expr(syms, chunk, pkg, unary.val, proto, members),
		Expr::Name(_) => Ok(()),
		Expr::Builtin(Builtin::Print { val }) => {
			check_reach_expr(syms, chunk, pkg, *val, proto, members)
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
	member: &Member,
) -> Result<(), Error> {
	if let Expr::Self_ = chunk.get_expr(member.receiver) {
		if !members.contains(&member.name) {
			let span = chunk.get_expr_span(expr_id);
			return Err(Error::ProtocolReach(
				span.loc(pkg),
				syms.resolve(proto).to_string(),
				syms.resolve(member.name).to_string(),
			));
		}
	}
	Ok(())
}

fn is_provided(chunk: &Chunk, def: &Def) -> bool {
	let block = chunk.get_block(def.body);
	!block.exprs.is_empty()
}

fn check_conformance(
	syms: &Interner,
	chunk: &Chunk,
	pkg: &Package,
	span: Span,
	type_: &Type,
	methods: &HashMap<Sym, (Span, &Def)>,
	protos: &HashMap<Sym, (ChunkId, &Proto)>,
) -> Result<(), Error> {
	if type_.impls.is_empty() {
		return Ok(());
	}
	let statics = collect_statics(chunk, &type_.items);
	// Provided members this type acquires, against the protocol each came from.
	let mut acquired: HashMap<Sym, Sym> = HashMap::new();
	for impl_name in &type_.impls {
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
		for member_id in &proto.items {
			let item = proto_chunk.get_proto_item(*member_id);
			let def = &item.def;
			let member = &def.name;
			if let Some((method_span, method_def)) = methods.get(member) {
				if !signatures_agree(proto_chunk, chunk, &def.params, &method_def.params) {
					let loc = method_span.loc(pkg);
					return Err(Error::SignatureMismatch(
						loc,
						syms.resolve(type_.name).to_string(),
						syms.resolve(proto.name).to_string(),
						syms.resolve(*member).to_string(),
					));
				}
				continue;
			}
			if !is_provided(proto_chunk, def) {
				let loc = span.loc(pkg);
				return Err(Error::MissingMember(
					loc,
					syms.resolve(type_.name).to_string(),
					syms.resolve(proto.name).to_string(),
					syms.resolve(*member).to_string(),
				));
			}
			if let Some(static_span) = statics.get(member) {
				let loc = static_span.loc(pkg);
				return Err(Error::MemberCollision(
					loc,
					syms.resolve(type_.name).to_string(),
					syms.resolve(proto.name).to_string(),
					syms.resolve(*member).to_string(),
				));
			}
			if let Some(first) = acquired.insert(*member, proto.name) {
				let loc = span.loc(pkg);
				return Err(Error::ProtocolConflict(
					loc,
					syms.resolve(first).to_string(),
					syms.resolve(proto.name).to_string(),
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
			methods.insert(def.name, (span, def));
		}
	}
	methods
}

fn collect_statics(chunk: &Chunk, items: &[TypeItemId]) -> HashMap<Sym, Span> {
	let mut statics = HashMap::new();
	for item_id in items {
		let span = chunk.get_type_item_span(*item_id);
		let name = match chunk.get_type_item(*item_id) {
			TypeItem::Case(type_) => type_.name,
			TypeItem::Field(..) => continue,
			TypeItem::Type(type_) => type_.name,
			TypeItem::Method(Method::Instance(..)) => continue,
			TypeItem::Method(Method::Static(def)) => def.name,
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
	for (proto_param, param) in proto.iter().zip(typ) {
		if proto_param.name != param.name {
			return false;
		}
		if !defaults_agree(proto_chunk, type_chunk, proto_param.default, param.default) {
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
	def: &Def,
) -> Result<(), Error> {
	ensure_required_precede_defaults(syms, pkg, span, &def.params)?;
	// A proc body resets the loop-depth counter. break inside a proc can't
	// reach an outer loop, even if the proc itself is lexically nested in one.
	check_block(chunk, pkg, def.body, 0)
}

fn check_block(chunk: &Chunk, pkg: &Package, block_id: BlockId, depth: u32) -> Result<(), Error> {
	let block = chunk.get_block(block_id);
	for expr_id in &block.exprs {
		check_expr(chunk, pkg, *expr_id, depth)?;
	}
	Ok(())
}

fn check_expr(chunk: &Chunk, pkg: &Package, expr_id: ExprId, depth: u32) -> Result<(), Error> {
	match chunk.get_expr(expr_id) {
		Expr::Each(each) => {
			check_expr(chunk, pkg, each.iter, depth)?;
			check_block(chunk, pkg, each.body, depth + 1)
		}
		Expr::Loop(loop_) => check_block(chunk, pkg, loop_.body, depth + 1),
		Expr::When(when) => {
			check_expr(chunk, pkg, when.cond, depth)?;
			check_block(chunk, pkg, when.then_branch, depth)?;
			if let Some(else_branch) = when.else_branch {
				check_block(chunk, pkg, else_branch, depth)?;
			}
			Ok(())
		}
		Expr::Match(match_) => {
			check_expr(chunk, pkg, match_.scrutinee, depth)?;
			for arm in &match_.arms {
				check_expr(chunk, pkg, arm.path, depth)?;
				check_block(chunk, pkg, arm.body, depth)?;
			}
			if let Some(else_branch) = match_.else_branch {
				check_block(chunk, pkg, else_branch, depth)?;
			}
			Ok(())
		}
		Expr::Return(return_) => match return_.val {
			Some(val) => check_expr(chunk, pkg, val, depth),
			None => Ok(()),
		},
		Expr::Break(break_) => {
			if depth == 0 {
				let span = chunk.get_expr_span(expr_id);
				return Err(Error::BreakOutsideLoop(span.loc(pkg)));
			}
			match break_.val {
				Some(val) => check_expr(chunk, pkg, val, depth),
				None => Ok(()),
			}
		}
		Expr::Self_ => Ok(()),
		Expr::Call(call) => {
			check_expr(chunk, pkg, call.callee, depth)?;
			for arg in &call.args {
				check_expr(chunk, pkg, arg.val, depth)?;
			}
			Ok(())
		}
		Expr::Member(member) => check_expr(chunk, pkg, member.receiver, depth),
		Expr::Access(access) => {
			check_expr(chunk, pkg, access.receiver, depth)?;
			check_expr(chunk, pkg, access.key, depth)
		}
		Expr::Mention(mention) => check_expr(chunk, pkg, mention.val, depth),
		Expr::Assign(assign) => {
			match &assign.place {
				Place::Name(_) => {}
				Place::Member(member) => check_expr(chunk, pkg, member.receiver, depth)?,
				Place::Access(access) => {
					check_expr(chunk, pkg, access.receiver, depth)?;
					check_expr(chunk, pkg, access.key, depth)?;
				}
			}
			check_expr(chunk, pkg, assign.val, depth)
		}
		Expr::Binary(binary) => {
			check_expr(chunk, pkg, binary.lhs, depth)?;
			check_expr(chunk, pkg, binary.rhs, depth)
		}
		Expr::Unary(unary) => check_expr(chunk, pkg, unary.val, depth),
		Expr::Name(_) => Ok(()),
		Expr::Builtin(Builtin::Print { val }) => check_expr(chunk, pkg, *val, depth),
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
	for param in params {
		if param.default.is_some() {
			defaulted = Some(param.name);
		} else if let Some(earlier) = defaulted {
			// A defaulted param came earlier, so this required one breaks the
			// invariant.
			let loc = span.loc(pkg);
			return Err(Error::RequiredAfterDefault(
				loc,
				syms.resolve(param.name).to_string(),
				syms.resolve(earlier).to_string(),
			));
		}
	}
	Ok(())
}
