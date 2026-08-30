use std::collections::{HashMap, HashSet};

use ordermap::OrderMap;

use crate::intern::{Interner, Sym};
use crate::pkg::{PackageId, Packages};
use crate::sem::modules::{Member, ModuleId, Modules, Resolved};
use crate::sem::{Error, Visit, check_params};
use crate::src::{Sources, Span};
use crate::syn::Chunks;
use crate::syn::nodes::{Def, DefId, Expr, ExprId, Lit, ModuleItem, ModuleItemId, Param, TypeItem};
use crate::syn::{self, Chunk, ChunkId};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TypeId(u32);

impl TypeId {
	pub fn index(self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ProtoId(u32);

impl ProtoId {
	pub fn index(self) -> usize {
		self.0 as usize
	}
}

// Identity of a native implementation. Global, unlike a 'TypeId'. Not specific
// to any one package, since the Rust code it addresses is linked once into the
// whole program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeTypeId(u32);

impl NativeTypeId {
	pub(crate) const fn new(id: u32) -> Self {
		Self(id)
	}

	pub fn index(self) -> usize {
		self.0 as usize
	}
}

// Every type and protocol that a package declares.
#[derive(Debug)]
pub struct Types {
	types: Vec<Type>,
	protos: Vec<Proto>,
	// Where a type's declaration lives, so the walk that binds its name arrives
	// at the description built for it. Only top-level types are keyed here; a
	// nested one is reached as a static of the type enclosing it.
	type_by_item: HashMap<(ChunkId, ModuleItemId), TypeId>,
	// Where a protocol's declaration lives, so an 'impl' name resolved through
	// the module map arrives at the description built for it.
	proto_by_item: HashMap<(ChunkId, ModuleItemId), ProtoId>,
	// Which protocol a proto item's def belongs to, so a walk that reaches a
	// def without going through its declaring 'Proto' can still name the
	// protocol and look up whether the item is required or provided.
	proto_by_proto_item: HashMap<(ChunkId, DefId), ProtoId>,
}

#[derive(Debug)]
pub enum Type {
	User(UserType),
	Native(NativeType),
}

impl Type {
	pub fn name(&self) -> Sym {
		match self {
			Type::User(typ) => typ.name,
			Type::Native(typ) => typ.name,
		}
	}

	pub fn members(&self) -> &HashMap<Sym, MemberSite> {
		match self {
			Type::User(typ) => &typ.members,
			Type::Native(typ) => &typ.members,
		}
	}

	pub fn statics(&self) -> &HashMap<Sym, Static> {
		match self {
			Type::User(typ) => &typ.statics,
			Type::Native(typ) => &typ.statics,
		}
	}

	pub fn impls(&self) -> &[(PackageId, ProtoId)] {
		match self {
			Type::User(typ) => &typ.impls,
			Type::Native(typ) => &typ.impls,
		}
	}
}

#[derive(Debug)]
pub struct UserType {
	pub name: Sym,
	pub chunk: ChunkId,
	pub span: Span,
	pub ctor_fields: Vec<Param>,
	pub body_fields: OrderMap<Sym, Field>,
	// Every member reachable on an instance, flattened. Includes what this type
	// declares, what it inherits as a case variant, and what it acquires from
	// the protocols it implements.
	pub members: HashMap<Sym, MemberSite>,
	pub statics: HashMap<Sym, Static>,
	pub impls: Vec<(PackageId, ProtoId)>,
	// Type this type was declared in; None at the module level.
	pub enclosing: Option<TypeId>,
	// Variants of this type; None if this type is itself a variant.
	pub variants: Option<Vec<TypeId>>,
}

// A type whose data and (some of) whose members are implemented in Rust. The
// implementations themselves are elided, to be specified by the runtime; this
// is the shape a native member presents to a caller.
#[derive(Debug)]
pub struct NativeType {
	pub name: Sym,
	pub chunk: ChunkId,
	pub span: Span,
	// The provider this declaration binds to.
	pub provider: NativeTypeId,
	// What Rust provides. Kept apart from the merged sets below because
	// conformance is decided by comparing against it.
	pub native_members: HashMap<Sym, Vec<NativeParam>>,
	pub native_statics: HashMap<Sym, Vec<NativeParam>>,
	// Every member reachable on a value of this type, flattened: Rust's, the
	// declaration's, and what it acquires from the protocols it implements.
	pub members: HashMap<Sym, MemberSite>,
	pub statics: HashMap<Sym, Static>,
	pub impls: Vec<(PackageId, ProtoId)>,
}

// A native member's parameter. Unlike a declared 'Param' there is no default
// expression to point at, only whether one exists: a native default is a Rust
// function producing a value, with nothing in any chunk to compare against.
#[derive(Debug, Clone)]
pub struct NativeParam {
	pub name: Sym,
	pub has_default: bool,
}

// The minimal information that the semantic checker needs to know about one of
// a package's native implementations: the provider an 'extern' of this name
// binds to, and the parameter shapes it must check declared members against
// for conformance.
pub struct NativeTypeShape {
	pub provider: NativeTypeId,
	pub members: HashMap<Sym, Vec<NativeParam>>,
	pub statics: HashMap<Sym, Vec<NativeParam>>,
}

// A declared protocol, its members split by whether they are required to be
// implemented by or provided directly to the implementing type.
#[derive(Debug)]
pub struct Proto {
	pub name: Sym,
	pub chunk: ChunkId,
	pub span: Span,
	pub required: OrderMap<Sym, DefId>,
	pub provided: OrderMap<Sym, DefId>,
	// Every protocol reachable through this one's prerequisites, itself
	// included, ordered so that a protocol always follows everything it
	// requires.
	pub impls: Vec<(PackageId, ProtoId)>,
	// Every member name declared anywhere in 'impls'.
	pub effective: HashSet<Sym>,
}

// A body field's initializer. Carries a span for more precise error reporting.
#[derive(Debug)]
pub struct Field {
	pub span: Span,
	pub init: ExprId,
}

// The declaration implementing a member. Includes a reference to the
// implementer's chunk, which may differ for e.g. an acquired protocol method.
// Differentiates between a declared method and a provided method because a
// provided method lives in a different arena.
#[derive(Debug, Clone, Copy)]
pub enum MemberSite {
	// A type only ever declares a member in its own file, so a declared site
	// needs no package; a provided one is read from the protocol's.
	Declared(ChunkId, DefId),
	Provided(PackageId, ChunkId, DefId),
	// An 'extern type' member implemented in Rust. It has no chunk or item to
	// point at: the implementation is in the native type's member map, and the
	// declaration only names it.
	Native,
}

#[derive(Debug, Clone, Copy)]
pub enum Static {
	Type(TypeId),
	// An 'extern type' static implemented in Rust. Does not have any source
	// code, and so belongs to no chunk.
	Native,
	// Like a declared member, a static is always written in the type's own
	// file; protocols have no static methods to provide.
	Proc(ChunkId, DefId),
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
	// Every type the package declares, nested ones included, in the order they
	// were described.
	pub fn ids(&self) -> impl ExactSizeIterator<Item = TypeId> {
		(0..self.types.len() as u32).map(TypeId)
	}

	pub fn get_type(&self, id: TypeId) -> &Type {
		&self.types[id.index()]
	}

	// The type a top-level declaration describes.
	pub fn get_type_by_item(&self, chunk: ChunkId, item_id: ModuleItemId) -> TypeId {
		self.type_by_item[&(chunk, item_id)]
	}

	// Every protocol the package declares, in the order they were described.
	pub fn proto_ids(&self) -> impl ExactSizeIterator<Item = ProtoId> {
		(0..self.protos.len() as u32).map(ProtoId)
	}

	pub fn get_proto(&self, id: ProtoId) -> &Proto {
		&self.protos[id.index()]
	}

	// The protocol a declaration describes.
	pub fn get_proto_by_item(&self, chunk: ChunkId, item_id: ModuleItemId) -> ProtoId {
		self.proto_by_item[&(chunk, item_id)]
	}

	// The protocol a def belongs to, if it is a proto item at all; a
	// module-level or method def belongs to none.
	pub fn try_get_proto_by_proto_item(&self, chunk: ChunkId, def_id: DefId) -> Option<ProtoId> {
		self.proto_by_proto_item.get(&(chunk, def_id)).copied()
	}

	fn add_proto(&mut self, chunk: ChunkId, item_id: ModuleItemId, proto: Proto) -> ProtoId {
		let id = ProtoId(self.protos.len() as u32);
		for member_item in proto.required.values().chain(proto.provided.values()) {
			self.proto_by_proto_item.insert((chunk, *member_item), id);
		}
		self.protos.push(proto);
		self.proto_by_item.insert((chunk, item_id), id);
		id
	}
}

// Describe every type and protocol in the package, resolving 'impl' names and
// deciding conformance as it goes. Unresolved names and conformance failures
// are reported here rather than assumed impossible.
pub fn check(
	syms: &Interner,
	sources: &Sources,
	chunks: &Chunks,
	pkgs: &Packages,
	mods: &Modules,
	natives: &HashMap<Sym, NativeTypeShape>,
) -> (Types, Vec<Error>) {
	let mut types = Types {
		types: Vec::new(),
		protos: Vec::new(),
		type_by_item: HashMap::new(),
		proto_by_item: HashMap::new(),
		proto_by_proto_item: HashMap::new(),
	};
	let mut errs = Vec::new();

	// Describe protocols first, package-wide, so a type can implement one
	// declared in any file.
	for module in mods.ids() {
		let chunk_id = mods.chunk(module);
		let chunk = chunks.get(chunk_id);
		for item_id in &chunk.top {
			if let ModuleItem::Proto(proto) = chunk.get_module_item(*item_id) {
				let span = chunk.get_module_item_span(*item_id);
				let desc = describe_proto(chunk, chunk_id, span, proto);
				types.add_proto(chunk_id, *item_id, desc);
			}
		}
	}
	// Resolve after every protocol is described, so a prerequisite may be
	// declared in any file of the package, and after every dependency package
	// is built, so a prerequisite may cross a package boundary.
	let mut impls: Vec<Vec<(PackageId, ProtoId)>> = vec![Vec::new(); types.protos.len()];
	for module in mods.ids() {
		let chunk_id = mods.chunk(module);
		let chunk = chunks.get(chunk_id);
		for item_id in &chunk.top {
			if let ModuleItem::Proto(proto) = chunk.get_module_item(*item_id) {
				let span = chunk.get_module_item_span(*item_id);
				let id = types.get_proto_by_item(chunk_id, *item_id);
				impls[id.index()] = resolve_impls(
					syms,
					sources,
					pkgs,
					mods,
					&types,
					module,
					span,
					&proto.impls,
					&mut errs,
				);
			}
		}
	}
	errs.append(&mut close_protos(
		syms,
		sources,
		pkgs,
		mods.pkg(),
		&mut types,
		&impls,
	));

	errs.append(&mut check_protos(syms, sources, pkgs, mods, &types));

	for module in mods.ids() {
		let chunk_id = mods.chunk(module);
		let chunk = chunks.get(chunk_id);
		for item_id in &chunk.top {
			let span = chunk.get_module_item_span(*item_id);
			match chunk.get_module_item(*item_id) {
				ModuleItem::Type(type_) => {
					let id = describe_type(
						syms, sources, chunks, pkgs, mods, &mut types, module, chunk_id, span,
						type_, None, None, false, &mut errs,
					);
					types.type_by_item.insert((chunk_id, *item_id), id);
				}
				ModuleItem::Extern(extern_) => {
					if let Some(id) = describe_extern(
						syms, sources, chunks, pkgs, mods, &mut types, natives, module, chunk_id,
						span, extern_, &mut errs,
					) {
						types.type_by_item.insert((chunk_id, *item_id), id);
					}
				}
				_ => continue,
			}
		}
	}

	errs.append(&mut check_structure(syms, sources, &types));

	(types, errs)
}

// Describe a protocol declaration, sorting its members into what a type
// implementing it must declare and what it may inherit as-is.
fn describe_proto(
	chunk: &Chunk,
	chunk_id: ChunkId,
	span: Span,
	proto: &syn::nodes::Proto,
) -> Proto {
	let mut required = OrderMap::new();
	let mut provided = OrderMap::new();
	for def_id in &proto.items {
		let def = chunk.get_def(*def_id);
		// A proc is provided if its def block is not empty, otherwise it is
		// required.
		if !chunk.get_block(def.body).exprs.is_empty() {
			provided.insert(def.name, *def_id);
		} else {
			required.insert(def.name, *def_id);
		}
	}
	Proto {
		name: proto.name,
		chunk: chunk_id,
		span,
		required,
		provided,
		impls: Vec::new(),
		effective: HashSet::new(),
	}
}

// Check that a protocol provides members only if it also requires at least one
// other member. A protocol with no members at all also passes this check. If a
// protocol had only provided members, it would have no contract for an
// implementing type to implement.
fn check_protos(
	syms: &Interner,
	sources: &Sources,
	pkgs: &Packages,
	mods: &Modules,
	types: &Types,
) -> Vec<Error> {
	let mut errs = Vec::new();
	for id in types.proto_ids() {
		let proto = types.get_proto(id);
		// A protocol satisfies the rule if anything it requires, *directly or
		// through a prerequisite*, has a required member.
		let has_requirement = proto.impls.iter().any(|(pkg, id)| {
			let owner = owner_types(pkgs, mods.pkg(), types, *pkg);
			!owner.get_proto(*id).required.is_empty()
		});
		if !proto.provided.is_empty() && !has_requirement {
			errs.push(Error::ProvidedWithoutRequired(
				sources.loc(proto.span),
				syms.resolve(proto.name).to_string(),
			));
		}
	}
	errs
}

// Walk each proto's implied impls to build a transitive closure of each proto's
// impl set and an effective set of required & provided members. A DFS that
// marks each proto on entry and unmarks it on exit, so an impl of a proto still
// marked is one that impls back onto the path being walked.
//
// *Import* cycles are already ruled out by the module-level cycle check. An
// impl cycle could occur by two protocols requiring one another.
fn close_protos(
	syms: &Interner,
	sources: &Sources,
	pkgs: &Packages,
	pkg_id: PackageId,
	types: &mut Types,
	impls: &[Vec<(PackageId, ProtoId)>],
) -> Vec<Error> {
	let mut errs = Vec::new();
	let mut visits = vec![Visit::Unseen; types.protos.len()];
	let mut path = Vec::new();
	let ids: Vec<ProtoId> = types.proto_ids().collect();
	for id in ids {
		close_proto(
			syms,
			sources,
			pkgs,
			pkg_id,
			types,
			impls,
			id,
			&mut visits,
			&mut path,
			&mut errs,
		);
	}
	errs
}

fn close_proto(
	syms: &Interner,
	sources: &Sources,
	pkgs: &Packages,
	pkg_id: PackageId,
	types: &mut Types,
	impls: &[Vec<(PackageId, ProtoId)>],
	id: ProtoId,
	visits: &mut [Visit],
	path: &mut Vec<ProtoId>,
	errs: &mut Vec<Error>,
) {
	// Reached only for a protocol already walked to completion: the loop below
	// never recurses into one still on the path, having reported it instead.
	if visits[id.index()] != Visit::Unseen {
		return;
	}
	visits[id.index()] = Visit::OnPath;
	path.push(id);

	// Walk into every prerequisite this protocol declares, so that each has a
	// closure to splice by the time this one is expanded.
	let direct = &impls[id.index()];
	for (dep_pkg, dep_id) in direct {
		// Don't walk a prerequisite in another package. Packages load in a
		// fixed order, so the current package can only name one already fully
		// described.
		if *dep_pkg != pkg_id {
			continue;
		}
		// If this protocol impls one still being walked, it's a cycle. The
		// location is this protocol's declaration, the nearest thing to the
		// impl closing the loop that a resolved impl can point at.
		if visits[dep_id.index()] == Visit::OnPath {
			errs.push(Error::ProtocolCycle(
				sources.loc(types.get_proto(id).span),
				render_proto_cycle(syms, types, path, *dep_id),
			));
			continue;
		}
		close_proto(
			syms, sources, pkgs, pkg_id, types, impls, *dep_id, visits, path, errs,
		);
	}
	// A cut back edge leaves its protocol's closure empty, contributing
	// nothing here, which is what keeps a reported cycle from expanding
	// forever. Nothing else can reach back to this protocol, so it is never
	// already present in its own expansion.
	let mut closure = expand_impls(pkgs, pkg_id, types, direct);
	closure.push((pkg_id, id));

	path.pop();
	visits[id.index()] = Visit::Done;
	let effective = effective_members(pkgs, pkg_id, types, &closure);
	let proto = &mut types.protos[id.index()];
	proto.impls = closure;
	proto.effective = effective;
}

// Every member name a closure declares, required or provided, which is exactly
// what a body written in any of those protocols may reach for on 'self'.
fn effective_members(
	pkgs: &Packages,
	pkg_id: PackageId,
	types: &Types,
	closure: &[(PackageId, ProtoId)],
) -> HashSet<Sym> {
	let mut effective = HashSet::new();
	for (pkg, id) in closure {
		let proto = owner_types(pkgs, pkg_id, types, *pkg).get_proto(*id);
		effective.extend(proto.required.keys().copied());
		effective.extend(proto.provided.keys().copied());
	}
	effective
}

// Render a protocol cycle as the walk found it, e.g. 'Hash → Equal → Hash',
// rather than naming only the back edge.
fn render_proto_cycle(syms: &Interner, types: &Types, path: &[ProtoId], back: ProtoId) -> String {
	let start = path.iter().position(|id| *id == back).unwrap();
	let mut names: Vec<String> = path[start..]
		.iter()
		.map(|id| syms.resolve(types.get_proto(*id).name).to_string())
		.collect();
	names.push(syms.resolve(types.get_proto(back).name).to_string());
	names.join(" → ")
}

// The 'Types' a package reference points into. The package being checked is not
// in 'pkgs' yet, so a reference to it reads from the arena still being built.
fn owner_types<'types>(
	pkgs: &'types Packages,
	pkg_id: PackageId,
	types: &'types Types,
	pkg: PackageId,
) -> &'types Types {
	match pkg == pkg_id {
		true => types,
		false => &pkgs.get(pkg).types,
	}
}

