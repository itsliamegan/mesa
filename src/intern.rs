use std::collections::HashMap;

use rustc_hash::{FxBuildHasher, FxHashMap};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Sym(u32);

impl Sym {
	pub const TYPE: Sym = Sym(0);
	pub const DEF: Sym = Sym(1);
	pub const EACH: Sym = Sym(2);
	pub const IN: Sym = Sym(3);
	pub const DO: Sym = Sym(4);
	pub const WHEN: Sym = Sym(5);
	pub const THEN: Sym = Sym(6);
	pub const ELSE: Sym = Sym(7);
	pub const END: Sym = Sym(8);
	pub const RETURN: Sym = Sym(9);
	pub const AND: Sym = Sym(10);
	pub const OR: Sym = Sym(11);
	pub const NOT: Sym = Sym(12);
	pub const SELF: Sym = Sym(13);
	pub const TRUE: Sym = Sym(14);
	pub const FALSE: Sym = Sym(15);
	pub const NIL: Sym = Sym(16);
}

const KEYWORDS: &[(&str, Sym)] = &[
	("type", Sym::TYPE),
	("def", Sym::DEF),
	("each", Sym::EACH),
	("in", Sym::IN),
	("do", Sym::DO),
	("when", Sym::WHEN),
	("then", Sym::THEN),
	("else", Sym::ELSE),
	("end", Sym::END),
	("return", Sym::RETURN),
	("and", Sym::AND),
	("or", Sym::OR),
	("not", Sym::NOT),
	("self", Sym::SELF),
	("true", Sym::TRUE),
	("false", Sym::FALSE),
	("nil", Sym::NIL),
];

pub struct Interner {
	syms: FxHashMap<&'static str, Sym>,
	names: Vec<&'static str>,
}

impl Interner {
	pub fn new() -> Self {
		let mut syms = HashMap::with_capacity_and_hasher(KEYWORDS.len(), FxBuildHasher);
		let mut names = Vec::new();

		for (name, sym) in KEYWORDS {
			syms.insert(*name, *sym);
			names.push(*name);
		}

		Self { syms, names }
	}

	pub fn intern(&mut self, name: &str) -> Sym {
		if let Some(sym) = self.syms.get(name) {
			return *sym;
		}

		let name = Box::leak(Box::from(name));
		let sym = Sym(self.names.len() as u32);
		self.syms.insert(name, sym);
		self.names.push(name);
		sym
	}

	pub fn resolve(&self, sym: Sym) -> &str {
		self.names[sym.0 as usize]
	}
}
