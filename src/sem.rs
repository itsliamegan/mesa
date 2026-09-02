pub mod modules;
pub mod protos;
pub mod types;

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display, Formatter};

use crate::intern::{Interner, Sym};
use crate::pkg::{PackageId, Packages};
use crate::sem::modules::ModuleId;
use crate::src::{Location, Sources, Span};
use crate::syn::Chunk;
use crate::syn::nodes::{
	BlockId, Builtin, DefId, Expr, ExprId, Lit, Member, ModuleItem, Param, Place,
};

#[derive(Debug)]
pub enum Error {
	RequiredAfterDefault(Location, String, String),
	DuplicateParam(Location, String),
	BreakOutsideLoop(Location),
	ParamsOnCaseParent(Location, String),
	FieldOnCaseParent(Location, String, String),
	ProvidedWithoutRequired(Location, String),
	UnknownProtocol(Location, String),
	NotAProtocol(Location, String),
	MissingMethod(Location, String, String, String),
	SignatureMismatch(Location, String, String, String),
	ProtocolConflict(Location, String, String, String),
	DuplicateImpl(Location, String),
	MethodCollision(Location, String, String, String),
	DuplicateTypeMember(Location, String, String),
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
	ProtocolCycle(Location, String),
}

impl Error {
	pub fn loc(&self) -> &Location {
		match self {
			Self::RequiredAfterDefault(loc, ..) => loc,
			Self::DuplicateParam(loc, _) => loc,
			Self::BreakOutsideLoop(loc) => loc,
			Self::ParamsOnCaseParent(loc, _) => loc,
			Self::FieldOnCaseParent(loc, ..) => loc,
			Self::ProvidedWithoutRequired(loc, _) => loc,
			Self::UnknownProtocol(loc, _) => loc,
			Self::NotAProtocol(loc, _) => loc,
			Self::MissingMethod(loc, ..) => loc,
			Self::SignatureMismatch(loc, ..) => loc,
			Self::ProtocolConflict(loc, ..) => loc,
			Self::DuplicateImpl(loc, _) => loc,
			Self::MethodCollision(loc, ..) => loc,
			Self::DuplicateTypeMember(loc, ..) => loc,
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
			Self::ProtocolCycle(loc, _) => loc,
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
			Self::DuplicateParam(_, name) => {
				write!(f, "duplicate param '{}'", name)
			}
			Self::BreakOutsideLoop(_) => write!(f, "'break' outside a loop"),
			Self::ParamsOnCaseParent(_, name) => {
				write!(f, "case type '{}' cannot declare params", name)
			}
			Self::FieldOnCaseParent(_, name, field) => {
				write!(f, "case type '{}' cannot declare field '{}'", name, field)
			}
			Self::ProvidedWithoutRequired(_, proto) => {
				write!(f, "protocol '{}' provides methods but requires none", proto)
			}
			Self::UnknownProtocol(_, proto) => write!(f, "unknown protocol '{}'", proto),
			Self::NotAProtocol(_, name) => write!(f, "'{}' is not a protocol", name),
			Self::MissingMethod(_, name, proto, member) => {
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
			Self::DuplicateImpl(_, name) => write!(f, "protocol '{}' is named twice", name),
			Self::MethodCollision(_, name, proto, member) => {
				write!(
					f,
					"method '{}' provided by protocol '{}' collides with a member of type '{}'",
					member, proto, name
				)
			}
			Self::DuplicateTypeMember(_, name, member) => {
				write!(
					f,
					"type '{}' declares member '{}' more than once",
					name, member
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
			Self::ProtocolCycle(_, cycle) => write!(f, "protocol cycle: {}", cycle),
		}
	}
}

// The possible flags for a node in a cycle-detection walk.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Visit {
	// Unseen, never entered.
	Unseen,
	// Entered and still on the path below this one.
	OnPath,
	// Entered and left.
	Done,
}

pub fn check(
	syms: &mut Interner,
	pkgs: &mut Packages,
	pkg_id: PackageId,
	natives: &HashMap<Sym, types::NativeTypeShape>,
) -> Result<(), Vec<Error>> {
	let modules = modules::check(syms, pkgs, pkg_id)?;
	pkgs.publish_modules(pkg_id, modules);

	let (protos, mut errs) = protos::check(syms, pkgs, pkg_id);
	pkgs.publish_protos(pkg_id, protos);

	// Describing resolves 'impl' names and decides conformance itself, so it
	// tolerates the package not yet being known valid; it runs before the
	// chunk walk rather than after, and its errors precede that walk's in the
	// bundle, mirroring how module errors already precede every chunk error.
	let (types, mut type_errs) = types::check(syms, pkgs, pkg_id, natives);
	errs.append(&mut type_errs);
	pkgs.publish_types(pkg_id, types);

	// Every chunk is a module by now, module checking having failed otherwise,
	// and the modules are held in chunk order, so this reports in file order.
	let pkg = pkgs.get(pkg_id);
	for id in pkg.modules().ids() {
		if let Err(mut chunk_errs) = check_chunk(syms, pkgs, pkg_id, id) {
			errs.append(&mut chunk_errs);
		}
	}

	if !errs.is_empty() {
		return Err(errs);
	}

	Ok(())
}

fn check_chunk(
	syms: &Interner,
	pkgs: &Packages,
	pkg_id: PackageId,
	module: ModuleId,
) -> Result<(), Vec<Error>> {
	let pkg = pkgs.get(pkg_id);
	let chunk_id = pkg.modules().chunk(module);
	let chunk = pkg.chunks().get(chunk_id);
	let mut errs = Vec::new();

	// Every def in the chunk, module-level, method, or protocol item, in one
	// walk. A proto item additionally checks that its provided body only
	// reaches the implementing instance through its own protocol's declared
	// members; a def belongs to no protocol unless the lookup below says so.
	for def_id in chunk.def_ids() {
		if let Err(err) = check_def(syms, pkg.sources(), chunk, def_id) {
			errs.push(err);
			continue;
		}
		if let Some(proto_id) = pkg.protos().try_get_proto_by_proto_item(chunk_id, def_id) {
			let def = chunk.get_def(def_id);
			let desc = pkg.protos().get_proto(proto_id);
			if desc.provided.contains_key(&def.name)
				&& let Err(err) =
					check_reach_block(syms, pkg.sources(), chunk, def.body, desc.name, desc)
			{
				errs.push(err);
			}
		}
	}

	for item_id in &chunk.top {
		if let ModuleItem::Expr(expr_id) = chunk.get_module_item(*item_id)
			&& let Err(err) = check_expr(pkg.sources(), chunk, *expr_id, 0)
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
	desc: &protos::Proto,
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
	desc: &protos::Proto,
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
	desc: &protos::Proto,
	member: &Member,
) -> Result<(), Error> {
	if let Expr::Self_ = chunk.get_expr(member.receiver) {
		if !desc.effective.contains(&member.name) {
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
	def_id: DefId,
) -> Result<(), Error> {
	let def = chunk.get_def(def_id);
	check_params(syms, sources, chunk.get_def_span(def_id), &def.params)?;
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

// Every invariant a param list must satisfy, wherever one is declared.
pub fn check_params(
	syms: &Interner,
	sources: &Sources,
	span: Span,
	params: &[Param],
) -> Result<(), Error> {
	check_params_are_distinct(syms, sources, span, params)?;
	check_required_precede_optional(syms, sources, span, params)
}

// Ensure that no two params share a name. A later param of the same name would
// silently win at every call site, and a caller naming it as a keyword arg has
// no way to reach the earlier one.
fn check_params_are_distinct(
	syms: &Interner,
	sources: &Sources,
	span: Span,
	params: &[Param],
) -> Result<(), Error> {
	let mut seen = HashSet::new();
	for param in params {
		if !seen.insert(param.name) {
			return Err(Error::DuplicateParam(
				sources.loc(span),
				syms.resolve(param.name).to_string(),
			));
		}
	}
	Ok(())
}

// Ensure that required params precede optional ones. This is a load-bearing
// invariant for virtually all arg/param handling.
fn check_required_precede_optional(
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