// Expand resolved prerequisites into everything they imply. The return value
// contains no duplicates and is ordered so that a prerequisite's own
// implication comes before it. This latter guarantee ensures that, when
// iterating this result, a body of a nearer impl can refine a farther one.
fn expand_impls(
	pkgs: &Packages,
	pkg_id: PackageId,
	types: &Types,
	direct: &[(PackageId, ProtoId)],
) -> Vec<(PackageId, ProtoId)> {
	let mut expanded = Vec::new();
	for (pkg, id) in direct {
		let owner = owner_types(pkgs, pkg_id, types, *pkg);
		expanded.extend(owner.get_proto(*id).impls.iter().copied());
	}
	let mut seen = HashSet::with_capacity(expanded.len());
	expanded.retain(|item| seen.insert(*item));
	expanded
}

// Describe a type declaration, producing a description that reduces all later
// member resolution to a single table lookup. Resolve the type's 'impl' names
// and decide conformance against each.
fn describe_type(
	syms: &Interner,
	sources: &Sources,
	chunks: &Chunks,
	pkgs: &Packages,
	mods: &Modules,
	types: &mut Types,
	module: ModuleId,
	chunk_id: ChunkId,
	span: Span,
	type_: &syn::nodes::Type,
	inherited: Option<&Inherited>,
	enclosing: Option<TypeId>,
	is_case: bool,
	errs: &mut Vec<Error>,
) -> TypeId {
	let chunk = chunks.get(chunk_id);
	// Claim the id before the body is walked so that a nested type can name the
	// type enclosing it while that type is still being described.
	let id = TypeId(types.types.len() as u32);
	types.types.push(Type::User(UserType {
		name: type_.name,
		chunk: chunk_id,
		span,
		ctor_fields: type_.params.to_vec(),
		body_fields: OrderMap::new(),
		members: HashMap::new(),
		statics: HashMap::new(),
		impls: Vec::new(),
		enclosing,
		variants: None,
	}));

	let mut body_fields = OrderMap::new();
	let mut statics = HashMap::new();
	// Every static's span, case and inner types included, kept only to report
	// where a member acquired from a protocol collides with one. The 'statics'
	// map above cannot serve this because a case/inner type's 'Static::Type'
	// is not known until it is described, below.
	let mut static_spans: HashMap<Sym, Span> = HashMap::new();
	let mut declared = HashMap::new();
	// A ctor param, a body field and an instance method are recorded in three
	// different maps but share one namespace: all three are what 'inst.x'
	// reaches, and two of them claiming one name leaves the earlier
	// unreachable. Statics, cases and inner types share the other namespace,
	// the one 'T.x' reaches, which 'static_spans' already collects.
	let mut instance_names: HashSet<Sym> = type_.params.iter().map(|param| param.name).collect();
	let mut static_names = HashSet::new();
	for item_id in &type_.items {
		let item_span = chunk.get_type_item_span(*item_id);
		let (name, duplicate) = match chunk.get_type_item(*item_id) {
			// Ignore cases and inner types here because they are described below.
			//
			// A variant's members *must* be resolved after their parent's
			// members because they inherit some members from their parent.
			//
			// An inner type inherits nothing but describing here would
			// interleave unrelated work.
			TypeItem::Case(variant) => {
				static_spans.insert(variant.name, item_span);
				(variant.name, !static_names.insert(variant.name))
			}
			TypeItem::Field(field) => {
				body_fields.insert(
					field.name,
					Field {
						span: item_span,
						init: field.init,
					},
				);
				(field.name, !instance_names.insert(field.name))
			}
			TypeItem::Type(inner) => {
				static_spans.insert(inner.name, item_span);
				(inner.name, !static_names.insert(inner.name))
			}
			TypeItem::Method(syn::nodes::Method::Instance(def_id)) => {
				let name = chunk.get_def(*def_id).name;
				declared.insert(name, MemberSite::Declared(chunk_id, *def_id));
				(name, !instance_names.insert(name))
			}
			TypeItem::Method(syn::nodes::Method::Static(def_id)) => {
				let name = chunk.get_def(*def_id).name;
				statics.insert(name, Static::Proc(chunk_id, *def_id));
				static_spans.insert(name, item_span);
				(name, !static_names.insert(name))
			}
		};
		if duplicate {
			errs.push(Error::DuplicateTypeMember(
				sources.loc(item_span),
				syms.resolve(type_.name).to_string(),
				syms.resolve(name).to_string(),
			));
		}
	}

	let direct = resolve_impls(
		syms,
		sources,
		pkgs,
		mods,
		types,
		module,
		span,
		&type_.impls,
		errs,
	);
	let impls = expand_impls(pkgs, mods.pkg(), types, &direct);
	let acquired = acquire_members(
		syms,
		sources,
		pkgs,
		mods.pkg(),
		chunks,
		types,
		type_.name,
		span,
		&declared,
		&HashMap::new(),
		inherited,
		&static_spans,
		&impls,
		errs,
	);

	// Union the four tiers by overwriting in the inverse order of their
	// precedence.
	//
	//   own declared > parent declared > own acquired > parent acquired
	//
	// An own declared method always overrides a parent's declared method, and
	// own acquired overrides a parent's acquired one.
	//
	// The four merges below account for these, but they also do redundant work:
	// declared overriddes the acquisitions even though that is already
	// accounted for by acquire_members. They stay as-is so that 'members' reads
	// clearly as "these four tiers, most specific wins".
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
		let item_span = chunk.get_type_item_span(*item_id);
		match chunk.get_type_item(*item_id) {
			TypeItem::Case(variant) => {
				let variant_id = describe_type(
					syms,
					sources,
					chunks,
					pkgs,
					mods,
					types,
					module,
					chunk_id,
					item_span,
					variant,
					Some(&own),
					Some(id),
					true,
					errs,
				);
				statics.insert(variant.name, Static::Type(variant_id));
				variants.push(variant_id);
			}
			TypeItem::Field(..) => {}
			// An inner type is enclosed but not a variant, so it inherits
			// nothing: only a case refines the type it is written in.
			TypeItem::Type(inner) => {
				let inner_id = describe_type(
					syms,
					sources,
					chunks,
					pkgs,
					mods,
					types,
					module,
					chunk_id,
					item_span,
					inner,
					None,
					Some(id),
					false,
					errs,
				);
				statics.insert(inner.name, Static::Type(inner_id));
			}
			TypeItem::Method(..) => {}
		}
	}

	let Type::User(described) = &mut types.types[id.index()] else {
		panic!()
	};
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

