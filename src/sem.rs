pub mod modules;
pub mod types;

use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

use crate::intern::{Interner, Sym};
use crate::pkg::{PackageId, Packages};
use crate::sem::modules::{ModuleId, Modules};
use crate::sem::types::Types;
use crate::src::{Location, Sources, Span};
use crate::syn::nodes::{
	BlockId, Builtin, Def, Expr, ExprId, Lit, Member, Method, ModuleItem, Param, Place, TypeItem,
};
use crate::syn::{Chunk, Chunks};

#[derive(Debug)]
pub enum Error {
	RequiredAfterDefault(Location, String, String),
	BreakOutsideLoop(Location),
	ParamsOnCaseParent(Location, String),
	FieldOnCaseParent(Location, String, String),
	ProvidedWithoutRequired(Location, String),
	UnknownProtocol(Location, String),
	NotAProtocol(Location, String),
	MissingMember(Location, String, String, String),
	SignatureMismatch(Location, String, String, String),
	ProtocolConflict(Location, String, String, String),
	MemberCollision(Location, String, String, String),
	NativeMemberCollision(Location, String, String),
	UnimplementedExtern(Location, String),
	UndeclaredNativeMember(Location, String, String),
	UnimplementedExternMember(Location, String, String),
	ExternSignatureMismatch(Location, String, String),
	ProtocolReach(Location, String, String),
	MissingModuleHeader(Location),
	DuplicateModuleHeader(Location),
	MisplacedModuleHeader(Location),
	UnpairedDirectory(Location),
	ReservedDirectory(Location),
	PrefixMismatch(Location, String, String),
	DuplicateModuleName(Location, String),
	ConflictingModuleName(Location, String, String),
	DuplicateMember(Location, String),
	UnknownImport(Location, String),
	ImportCycle(Location, String),
}

impl Error {
	pub fn loc(&self) -> &Location {
		match self {
			Self::RequiredAfterDefault(loc, ..) => loc,
			Self::BreakOutsideLoop(loc) => loc,
			Self::ParamsOnCaseParent(loc, _) => loc,
			Self::FieldOnCaseParent(loc, ..) => loc,
			Self::ProvidedWithoutRequired(loc, _) => loc,
			Self::UnknownProtocol(loc, _) => loc,
			Self::NotAProtocol(loc, _) => loc,
			Self::MissingMember(loc, ..) => loc,
			Self::SignatureMismatch(loc, ..) => loc,
			Self::ProtocolConflict(loc, ..) => loc,
			Self::MemberCollision(loc, ..) => loc,
			Self::NativeMemberCollision(loc, ..) => loc,
			Self::UnimplementedExtern(loc, _) => loc,
			Self::UndeclaredNativeMember(loc, ..) => loc,
			Self::UnimplementedExternMember(loc, ..) => loc,
			Self::ExternSignatureMismatch(loc, ..) => loc,
			Self::ProtocolReach(loc, ..) => loc,
			Self::MissingModuleHeader(loc) => loc,
			Self::DuplicateModuleHeader(loc) => loc,
			Self::MisplacedModuleHeader(loc) => loc,
			Self::UnpairedDirectory(loc) => loc,
			Self::ReservedDirectory(loc) => loc,
			Self::PrefixMismatch(loc, ..) => loc,
			Self::DuplicateModuleName(loc, _) => loc,
			Self::ConflictingModuleName(loc, ..) => loc,
			Self::DuplicateMember(loc, _) => loc,
			Self::UnknownImport(loc, _) => loc,
			Self::ImportCycle(loc, _) => loc,
		}
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		write!(f, "{}: semantic error: ", self.loc())?;
		match self {
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
			Self::NotAProtocol(_, name) => write!(f, "'{}' is not a protocol", name),
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
			Self::NativeMemberCollision(_, name, member) => {
				write!(
					f,
					"member '{}' of extern type '{}' is declared more than once",
					member, name
				)
			}
			Self::UnimplementedExtern(_, name) => {
				write!(f, "extern type '{}' has no native implementation", name)
			}
			Self::UndeclaredNativeMember(_, name, member) => {
				write!(
					f,
					"member '{}' of extern type '{}' is implemented natively but not declared",
					member, name
				)
			}
			Self::UnimplementedExternMember(_, name, member) => {
				write!(
					f,
					"member '{}' of extern type '{}' has no native implementation",
					member, name
				)
			}
			Self::ExternSignatureMismatch(_, name, member) => {
				write!(
					f,
					"member '{}' of extern type '{}' does not match its native implementation",
					member, name
				)
			}
			Self::ProtocolReach(_, proto, member) => {
				write!(
					f,
					"provided body cannot reference 'self.{}', which protocol '{}' does not declare",
					member, proto
				)
			}
			Self::MissingModuleHeader(_) => write!(f, "file declares no module"),
			Self::DuplicateModuleHeader(_) => write!(f, "file declares more than one module"),
			Self::MisplacedModuleHeader(_) => {
				write!(f, "'module' must be the first item in a file")
			}
			Self::UnpairedDirectory(_) => {
				write!(f, "directory has no sibling module file")
			}
			Self::ReservedDirectory(_) => {
				write!(
					f,
					"'src/package/' is reserved; the root module's children live in src/"
				)
			}
			Self::PrefixMismatch(_, declared, prefix) => {
				write!(
					f,
					"module '{}' must be declared under '{}'",
					declared, prefix
				)
			}
			Self::DuplicateModuleName(_, name) => write!(f, "duplicate module '{}'", name),
			Self::ConflictingModuleName(_, name, pkg) => {
				write!(f, "module '{}' conflicts with package '{}'", name, pkg)
			}
			Self::DuplicateMember(_, name) => write!(f, "duplicate member '{}'", name),
			Self::UnknownImport(_, path) => write!(f, "unknown import '{}'", path),
			Self::ImportCycle(_, cycle) => write!(f, "import cycle: {}", cycle),
		}
	}
}

