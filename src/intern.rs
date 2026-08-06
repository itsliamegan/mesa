use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Sym(u32);

pub struct Interner {
	syms: HashMap<&'static str, Sym>,
	names: Vec<&'static str>,
}

impl Interner {
	pub fn new() -> Self {
		Self {
			syms: HashMap::new(),
			names: Vec::new(),
		}
	}

	pub fn intern(&mut self, name: &str) -> Sym {
		if let Some(sym) = self.syms.get(name) {
			return *sym;
		}

		let leaked = Box::leak(Box::from(name));
		let sym = Sym(self.names.len() as u32);
		self.syms.insert(leaked, sym);
		self.names.push(leaked);
		sym
	}

	pub fn resolve(&self, sym: Sym) -> &str {
		self.names[sym.0 as usize]
	}
}