// Describe an 'extern type', a type whose data and some of whose members come
// from Rust, and whose declaration names the rest. Allocates a real 'TypeId'
// like any other type, so a later reference to it resolves the same way;
// returns 'None' when the declaration names no registered implementation,
// leaving nothing to describe.
fn describe_extern(
	syms: &Interner,
	sources: &Sources,
	chunks: &Chunks,
	pkgs: &Packages,
	mods: &Modules,
	types: &mut Types,
	natives: &HashMap<Sym, NativeTypeShape>,
	module: ModuleId,
	chunk_id: ChunkId,
	span: Span,
	extern_: &syn::nodes::Extern,
	errs: &mut Vec<Error>,
) -> Option<TypeId> {
	let chunk = chunks.get(chunk_id);
	// Resolved before the implementation is looked up: an unresolvable protocol
	// is a mistake in the declaration either way, and reporting it does not
	// depend on there being anything to describe.
	let direct = resolve_impls(
		syms,
		sources,
		pkgs,
		mods,
		types,
		module,
		span,
		&extern_.impls,
		errs,
	);
	let impls = expand_impls(pkgs, mods.pkg(), types, &direct);

	// A declaration with no backing implementation is an implementation error,
	// and leaves nothing to describe it into.
	let Some(native) = natives.get(&extern_.name) else {
		errs.push(Error::UnimplementedExtern(
			sources.loc(span),
			syms.resolve(extern_.name).to_string(),
		));
		return None;
	};

	let mut declared = HashMap::new();
	let mut statics = HashMap::new();
	let mut static_spans: HashMap<Sym, Span> = HashMap::new();

	// Every member of an extern type is declared in its own body, natively
	// implemented or not, so a declaration displacing another is a duplicate
	// whichever kinds the two are. Members and statics keep separate maps and
	// so are separate namespaces: a name taken in one is still free in the
	// other. A name reported here is recorded so the sweep below does not
	// raise a second, derived complaint about the same mistake.
	let mut collided = HashSet::new();
	for item_id in &extern_.items {
		let item_span = chunk.get_extern_item_span(*item_id);
		let (name, claimed) = match chunk.get_extern_item(*item_id) {
			syn::nodes::ExternItem::User(syn::nodes::Method::Instance(def_id)) => {
				let name = chunk.get_def(*def_id).name;
				let claimed = declared
					.insert(name, MemberSite::Declared(chunk_id, *def_id))
					.is_some();
				(name, claimed)
			}
			syn::nodes::ExternItem::User(syn::nodes::Method::Static(def_id)) => {
				let name = chunk.get_def(*def_id).name;
				let claimed = statics
					.insert(name, Static::Proc(chunk_id, *def_id))
					.is_some();
				static_spans.insert(name, item_span);
				(name, claimed)
			}
			syn::nodes::ExternItem::Native(syn::nodes::Method::Instance(def_id)) => {
				let def = chunk.get_def(*def_id);
				check_native_signature(
					syms,
					sources,
					&native.members,
					extern_.name,
					def,
					item_span,
					errs,
				);
				let claimed = declared.insert(def.name, MemberSite::Native).is_some();
				(def.name, claimed)
			}
			syn::nodes::ExternItem::Native(syn::nodes::Method::Static(def_id)) => {
				let def = chunk.get_def(*def_id);
				check_native_signature(
					syms,
					sources,
					&native.statics,
					extern_.name,
					def,
					item_span,
					errs,
				);
				let claimed = statics.insert(def.name, Static::Native).is_some();
				static_spans.insert(def.name, item_span);
				(def.name, claimed)
			}
		};
		if claimed {
			collided.insert(name);
			errs.push(Error::DuplicateTypeMember(
				sources.loc(item_span),
				syms.resolve(extern_.name).to_string(),
				syms.resolve(name).to_string(),
			));
		}
	}

	// The registration is the source of truth for what a member does, and the
	// declaration for whether it exists; an implementation the body does not
	// name is reachable from no mesa source. Reported at the type, which is
	// the only span there is — the registration has none.
	for name in native.members.keys() {
		if let Some(MemberSite::Native) = declared.get(name) {
			continue;
		}
		if collided.contains(name) {
			continue;
		}
		errs.push(Error::UndeclaredNativeMember(
			sources.loc(span),
			syms.resolve(extern_.name).to_string(),
			syms.resolve(*name).to_string(),
		));
	}
	for name in native.statics.keys() {
		if let Some(Static::Native) = statics.get(name) {
			continue;
		}
		if collided.contains(name) {
			continue;
		}
		errs.push(Error::UndeclaredNativeMember(
			sources.loc(span),
			syms.resolve(extern_.name).to_string(),
			syms.resolve(*name).to_string(),
		));
	}

	let acquired = acquire_members(
		syms,
		sources,
		pkgs,
		mods.pkg(),
		chunks,
		types,
		extern_.name,
		span,
		&declared,
		&native.members,
		None,
		&static_spans,
		&impls,
		errs,
	);

	let mut members = acquired;
	members.extend(&declared);

	let id = TypeId(types.types.len() as u32);
	types.types.push(Type::Native(NativeType {
		name: extern_.name,
		chunk: chunk_id,
		span,
		provider: native.provider,
		native_members: native.members.clone(),
		native_statics: native.statics.clone(),
		members,
		statics,
		impls,
	}));
	Some(id)
}