pub fn check(
	syms: &Interner,
	pkgs: &Packages,
	pkg_id: PackageId,
	sources: &Sources,
	chunks: &Chunks,
	natives: &HashMap<Sym, types::NativeTypeShape>,
) -> Result<(Modules, Types), Vec<Error>> {
	let mods = modules::check(syms, pkgs, pkg_id, sources, chunks)?;

	// Describing resolves 'impl' names and decides conformance itself, so it
	// tolerates the package not yet being known valid; it runs before the
	// chunk walk rather than after, and its errors precede that walk's in the
	// bundle, mirroring how module errors already precede every chunk error.
	let (types, mut errs) = types::check(syms, sources, chunks, pkgs, &mods, natives);

	// Every chunk is a module by now, module checking having failed otherwise,
	// and the modules are held in chunk order, so this reports in file order.
	for id in mods.ids() {
		if let Err(mut chunk_errs) = check_chunk(syms, sources, chunks, &mods, &types, id) {
			errs.append(&mut chunk_errs);
		}
	}

	if !errs.is_empty() {
		return Err(errs);
	}

	Ok((mods, types))
}

fn check_chunk(
	syms: &Interner,
	sources: &Sources,
	chunks: &Chunks,
	mods: &Modules,
	types: &Types,
	module: ModuleId,
) -> Result<(), Vec<Error>> {
	let chunk_id = mods.chunk(module);
	let chunk = chunks.get(chunk_id);
	let mut errs = Vec::new();

	// 'Def' has no expression-level variant so every def in the chunk is
	// reachable as a 'ModuleItem', a 'TypeItem::Method' or a 'ProtoItem'. A
	// flat walk over the three arenas finds all of them, once each, with no
	// structure to descend, however deeply the type declaring one is nested.
	for item_id in &chunk.top {
		let span = chunk.get_module_item_span(*item_id);
		let result = match chunk.get_module_item(*item_id) {
			ModuleItem::Module(_) => Ok(()),
			ModuleItem::Import(_) => Ok(()),
			ModuleItem::Export(_) => Ok(()),
			ModuleItem::Type(_) => Ok(()),
			ModuleItem::Extern(_) => Ok(()),
			ModuleItem::Proto(_) => Ok(()),
			ModuleItem::Def(def) => check_def(syms, sources, chunk, span, def),
			ModuleItem::Expr(expr_id) => check_expr(sources, chunk, *expr_id, 0),
		};
		if let Err(err) = result {
			errs.push(err);
		}
	}

	for item_id in chunk.type_item_ids() {
		let def = match chunk.get_type_item(item_id) {
			TypeItem::Method(Method::Instance(def)) => def,
			TypeItem::Method(Method::Static(def)) => def,
			TypeItem::Case(..)
			| TypeItem::Field(..)
			| TypeItem::Type(..)
			| TypeItem::Extern(..) => continue,
		};
		let span = chunk.get_type_item_span(item_id);
		if let Err(err) = check_def(syms, sources, chunk, span, def) {
			errs.push(err);
		}
	}

	// Check that none of the provided members violate standard block-level
	// invariants or reach the implementing instance outside of the protocol's
	// declared members.
	for item_id in chunk.proto_item_ids() {
		let item_span = chunk.get_proto_item_span(item_id);
		let item = chunk.get_proto_item(item_id);
		let def = &item.def;
		if let Err(err) = check_def(syms, sources, chunk, item_span, def) {
			errs.push(err);
			continue;
		}
		let proto_id = types.get_proto_by_proto_item(chunk_id, item_id);
		let desc = types.get_proto(proto_id);
		if desc.provided.contains_key(&def.name)
			&& let Err(err) = check_reach_block(syms, sources, chunk, def.body, desc.name, desc)
		{
			errs.push(err);
		}
	}

	if !errs.is_empty() {
		return Err(errs);
	}

	Ok(())
}

