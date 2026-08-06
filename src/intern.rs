use std::collections::HashMap;

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
	pub const TRUE: Sym = Sym(10);
	pub const FALSE: Sym = Sym(11);
	pub const NIL: Sym = Sym(12);
}

const KEYWORDS: &[(&'static str, Sym)] = &[
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
	("true", Sym::TRUE),
	("false", Sym::FALSE),
	("nil", Sym::NIL),
];

pub struct Interner {
	syms: HashMap<&'static str, Sym>,
	names: Vec<&'static str>,
}

impl Interner {
	pub fn new() -> Self {
		let mut syms = HashMap::new();
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
