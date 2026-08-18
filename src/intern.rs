use std::collections::HashMap;

use rustc_hash::{FxBuildHasher, FxHashMap};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Sym(u32);

impl Sym {
	pub const MODULE: Sym = Sym(0);
	pub const IMPORT: Sym = Sym(1);
	pub const EXPORT: Sym = Sym(2);
	pub const TYPE: Sym = Sym(3);
	pub const CASE: Sym = Sym(4);
	pub const PROTO: Sym = Sym(5);
	pub const IMPL: Sym = Sym(6);
	pub const DEF: Sym = Sym(7);
	pub const EACH: Sym = Sym(8);
	pub const LOOP: Sym = Sym(9);
	pub const DO: Sym = Sym(10);
	pub const IN: Sym = Sym(11);
	pub const WHEN: Sym = Sym(12);
	pub const THEN: Sym = Sym(13);
	pub const ELSE: Sym = Sym(14);
	pub const END: Sym = Sym(15);
	pub const RETURN: Sym = Sym(16);
	pub const BREAK: Sym = Sym(17);
	pub const AND: Sym = Sym(18);
	pub const OR: Sym = Sym(19);
	pub const NOT: Sym = Sym(20);
	pub const SELF: Sym = Sym(21);
	pub const TRUE: Sym = Sym(22);
	pub const FALSE: Sym = Sym(23);
	pub const NIL: Sym = Sym(24);
}

const KEYWORDS: &[(&str, Sym)] = &[
	("module", Sym::MODULE),
	("import", Sym::IMPORT),
	("export", Sym::EXPORT),
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

pub const CORE_TYPE_NAMES: &[&str] = &[
	"Nil", "Num", "Bool", "Char", "Str", "List", "Dict", "Proc", "Type", "Proto", "Module",
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

	// Resolve a single symbol to its string name.
	pub fn resolve(&self, sym: Sym) -> &str {
		self.names[sym.0 as usize]
	}

	// Resolve a path of symbols representing nested lookup to a dotted path
	// string (e.g. "JSON.Codec.encode").
	pub fn resolve_path(&self, path: &[Sym]) -> String {
		path.iter()
			.map(|sym| self.resolve(*sym))
			.collect::<Vec<_>>()
			.join(".")
	}
}