// Check that a provided body only reaches the implementing instance through the
// members its own protocol declares.
fn check_reach_block(
	syms: &Interner,
	sources: &Sources,
	chunk: &Chunk,
	block_id: BlockId,
	proto: Sym,
	desc: &types::Proto,
) -> Result<(), Error> {
	let block = chunk.get_block(block_id);
	for expr_id in &block.exprs {
		check_reach_expr(syms, sources, chunk, *expr_id, proto, desc)?;
	}
	Ok(())
}

fn check_reach_expr(
	syms: &Interner,
	sources: &Sources,
	chunk: &Chunk,
	expr_id: ExprId,
	proto: Sym,
	desc: &types::Proto,
) -> Result<(), Error> {
	match chunk.get_expr(expr_id) {
		Expr::Each(each) => {
			check_reach_expr(syms, sources, chunk, each.iter, proto, desc)?;
			check_reach_block(syms, sources, chunk, each.body, proto, desc)
		}
		Expr::Loop(loop_) => check_reach_block(syms, sources, chunk, loop_.body, proto, desc),
		Expr::When(when) => {
			check_reach_expr(syms, sources, chunk, when.cond, proto, desc)?;
			check_reach_block(syms, sources, chunk, when.then_branch, proto, desc)?;
			if let Some(else_branch) = when.else_branch {
				check_reach_block(syms, sources, chunk, else_branch, proto, desc)?;
			}
			Ok(())
		}
		Expr::Match(match_) => {
			check_reach_expr(syms, sources, chunk, match_.scrutinee, proto, desc)?;
			for arm in &match_.arms {
				check_reach_expr(syms, sources, chunk, arm.path, proto, desc)?;
				check_reach_block(syms, sources, chunk, arm.body, proto, desc)?;
			}
			if let Some(else_branch) = match_.else_branch {
				check_reach_block(syms, sources, chunk, else_branch, proto, desc)?;
			}
			Ok(())
		}
		Expr::Do(do_) => {
			check_reach_block(syms, sources, chunk, do_.body, proto, desc)?;
			for arm in &do_.arms {
				check_reach_expr(syms, sources, chunk, arm.path, proto, desc)?;
				check_reach_block(syms, sources, chunk, arm.body, proto, desc)?;
			}
			if let Some(else_branch) = do_.else_branch {
				check_reach_block(syms, sources, chunk, else_branch, proto, desc)?;
			}
			Ok(())
		}
		Expr::Return(return_) => match return_.val {
			Some(val) => check_reach_expr(syms, sources, chunk, val, proto, desc),
			None => Ok(()),
		},
		Expr::Break(break_) => match break_.val {
			Some(val) => check_reach_expr(syms, sources, chunk, val, proto, desc),
			None => Ok(()),
		},
		Expr::Raise(raise) => check_reach_expr(syms, sources, chunk, raise.val, proto, desc),
		Expr::Self_ => Ok(()),
		Expr::Call(call) => {
			check_reach_expr(syms, sources, chunk, call.callee, proto, desc)?;
			for arg in &call.args {
				check_reach_expr(syms, sources, chunk, arg.val, proto, desc)?;
			}
			Ok(())
		}
		Expr::Member(member) => {
			check_reaches_member(syms, sources, chunk, expr_id, proto, desc, member)?;
			check_reach_expr(syms, sources, chunk, member.receiver, proto, desc)
		}
		Expr::Access(access) => {
			check_reach_expr(syms, sources, chunk, access.receiver, proto, desc)?;
			check_reach_expr(syms, sources, chunk, access.key, proto, desc)
		}
		Expr::Mention(mention) => check_reach_expr(syms, sources, chunk, mention.val, proto, desc),
		Expr::Assign(assign) => {
			match &assign.place {
				Place::Name(_) => {}
				Place::Member(member) => {
					check_reaches_member(syms, sources, chunk, expr_id, proto, desc, member)?;
					check_reach_expr(syms, sources, chunk, member.receiver, proto, desc)?;
				}
				Place::Access(access) => {
					check_reach_expr(syms, sources, chunk, access.receiver, proto, desc)?;
					check_reach_expr(syms, sources, chunk, access.key, proto, desc)?;
				}
			}
			check_reach_expr(syms, sources, chunk, assign.val, proto, desc)
		}
		Expr::Binary(binary) => {
			check_reach_expr(syms, sources, chunk, binary.lhs, proto, desc)?;
			check_reach_expr(syms, sources, chunk, binary.rhs, proto, desc)
		}
		Expr::Unary(unary) => check_reach_expr(syms, sources, chunk, unary.val, proto, desc),
		Expr::Name(_) => Ok(()),
		Expr::Builtin(builtin) => match builtin {
			Builtin::Print { val } => check_reach_expr(syms, sources, chunk, *val, proto, desc),
			Builtin::Type { val } => check_reach_expr(syms, sources, chunk, *val, proto, desc),
		},
		Expr::Lit(lit) => match lit {
			Lit::List(items) => {
				for item in items {
					check_reach_expr(syms, sources, chunk, *item, proto, desc)?;
				}
				Ok(())
			}
			Lit::Dict(pairs) => {
				for (key, val) in pairs {
					check_reach_expr(syms, sources, chunk, *key, proto, desc)?;
					check_reach_expr(syms, sources, chunk, *val, proto, desc)?;
				}
				Ok(())
			}
			Lit::Str(_) | Lit::Char(_) | Lit::Num(_) | Lit::Bool(_) | Lit::Nil => Ok(()),
		},
	}
}

