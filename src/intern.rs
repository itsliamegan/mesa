use std::collections::HashMap;

use rustc_hash::{FxBuildHasher, FxHashMap};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Sym(u32);

impl Sym {
	pub const TYPE: Sym = Sym(0);
	pub const CASE: Sym = Sym(1);
	pub const PROTO: Sym = Sym(2);
	pub const IMPL: Sym = Sym(3);
	pub const DEF: Sym = Sym(4);
	pub const EACH: Sym = Sym(5);
	pub const LOOP: Sym = Sym(6);
	pub const DO: Sym = Sym(7);
	pub const IN: Sym = Sym(8);
	pub const WHEN: Sym = Sym(9);
	pub const THEN: Sym = Sym(10);
	pub const ELSE: Sym = Sym(11);
	pub const END: Sym = Sym(12);
	pub const RETURN: Sym = Sym(13);
	pub const BREAK: Sym = Sym(14);
	pub const AND: Sym = Sym(15);
	pub const OR: Sym = Sym(16);
	pub const NOT: Sym = Sym(17);
	pub const SELF: Sym = Sym(18);
	pub const TRUE: Sym = Sym(19);
	pub const FALSE: Sym = Sym(20);
	pub const NIL: Sym = Sym(21);
}

const KEYWORDS: &[(&str, Sym)] = &[
	("type", Sym::TYPE),
	("case", Sym::CASE),
	("proto", Sym::PROTO),
	("impl", Sym::IMPL),
	("def", Sym::DEF),
	("each", Sym::EACH),
	("loop", Sym::LOOP),
	("do", Sym::DO),
	("in", Sym::IN),
	("when", Sym::WHEN),
	("then", Sym::THEN),
	("else", Sym::ELSE),
	("end", Sym::END),
	("return", Sym::RETURN),
	("break", Sym::BREAK),
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
