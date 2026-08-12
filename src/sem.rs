use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};

use crate::intern::Interner;
use crate::rt::CORE_TYPE_NAMES;
use crate::syn::{Chunk, Def, Location, ModuleItem, Source, Type};

#[derive(Debug)]
pub enum Error {
	PreludeShadowed(Location, String),
}

impl Error {
	pub fn loc(&self) -> &Location {
		match self {
			Self::PreludeShadowed(loc, _) => loc,
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
		}
	}
}

pub fn check(syms: &mut Interner, chunk: &Chunk, src: &Source) -> Result<(), Error> {
	let mut prelude = HashSet::new();
	for name in CORE_TYPE_NAMES {
		prelude.insert(syms.intern(name));
	}
	for item_id in &chunk.top {
		let name = match chunk.get_module_item(*item_id) {
			ModuleItem::Type(Type(name, ..)) => *name,
			ModuleItem::Def(Def(name, ..)) => *name,
			ModuleItem::Expr(_) => continue,
		};
		if prelude.contains(&name) {
			let span = chunk.get_module_item_span(*item_id);
			let loc = src.loc(span.start);
			return Err(Error::PreludeShadowed(loc, syms.resolve(name).to_string()));
		}
	}
	Ok(())
}