fn check_reaches_member(
	syms: &Interner,
	sources: &Sources,
	chunk: &Chunk,
	expr_id: ExprId,
	proto: Sym,
	desc: &types::Proto,
	member: &Member,
) -> Result<(), Error> {
	if let Expr::Self_ = chunk.get_expr(member.receiver) {
		if !desc.required.contains_key(&member.name) && !desc.provided.contains_key(&member.name) {
			let span = chunk.get_expr_span(expr_id);
			return Err(Error::ProtocolReach(
				sources.loc(span),
				syms.resolve(proto).to_string(),
				syms.resolve(member.name).to_string(),
			));
		}
	}
	Ok(())
}

fn check_def(
	syms: &Interner,
	sources: &Sources,
	chunk: &Chunk,
	span: Span,
	def: &Def,
) -> Result<(), Error> {
	check_required_precede_optional(syms, sources, span, &def.params)?;
	// A proc body resets the loop-depth counter. break inside a proc can't
	// reach an outer loop, even if the proc itself is lexically nested in one.
	check_block(sources, chunk, def.body, 0)
}

fn check_block(
	sources: &Sources,
	chunk: &Chunk,
	block_id: BlockId,
	depth: u32,
) -> Result<(), Error> {
	let block = chunk.get_block(block_id);
	for expr_id in &block.exprs {
		check_expr(sources, chunk, *expr_id, depth)?;
	}
	Ok(())
}

