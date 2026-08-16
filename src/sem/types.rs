use std::collections::HashMap;

use ordermap::OrderMap;

use crate::intern::Sym;
use crate::pkg::Package;
use crate::sem::is_provided;
use crate::sem::modules::{Binding, Member, ModuleId, Modules};
use crate::syn::{
	self, Chunk, ChunkId, ExprId, ModuleItem, ModuleItemId, Param, ProtoItemId, TypeItem,
	TypeItemId,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct TypeId(u32);

impl TypeId {
	fn index(self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ProtoId(u32);

impl ProtoId {
	fn index(self) -> usize {
		self.0 as usize
	}
}

// Every type and protocol that a package declares.
#[derive(Debug)]
pub struct Types {
	types: Vec<Type>,
	protos: Vec<Proto>,
	// Where a protocol's declaration lives, so an 'impl' name resolved through
	// the module map arrives at the description built for it.
	proto_by_item: HashMap<(ChunkId, ModuleItemId), ProtoId>,
}

#[derive(Debug)]
pub struct Type {
	pub name: Sym,
	pub chunk: ChunkId,
	pub ctor_fields: Vec<Param>,
	pub body_fields: OrderMap<Sym, ExprId>,
	// Every member reachable on an instance, flattened. Includes what this type
	// declares, what it inherits as a case variant, and what it acquires from
	// the protocols it implements.
	pub members: HashMap<Sym, MemberSite>,
	pub statics: HashMap<Sym, Static>,
	pub impls: Vec<ProtoId>,
	// Type this type was declared in; None at the module level.
	pub enclosing: Option<TypeId>,
	// Variants of this type; None if this type is itself a variant.
	pub variants: Option<Vec<TypeId>>,
}

#[derive(Debug)]
pub struct Proto {
	pub name: Sym,
	pub chunk: ChunkId,
	pub members: HashMap<Sym, ProtoItemId>,
}

// The declaration implementing a member. Includes a reference to the
// implementer's chunk, which may differ for e.g. an acquired protocol method.
// Differentiates between a declared method and a provided method because a
// provided method lives in a different arena.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MemberSite {
	Declared(ChunkId, TypeItemId),
	Provided(ChunkId, ProtoItemId),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Static {
	Type(TypeId),
	Proc(ChunkId, TypeItemId),
}

// What a case variant takes from the type enclosing it. The two halves must be
// kept separate because they have different precedence; a parent's declared
// method overrides a member the variant acquires from a protocol it implements.
#[derive(Debug)]
struct Inherited {
	declared: HashMap<Sym, MemberSite>,
	acquired: HashMap<Sym, MemberSite>,
}

impl Types {
	pub fn get_type(&self, id: TypeId) -> &Type {
		&self.types[id.index()]
	}

	pub fn get_proto(&self, id: ProtoId) -> &Proto {
		&self.protos[id.index()]
	}

	fn add_proto(&mut self, chunk: ChunkId, item_id: ModuleItemId, proto: Proto) -> ProtoId {
		let id = ProtoId(self.protos.len() as u32);
		self.protos.push(proto);
		self.proto_by_item.insert((chunk, item_id), id);
		id
	}
}

// Describe every type and protocol in the package. This must run *after*
// checking has validated all invariants about conformance, 'impl' resolution,
// etc.
pub fn describe(pkg: &Package, mods: &Modules) -> Types {
	let mut types = Types {
		types: Vec::new(),
		protos: Vec::new(),
		proto_by_item: HashMap::new(),
	};

	// Describe protocols first, package-wide, so a type can implement one
	// declared in any file.
	for module in mods.ids() {
		let chunk_id = mods.chunk(module);
		let chunk = pkg.get_chunk(chunk_id);
		for item_id in &chunk.top {
			if let ModuleItem::Proto(proto) = chunk.get_module_item(*item_id) {
				let desc = describe_proto(chunk, chunk_id, proto);
				types.add_proto(chunk_id, *item_id, desc);
			}
		}
	}

	for module in mods.ids() {
		let chunk_id = mods.chunk(module);
		let chunk = pkg.get_chunk(chunk_id);
		for item_id in &chunk.top {
			if let ModuleItem::Type(type_) = chunk.get_module_item(*item_id) {
				describe_type(
					&mut types, pkg, mods, module, chunk_id, type_, None, None, false,
				);
			}
		}
	}

	types
}

// Describe a protocol declaration, producing a description which simply names
// its members, both required and provided.
fn describe_proto(chunk: &Chunk, chunk_id: ChunkId, proto: &syn::Proto) -> Proto {
	let mut members = HashMap::new();
	for item_id in &proto.items {
		let item = chunk.get_proto_item(*item_id);
		members.insert(item.def.name, *item_id);
	}
	Proto {
		name: proto.name,
		chunk: chunk_id,
		members,
	}
}

// Describe a type declaration, producing a description that reduces all later
// member resolution to a single table lookup.
fn describe_type(
	types: &mut Types,
	pkg: &Package,
	mods: &Modules,
	module: ModuleId,
	chunk_id: ChunkId,
	type_: &syn::Type,
	inherited: Option<&Inherited>,
	enclosing: Option<TypeId>,
	is_case: bool,
) -> TypeId {
	let chunk = pkg.get_chunk(chunk_id);
	// Claim the id before the body is walked so that a nested type can name the
	// type enclosing it while that type is still being described.
	let id = TypeId(types.types.len() as u32);
	types.types.push(Type {
		name: type_.name,
		chunk: chunk_id,
		ctor_fields: type_.params.to_vec(),
		body_fields: OrderMap::new(),
		members: HashMap::new(),
		statics: HashMap::new(),
		impls: Vec::new(),
		enclosing,
		variants: None,
	});

	let mut body_fields = OrderMap::new();
	let mut statics = HashMap::new();
	let mut declared = HashMap::new();
	for item_id in &type_.items {
		match chunk.get_type_item(*item_id) {
			// Ignore cases and inner types here because they are described below.
			//
			// A variant's members *must* be resolved after their parent's
			// members because they inherit some members from their parent.
			//
			// An inner type inherits nothing but describing here would
			// interleave unrelated work.
			TypeItem::Case(..) => {}
			TypeItem::Field(field) => {
				body_fields.insert(field.name, field.init);
			}
			TypeItem::Type(..) => {}
			TypeItem::Method(syn::Method::Instance(def)) => {
				declared.insert(def.name, MemberSite::Declared(chunk_id, *item_id));
			}
			TypeItem::Method(syn::Method::Static(def)) => {
				statics.insert(def.name, Static::Proc(chunk_id, *item_id));
			}
		}
	}

	let impls = resolve_impls(types, mods, module, type_);
	let acquired = acquired_members(types, pkg, &impls);

	// Statically compute the members by overwriting in the inverse order of
	// their precedence.
	//
	//   own declared > parent declared > own acquired > parent acquired
	//
	// Declared beats acquired at both levels, which is what lets a type
	// override a protocol. Notably, a parent can declare a method to override a
	// method that a variant acquires from a protocol it implements.
	let mut members = HashMap::new();
	if let Some(inherited) = inherited {
		members.extend(&inherited.acquired);
	}
	members.extend(&acquired);
	if let Some(inherited) = inherited {
		members.extend(&inherited.declared);
	}
	members.extend(&declared);

	let own = Inherited {
		declared: declared.clone(),
		acquired,
	};
	let mut variants = Vec::new();
	for item_id in &type_.items {
		match chunk.get_type_item(*item_id) {
			TypeItem::Case(variant) => {
				let variant_id = describe_type(
					types,
					pkg,
					mods,
					module,
					chunk_id,
					variant,
					Some(&own),
					Some(id),
					true,
				);
				statics.insert(variant.name, Static::Type(variant_id));
				variants.push(variant_id);
			}
			TypeItem::Field(..) => {}
			// An inner type is enclosed but not a variant, so it inherits
			// nothing: only a case refines the type it is written in.
			TypeItem::Type(inner) => {
				let inner_id = describe_type(
					types,
					pkg,
					mods,
					module,
					chunk_id,
					inner,
					None,
					Some(id),
					false,
				);
				statics.insert(inner.name, Static::Type(inner_id));
			}
			TypeItem::Method(..) => {}
		}
	}

	let described = &mut types.types[id.index()];
	described.body_fields = body_fields;
	described.members = members;
	described.statics = statics;
	described.impls = impls;
	described.variants = match is_case {
		true => None,
		false => Some(variants),
	};
	id
}

// Resolve the protocols an 'impl' line names against the symbols that the
// declaring file binds.
fn resolve_impls(
	types: &Types,
	mods: &Modules,
	module: ModuleId,
	type_: &syn::Type,
) -> Vec<ProtoId> {
	let mut impls = Vec::with_capacity(type_.impls.len());
	for impl_name in &type_.impls {
		let Some(Binding::Member(owner, Member::Proto(item_id))) = mods.binding(module, *impl_name)
		else {
			panic!()
		};
		impls.push(types.proto_by_item[&(mods.chunk(owner), item_id)]);
	}
	impls
}

// Determine every member that *could* be acquired by a type which implements
// the given protocols. If the type declares its own version of any of these
// members, it will be overridden later. Two protocols never collide here,
// because collision checking happened earlier.
//
// A required member is not provided and so is never acquired; thus it is not
// included in this map.
fn acquired_members(types: &Types, pkg: &Package, impls: &[ProtoId]) -> HashMap<Sym, MemberSite> {
	let mut acquired = HashMap::new();
	for proto_id in impls {
		let proto = types.get_proto(*proto_id);
		let proto_chunk = pkg.get_chunk(proto.chunk);
		for (member, item_id) in &proto.members {
			if !is_provided(proto_chunk, &proto_chunk.get_proto_item(*item_id).def) {
				continue;
			}
			acquired.insert(*member, MemberSite::Provided(proto.chunk, *item_id));
		}
	}
	acquired
}

#[cfg(test)]
mod tests {
	use super::*;

	use crate::intern::Interner;
	use crate::sem::load::{self, ROOT_FILE};

	// A described package built from literal source, which keeps every case
	// below testable without a temp dir.
	fn describe_package(files: &[(&str, &str)]) -> (Interner, Package, Types) {
		let mut syms = Interner::new();
		let files = files
			.iter()
			.map(|(file, text)| (file.to_string(), text.to_string()))
			.collect();
		let (pkg, _) = load::parse(&mut syms, files).unwrap();
		let (_, types) = crate::sem::check(&mut syms, &pkg, &[]).unwrap();
		(syms, pkg, types)
	}

	fn named<'types>(types: &'types Types, syms: &Interner, name: &str) -> &'types Type {
		types
			.types
			.iter()
			.find(|type_| syms.resolve(type_.name) == name)
			.unwrap()
	}

	fn sorted(syms: &Interner, names: impl Iterator<Item = Sym>) -> Vec<String> {
		let mut names: Vec<String> = names.map(|name| syms.resolve(name).to_string()).collect();
		names.sort();
		names
	}

	// The declaration a member resolves to, rendered as the name it is written
	// under and whether it was declared or provided — which is the whole
	// question the flattened map exists to answer.
	fn member(syms: &Interner, type_: &Type, member: &str) -> MemberSite {
		let (_, site) = type_
			.members
			.iter()
			.find(|(name, _)| syms.resolve(**name) == member)
			.unwrap();
		*site
	}

	fn site(pkg: &Package, syms: &Interner, type_: &Type, name: &str) -> String {
		match member(syms, type_, name) {
			MemberSite::Declared(chunk_id, item_id) => {
				let TypeItem::Method(syn::Method::Instance(def)) =
					pkg.get_chunk(chunk_id).get_type_item(item_id)
				else {
					panic!()
				};
				format!("declared {}", syms.resolve(def.name))
			}
			MemberSite::Provided(chunk_id, item_id) => {
				let item = pkg.get_chunk(chunk_id).get_proto_item(item_id);
				format!("provided {}", syms.resolve(item.def.name))
			}
		}
	}

	#[test]
	fn test_describes_a_type() {
		let (syms, _, types) = describe_package(&[(
			ROOT_FILE,
			"module Test\n\
			 type Point(x, y)\n\
			 \tlabel := \"p\"\n\
			 \tscale := 2\n\
			 \tdef sum()\n\
			 \tend\n\
			 \tdef self.origin()\n\
			 \tend\n\
			 \ttype Inner(z)\n\
			 \tend\n\
			 end\n",
		)]);

		let point = named(&types, &syms, "Point");
		assert_eq!(
			vec!["x", "y"],
			sorted(&syms, point.ctor_fields.iter().map(|p| p.name))
		);
		// Body fields keep declaration order; the member and static maps do not.
		assert_eq!(
			vec!["label", "scale"],
			point
				.body_fields
				.keys()
				.map(|n| syms.resolve(*n))
				.collect::<Vec<_>>()
		);
		assert_eq!(vec!["sum"], sorted(&syms, point.members.keys().copied()));
		assert_eq!(
			vec!["Inner", "origin"],
			sorted(&syms, point.statics.keys().copied())
		);
		assert!(point.enclosing.is_none());
		assert_eq!(Some(&Vec::new()), point.variants.as_ref());

		// An inner type is enclosed but is not a variant, so its parent lists
		// no variants and the inner type is reachable only as a static.
		let inner = named(&types, &syms, "Inner");
		assert!(inner.enclosing.is_some());
		assert_eq!(Some(&Vec::new()), inner.variants.as_ref());
		assert!(inner.members.is_empty());
	}

	#[test]
	fn test_flattens_a_parents_methods_into_its_variants() {
		let (syms, pkg, types) = describe_package(&[(
			ROOT_FILE,
			"module Test\n\
			 type Expr\n\
			 \tcase Ident(name)\n\
			 \t\tdef show()\n\
			 \t\tend\n\
			 \tend\n\
			 \tcase Lit(value)\n\
			 \tend\n\
			 \tdef kind()\n\
			 \tend\n\
			 end\n",
		)]);

		let expr = named(&types, &syms, "Expr");
		let ident = named(&types, &syms, "Ident");
		let lit = named(&types, &syms, "Lit");
		assert_eq!(2, expr.variants.as_ref().unwrap().len());
		assert!(ident.variants.is_none() && lit.variants.is_none());
		assert_eq!(
			Some(&expr.name),
			Some(&types.get_type(ident.enclosing.unwrap()).name)
		);

		// A variant answers for its parent's methods as well as its own, so one
		// lookup replaces walking outward to the enclosing type.
		assert_eq!(
			vec!["kind", "show"],
			sorted(&syms, ident.members.keys().copied())
		);
		assert_eq!(vec!["kind"], sorted(&syms, lit.members.keys().copied()));
		assert_eq!("declared kind", site(&pkg, &syms, ident, "kind"));
	}

	#[test]
	fn test_flattens_provided_members() {
		let (syms, pkg, types) = describe_package(&[(
			ROOT_FILE,
			"module Test\n\
			 proto Order\n\
			 \tdef compare(other)\n\
			 \tend\n\
			 \tdef min(other)\n\
			 \t\treturn self\n\
			 \tend\n\
			 end\n\
			 type Task(priority)\n\
			 \timpl Order\n\
			 \tdef compare(other)\n\
			 \tend\n\
			 end\n\
			 type Fixed(priority)\n\
			 \timpl Order\n\
			 \tdef compare(other)\n\
			 \tend\n\
			 \tdef min(other)\n\
			 \t\treturn self\n\
			 \tend\n\
			 end\n",
		)]);

		// A required member resolves to the type's own declaration and a
		// provided one to the protocol's, both through the same map.
		let task = named(&types, &syms, "Task");
		assert_eq!(
			vec!["compare", "min"],
			sorted(&syms, task.members.keys().copied())
		);
		assert_eq!("declared compare", site(&pkg, &syms, task, "compare"));
		assert_eq!("provided min", site(&pkg, &syms, task, "min"));
		assert_eq!(1, task.impls.len());

		// Declaring a provided member overrides it, which is the override rule
		// falling out of the map rather than being applied at lookup.
		let fixed = named(&types, &syms, "Fixed");
		assert_eq!("declared min", site(&pkg, &syms, fixed, "min"));
	}

	#[test]
	fn test_acquires_a_provided_member_across_modules() {
		let (syms, pkg, types) = describe_package(&[
			(
				ROOT_FILE,
				"module Test\n\
				 proto Order\n\
				 \tdef compare(other)\n\
				 \tend\n\
				 \tdef min(other)\n\
				 \t\treturn self\n\
				 \tend\n\
				 end\n",
			),
			(
				"src/task.ms",
				"module Test.Task\n\
				 import Test.Order\n\
				 type Task(priority)\n\
				 \timpl Order\n\
				 \tdef compare(other)\n\
				 \tend\n\
				 end\n",
			),
		]);

		// The site names the protocol's chunk, not the implementing type's,
		// which is what lets rt reach the body through the module that wrote it.
		let task = named(&types, &syms, "Task");
		assert_eq!("provided min", site(&pkg, &syms, task, "min"));
		let MemberSite::Provided(chunk_id, _) = member(&syms, task, "min") else {
			panic!()
		};
		assert_ne!(task.chunk, chunk_id);
		assert_eq!(types.get_proto(task.impls[0]).chunk, chunk_id);
	}

	#[test]
	fn test_a_parents_method_beats_a_variants_provided_member() {
		let (syms, pkg, types) = describe_package(&[(
			ROOT_FILE,
			"module Test\n\
			 proto Named\n\
			 \tdef name()\n\
			 \tend\n\
			 \tdef label()\n\
			 \t\treturn self.name()\n\
			 \tend\n\
			 end\n\
			 type Expr\n\
			 \tcase Ident(n)\n\
			 \t\timpl Named\n\
			 \t\tdef name()\n\
			 \t\tend\n\
			 \tend\n\
			 \tdef label()\n\
			 \tend\n\
			 end\n",
		)]);

		// Four tiers collapse into one map, and the ordering they encoded is
		// preserved: a parent's declared method outranks a member the variant
		// acquires from its own protocol.
		let ident = named(&types, &syms, "Ident");
		assert_eq!(
			vec!["label", "name"],
			sorted(&syms, ident.members.keys().copied())
		);
		assert_eq!("declared label", site(&pkg, &syms, ident, "label"));
		assert_eq!("declared name", site(&pkg, &syms, ident, "name"));
	}
}