// Check that the structure of a type is valid. It must not have fields if it is
// a case parent, and it must declare params in the standard
// required-before-optional order.
fn check_structure(syms: &Interner, sources: &Sources, types: &Types) -> Vec<Error> {
	let mut errs = Vec::new();
	for id in types.ids() {
		// A native type has no ctor params or body fields of its own to order
		// or forbid: its declaration only names an implementation.
		let Type::User(type_) = types.get_type(id) else {
			continue;
		};

		if let Err(err) = check_params(syms, sources, type_.span, &type_.ctor_fields) {
			errs.push(err);
		}

		let has_cases = if let Some(variants) = &type_.variants
			&& !variants.is_empty()
		{
			true
		} else {
			false
		};
		if !has_cases {
			continue;
		}
		if !type_.ctor_fields.is_empty() {
			errs.push(Error::ParamsOnCaseParent(
				sources.loc(type_.span),
				syms.resolve(type_.name).to_string(),
			));
		}
		if let Some((name, field)) = type_.body_fields.iter().next() {
			errs.push(Error::FieldOnCaseParent(
				sources.loc(field.span),
				syms.resolve(type_.name).to_string(),
				syms.resolve(*name).to_string(),
			));
		}
	}
	errs
}

// Resolve the protocols an 'impl' line names against the symbols that the
// declaring file binds. A name that fails to resolve contributes no 'ProtoId'.
fn resolve_impls(
	syms: &Interner,
	sources: &Sources,
	pkgs: &Packages,
	mods: &Modules,
	types: &Types,
	module: ModuleId,
	span: Span,
	paths: &[Vec<Sym>],
	errs: &mut Vec<Error>,
) -> Vec<(PackageId, ProtoId)> {
	let mut impls = Vec::with_capacity(paths.len());
	for path in paths {
		match mods.resolve_path_from(pkgs, module, path) {
			Some(Resolved::Member(pkg, owner, Member::Proto(item_id))) => {
				let owner_mods = if pkg == mods.pkg() {
					mods
				} else {
					&pkgs.get(pkg).modules
				};
				let id = owner_types(pkgs, mods.pkg(), types, pkg)
					.get_proto_by_item(owner_mods.chunk(owner), item_id);
				// One protocol named twice in one clause is a typo with no
				// reading that the author meant. Implication is silent, but
				// repetition is not: a name reached through a prerequisite is a
				// choice, a name written twice is a slip. Compared by id rather
				// than by path, so two spellings of one protocol still collide.
				if impls.contains(&(pkg, id)) {
					errs.push(Error::DuplicateImpl(
						sources.loc(span),
						syms.resolve_path(path),
					));
					continue;
				}
				impls.push((pkg, id));
			}
			Some(Resolved::Member(..) | Resolved::Module(..) | Resolved::Partial(..)) => {
				errs.push(Error::NotAProtocol(
					sources.loc(span),
					syms.resolve_path(path),
				));
			}
			None => {
				errs.push(Error::UnknownProtocol(
					sources.loc(span),
					syms.resolve_path(path),
				));
			}
		}
	}
	impls
}

