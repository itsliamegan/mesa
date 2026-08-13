use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};

use crate::intern::{Interner, Sym};
use crate::rt::CORE_TYPE_NAMES;
use crate::syn::{Chunk, Def, Location, Method, ModuleItem, Param, Source, Span, Type, TypeItem};

#[derive(Debug)]
pub enum Error {
	PreludeShadowed(Location, String),
	RequiredAfterDefault(Location, String, String),
}

impl Error {
	pub fn loc(&self) -> &Location {
		match self {
			Self::PreludeShadowed(loc, _) => loc,
			Self::RequiredAfterDefault(loc, ..) => loc,
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
			ModuleItem::Type(Type(name, fields, items)) => {
				ensure_required_precede_defaults(syms, src, span, fields)?;
				for item_id in items {
					let span = chunk.get_type_item_span(*item_id);
					let params = match chunk.get_type_item(*item_id) {
						TypeItem::Method(Method::Instance(Def(_, params, _))) => params,
						TypeItem::Method(Method::Static(Def(_, params, _))) => params,
						TypeItem::Field(_) => continue,
					};
					ensure_required_precede_defaults(syms, src, span, params)?;
				}
				*name
			}
			ModuleItem::Def(Def(name, params, _)) => {
				ensure_required_precede_defaults(syms, src, span, params)?;
				*name
			}
			ModuleItem::Expr(_) => continue,
		};
		if prelude.contains(&name) {
			let loc = src.loc(span.start);
			return Err(Error::PreludeShadowed(loc, syms.resolve(name).to_string()));
		}
	}
	Ok(())
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