fn check_expr(sources: &Sources, chunk: &Chunk, expr_id: ExprId, depth: u32) -> Result<(), Error> {
	match chunk.get_expr(expr_id) {
		Expr::Each(each) => {
			check_expr(sources, chunk, each.iter, depth)?;
			check_block(sources, chunk, each.body, depth + 1)
		}
		Expr::Loop(loop_) => check_block(sources, chunk, loop_.body, depth + 1),
		Expr::When(when) => {
			check_expr(sources, chunk, when.cond, depth)?;
			check_block(sources, chunk, when.then_branch, depth)?;
			if let Some(else_branch) = when.else_branch {
				check_block(sources, chunk, else_branch, depth)?;
			}
			Ok(())
		}
		Expr::Match(match_) => {
			check_expr(sources, chunk, match_.scrutinee, depth)?;
			for arm in &match_.arms {
				check_expr(sources, chunk, arm.path, depth)?;
				check_block(sources, chunk, arm.body, depth)?;
			}
			if let Some(else_branch) = match_.else_branch {
				check_block(sources, chunk, else_branch, depth)?;
			}
			Ok(())
		}
		Expr::Do(do_) => {
			check_block(sources, chunk, do_.body, depth)?;
			for arm in &do_.arms {
				check_expr(sources, chunk, arm.path, depth)?;
				check_block(sources, chunk, arm.body, depth)?;
			}
			if let Some(else_branch) = do_.else_branch {
				check_block(sources, chunk, else_branch, depth)?;
			}
			Ok(())
		}
		Expr::Return(return_) => match return_.val {
			Some(val) => check_expr(sources, chunk, val, depth),
			None => Ok(()),
		},
		Expr::Break(break_) => {
			if depth == 0 {
				let span = chunk.get_expr_span(expr_id);
				return Err(Error::BreakOutsideLoop(sources.loc(span)));
			}
			match break_.val {
				Some(val) => check_expr(sources, chunk, val, depth),
				None => Ok(()),
			}
		}
		Expr::Raise(raise) => check_expr(sources, chunk, raise.val, depth),
		Expr::Self_ => Ok(()),
		Expr::Call(call) => {
			check_expr(sources, chunk, call.callee, depth)?;
			for arg in &call.args {
				check_expr(sources, chunk, arg.val, depth)?;
			}
			Ok(())
		}
		Expr::Member(member) => check_expr(sources, chunk, member.receiver, depth),
		Expr::Access(access) => {
			check_expr(sources, chunk, access.receiver, depth)?;
			check_expr(sources, chunk, access.key, depth)
		}
		Expr::Mention(mention) => check_expr(sources, chunk, mention.val, depth),
		Expr::Assign(assign) => {
			match &assign.place {
				Place::Name(_) => {}
				Place::Member(member) => check_expr(sources, chunk, member.receiver, depth)?,
				Place::Access(access) => {
					check_expr(sources, chunk, access.receiver, depth)?;
					check_expr(sources, chunk, access.key, depth)?;
				}
			}
			check_expr(sources, chunk, assign.val, depth)
		}
		Expr::Binary(binary) => {
			check_expr(sources, chunk, binary.lhs, depth)?;
			check_expr(sources, chunk, binary.rhs, depth)
		}
		Expr::Unary(unary) => check_expr(sources, chunk, unary.val, depth),
		Expr::Name(_) => Ok(()),
		Expr::Builtin(builtin) => match builtin {
			Builtin::Print { val } => check_expr(sources, chunk, *val, depth),
			Builtin::Type { val } => check_expr(sources, chunk, *val, depth),
		},
		Expr::Lit(lit) => match lit {
			Lit::List(items) => {
				for item in items {
					check_expr(sources, chunk, *item, depth)?;
				}
				Ok(())
			}
			Lit::Dict(pairs) => {
				for (key, val) in pairs {
					check_expr(sources, chunk, *key, depth)?;
					check_expr(sources, chunk, *val, depth)?;
				}
				Ok(())
			}
			Lit::Str(_) | Lit::Char(_) | Lit::Num(_) | Lit::Bool(_) | Lit::Nil => Ok(()),
		},
	}
}

// Ensure that required params precede optional ones. This is a load-bearing
// invariant for virtually all arg/param handling.
pub fn check_required_precede_optional(
	syms: &Interner,
	sources: &Sources,
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
			let loc = sources.loc(span);
			return Err(Error::RequiredAfterDefault(
				loc,
				syms.resolve(param.name).to_string(),
				syms.resolve(earlier).to_string(),
			));
		}
	}
	Ok(())
}