// Determine every member that a type acquires from the protocols it implements,
// deciding conformance against each protocol's members as it goes. Flattened to
// just the *actual* protocol provisions, because every conformance error is a
// statement about what the flattening would otherwise produce.
fn acquire_members(
	syms: &Interner,
	sources: &Sources,
	pkgs: &Packages,
	pkg_id: PackageId,
	chunks: &Chunks,
	types: &Types,
	type_name: Sym,
	type_span: Span,
	declared: &HashMap<Sym, MemberSite>,
	native_members: &HashMap<Sym, Vec<NativeParam>>,
	inherited: Option<&Inherited>,
	static_spans: &HashMap<Sym, Span>,
	impls: &[(PackageId, ProtoId)],
	errs: &mut Vec<Error>,
) -> HashMap<Sym, MemberSite> {
	let mut acquired: HashMap<Sym, MemberSite> = HashMap::new();
	// The protocol each acquired member came from, kept so the next provider
	// of the same name can ask whether it is related to this one.
	let mut from: HashMap<Sym, (PackageId, ProtoId)> = HashMap::new();
	for (proto_pkg, proto_id) in impls {
		// The protocol may belong to another package, whose arenas are the only
		// place its declaration and body can be read from.
		let (proto_types, proto_chunks) = match *proto_pkg == pkg_id {
			true => (types, chunks),
			false => (&pkgs.get(*proto_pkg).types, &pkgs.get(*proto_pkg).chunks),
		};
		let proto = proto_types.get_proto(*proto_id);
		let proto_chunk = proto_chunks.get(proto.chunk);
		// Which map a member came from already answers whether it is provided,
		// so nothing here re-asks the body the way a lookup against
		// 'Proto.members' used to.
		let required = proto
			.required
			.iter()
			.map(|(member, item)| (member, item, false));
		let provided = proto
			.provided
			.iter()
			.map(|(member, item)| (member, item, true));
		for (member, item_id, provided) in required.chain(provided) {
			let def = proto_chunk.get_def(*item_id);

			// A required member is satisfied, or a provided one overridden, by
			// either the type's own declaration or one it inherits from a case
			// parent. A parent's declared method outranks a member the variant
			// would otherwise acquire from its own protocol.
			let own = if let Some(declared) = declared.get(member) {
				Some(declared)
			} else if let Some(inherited) = inherited
				&& let Some(declared) = inherited.declared.get(member)
			{
				Some(declared)
			} else {
				None
			};
			match own {
				Some(MemberSite::Declared(site_chunk, site_def)) => {
					let site_chunk = chunks.get(*site_chunk);
					let method_def = site_chunk.get_def(*site_def);
					if !signatures_agree(proto_chunk, site_chunk, &def.params, &method_def.params) {
						errs.push(Error::SignatureMismatch(
							sources.loc(site_chunk.get_def_span(*site_def)),
							syms.resolve(type_name).to_string(),
							syms.resolve(proto.name).to_string(),
							syms.resolve(*member).to_string(),
						));
					}
					continue;
				}
				Some(MemberSite::Provided(..)) => panic!(),
				Some(MemberSite::Native) => {
					let params = &native_members[member];
					if !native_signature_agrees(&def.params, params) {
						errs.push(Error::SignatureMismatch(
							sources.loc(type_span),
							syms.resolve(type_name).to_string(),
							syms.resolve(proto.name).to_string(),
							syms.resolve(*member).to_string(),
						));
					}
					continue;
				}
				None => {}
			}

			if !provided {
				errs.push(Error::MissingMember(
					sources.loc(type_span),
					syms.resolve(type_name).to_string(),
					syms.resolve(proto.name).to_string(),
					syms.resolve(*member).to_string(),
				));
				continue;
			}

			if let Some(static_span) = static_spans.get(member) {
				errs.push(Error::MemberCollision(
					sources.loc(*static_span),
					syms.resolve(type_name).to_string(),
					syms.resolve(proto.name).to_string(),
					syms.resolve(*member).to_string(),
				));
				continue;
			}

			if let Some(first) = from.insert(*member, (*proto_pkg, *proto_id)) {
				let first_proto = owner_types(pkgs, pkg_id, types, first.0).get_proto(first.1);
				// Related protocols do not conflict; the nearer body refines
				// the farther one, and the traversal order guarantees this
				// write is the nearer. Unrelated ones do, and the type
				// resolves it by declaring the member.
				let related = first_proto.impls.contains(&(*proto_pkg, *proto_id))
					|| proto.impls.contains(&first);
				if !related {
					errs.push(Error::ProtocolConflict(
						sources.loc(type_span),
						syms.resolve(first_proto.name).to_string(),
						syms.resolve(proto.name).to_string(),
						syms.resolve(*member).to_string(),
					));
					continue;
				}
			}

			acquired.insert(
				*member,
				MemberSite::Provided(*proto_pkg, proto.chunk, *item_id),
			);
		}
	}
	acquired
}

