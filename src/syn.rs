pub mod lex;
pub mod nodes;
pub mod parse;

use std::fmt::{self, Display, Formatter};

use crate::src::Location;
use crate::syn::lex::TokenTag;

#[derive(Debug)]
pub enum Error {
	UnexpectedChar(Location, char),
	UnexpectedToken(Location, TokenTag),
	UnterminatedStrLit(Location),
	UnterminatedCharLit(Location),
	MultiCharLit(Location),
	UnknownEsc(Location, char),
	UnknownBuiltin(Location, String),
	PositionalAfterKeyword(Location),
}

impl Error {
	fn loc(&self) -> &Location {
		match self {
			Self::UnexpectedChar(loc, _) => loc,
			Self::UnexpectedToken(loc, _) => loc,
			Self::UnterminatedStrLit(loc) => loc,
			Self::UnterminatedCharLit(loc) => loc,
			Self::MultiCharLit(loc) => loc,
			Self::UnknownEsc(loc, _) => loc,
			Self::UnknownBuiltin(loc, _) => loc,
			Self::PositionalAfterKeyword(loc) => loc,
		}
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		write!(f, "{}: syntax error: ", self.loc())?;
		match self {
			Self::UnexpectedChar(_, char) => write!(f, "unexpected character '{}'", char),
			Self::UnexpectedToken(_, tag) => write!(f, "unexpected token {}", tag.name()),
			Self::UnterminatedStrLit(_) => write!(f, "unterminated string literal"),
			Self::UnterminatedCharLit(_) => write!(f, "unterminated character literal"),
			Self::MultiCharLit(_) => write!(f, "character literal must hold exactly one character"),
			Self::UnknownEsc(_, esc) => write!(f, "unknown escape sequence '\\{}'", esc),
			Self::UnknownBuiltin(_, builtin) => write!(f, "unknown builtin '{}'", builtin),
			Self::PositionalAfterKeyword(_) => {
				write!(f, "positional argument after keyword argument")
			}
		}
	}
}
