mod eval;
mod modules;
mod scope;
mod types;
mod val;

use std::fmt::{self, Display, Formatter};

pub use eval::Interpreter;

#[derive(Debug)]
pub enum Error {
	ProtocolError(ProtocolError),
	ArgumentError(ArgumentError),
	TypeError(TypeError),
	MemberError(MemberError),
	IndexError(IndexError),
	NameError(String),
	KeyError(String),
}

#[derive(Debug)]
pub enum ProtocolError {
	NotIterable(String),
	NotAccessible(String),
	NotAppendable(String),
	NotOrderable(String),
}

#[derive(Debug)]
pub enum ArgumentError {
	Missing(Vec<String>),
	TooMany(usize, usize),
	Unknown(String),
	Duplicate(String),
}

#[derive(Debug)]
pub enum TypeError {
	IndexNonNum(String),
	ArithNonNum(String),
	ConcatNonStr(String),
	NotCallable(String),
	NotConstructible(String),
	NotInvokable(String),
	CaseNonType(String),
}

#[derive(Debug)]
pub enum MemberError {
	Missing(String, String),
	ReadOnly(String, String),
}

#[derive(Debug)]
pub enum IndexError {
	OutOfRange(f64),
	NonIntegral(f64),
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::ProtocolError(err) => Display::fmt(err, f),
			Self::ArgumentError(err) => Display::fmt(err, f),
			Self::TypeError(err) => Display::fmt(err, f),
			Self::MemberError(err) => Display::fmt(err, f),
			Self::IndexError(err) => Display::fmt(err, f),
			Self::NameError(name) => write!(f, "name '{}' is not defined", name),
			Self::KeyError(key) => write!(f, "key {} not found", key),
		}
	}
}

impl Display for ProtocolError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::NotIterable(type_name) => write!(f, "type {} is not iterable", type_name),
			Self::NotAccessible(type_name) => write!(f, "type {} is not accessible", type_name),
			Self::NotAppendable(type_name) => write!(f, "type {} is not appendable", type_name),
			Self::NotOrderable(type_name) => write!(f, "type {} is not orderable", type_name),
		}
	}
}

impl Display for ArgumentError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::Missing(names) => {
				let noun = if names.len() == 1 { "arg" } else { "args" };
				let names = names
					.iter()
					.map(|name| format!("'{}'", name))
					.collect::<Vec<_>>()
					.join(", ");
				write!(f, "missing {} {}", noun, names)
			}
			Self::TooMany(have, want) => {
				write!(f, "too many args; have {}, want at most {}", have, want)
			}
			Self::Unknown(name) => write!(f, "no param named '{}'", name),
			Self::Duplicate(name) => write!(f, "arg '{}' given twice", name),
		}
	}
}

impl Display for TypeError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::IndexNonNum(val) => write!(f, "index {} is not a number", val),
			Self::ArithNonNum(type_name) => {
				write!(f, "type {} cannot be used in arithmetic", type_name)
			}
			Self::ConcatNonStr(type_name) => write!(f, "type {} cannot be concatenated", type_name),
			Self::NotCallable(type_name) => write!(f, "type {} is not callable", type_name),
			Self::NotConstructible(type_name) => {
				write!(f, "type {} cannot be constructed", type_name)
			}
			Self::NotInvokable(type_name) => write!(f, "type {} is not invokable", type_name),
			Self::CaseNonType(type_name) => {
				write!(f, "type {} cannot be matched against", type_name)
			}
		}
	}
}

impl Display for MemberError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::Missing(namespace, name) => {
				write!(f, "{} has no such member '{}'", namespace, name)
			}
			Self::ReadOnly(namespace, name) => {
				write!(f, "member '{}' on {} is read-only", name, namespace)
			}
		}
	}
}

impl Display for IndexError {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self {
			Self::OutOfRange(idx) => write!(f, "index {} is out of bounds", idx),
			Self::NonIntegral(idx) => write!(f, "index {} is not a whole number", idx),
		}
	}
}