fn signatures_agree(
	proto_chunk: &Chunk,
	type_chunk: &Chunk,
	proto: &[Param],
	typ: &[Param],
) -> bool {
	if proto.len() != typ.len() {
		return false;
	}
	for (proto_param, param) in proto.iter().zip(typ) {
		if proto_param.name != param.name {
			return false;
		}
		if !defaults_agree(proto_chunk, type_chunk, proto_param.default, param.default) {
			return false;
		}
	}
	true
}

// Check whether a native member satisfies the requirements of a protocol
// signature. Arity, param names, and the existence of defaults must match.
// Default *values* are not compared, because they can be backed by an arbitrary
// Rust function.
// Check one 'extern def' against the registration it names: the member must be
// implemented, and the params it declares must agree with the implementation's.
fn check_native_signature(
	syms: &Interner,
	sources: &Sources,
	native: &HashMap<Sym, Vec<NativeParam>>,
	type_name: Sym,
	def: &Def,
	span: Span,
	errs: &mut Vec<Error>,
) {
	let Some(params) = native.get(&def.name) else {
		errs.push(Error::UnimplementedExternMember(
			sources.loc(span),
			syms.resolve(type_name).to_string(),
			syms.resolve(def.name).to_string(),
		));
		return;
	};
	if !native_signature_agrees(&def.params, params) {
		errs.push(Error::ExternSignatureMismatch(
			sources.loc(span),
			syms.resolve(type_name).to_string(),
			syms.resolve(def.name).to_string(),
		));
	}
}

