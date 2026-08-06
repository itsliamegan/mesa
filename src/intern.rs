use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SymId(usize);

#[derive(Debug)]
pub struct Sym(pub SymId, pub String);

pub struct Interner {
	syms: Vec<Sym>,
	ids: HashMap<String, SymId>,
}

impl Interner {
	pub fn new() -> Self {
		Self {
			syms: Vec::new(),
			ids: HashMap::new(),
		}
	}

	pub fn get_or_add(&mut self, name: &str) -> SymId {
		match self.get_by_name(name) {
			Some(Sym(id, _)) => *id,
			None => {
				let id = SymId(self.syms.len());
				let sym = Sym(id, String::from(name));
				self.syms.push(sym);
				self.ids.insert(String::from(name), id);
				id
			}
		}
	}

	pub fn get_by_name(&self, name: &str) -> Option<&Sym> {
		match self.ids.get(name) {
			Some(id) => Some(&self.syms[id.0]),
			None => None,
		}
	}

	pub fn get_by_id(&self, id: SymId) -> &Sym {
		&self.syms[id.0]
	}
}