fn native_signature_agrees(declared: &[Param], native: &[NativeParam]) -> bool {
	if declared.len() != native.len() {
		return false;
	}
	for (declared_param, param) in declared.iter().zip(native) {
		if declared_param.name != param.name {
			return false;
		}
		if declared_param.default.is_some() != param.has_default {
			return false;
		}
	}
	true
}

fn defaults_agree(
	proto_chunk: &Chunk,
	type_chunk: &Chunk,
	proto: Option<ExprId>,
	typ: Option<ExprId>,
) -> bool {
	let (proto, typ) = match (proto, typ) {
		(Some(proto), Some(typ)) => (proto, typ),
		(None, None) => return true,
		_ => return false,
	};
	// Only literals are compared; anything else is taken to agree until there
	// is a structural comparison over the arena.
	match (proto_chunk.get_expr(proto), type_chunk.get_expr(typ)) {
		(Expr::Lit(proto), Expr::Lit(typ)) => lits_agree(proto, typ),
		_ => true,
	}
}

fn lits_agree(proto: &Lit, typ: &Lit) -> bool {
	match (proto, typ) {
		(Lit::Str(proto), Lit::Str(typ)) => proto == typ,
		(Lit::Char(proto), Lit::Char(typ)) => proto == typ,
		(Lit::Num(proto), Lit::Num(typ)) => proto == typ,
		(Lit::Bool(proto), Lit::Bool(typ)) => proto == typ,
		(Lit::List(..), _) | (_, Lit::List(..)) => true,
		(Lit::Dict(..), _) | (_, Lit::Dict(..)) => true,
		(Lit::Nil, Lit::Nil) => true,
		_ => false,
	}
}
