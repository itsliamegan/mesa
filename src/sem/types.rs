use std::collections::HashMap;

use ordermap::OrderMap;

use crate::intern::{Interner, Sym};
use crate::pkg::Packages;
use crate::sem::modules::{Member, ModuleId, Modules};
use crate::sem::{Error, check_required_precede_optional};
use crate::src::{Sources, Span};
use crate::syn::Chunks;
use crate::syn::nodes::{
	Expr, ExprId, Lit, ModuleItem, ModuleItemId, Param, ProtoItemId, TypeItem, TypeItemId,
};
use crate::syn::{self, Chunk, ChunkId};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TypeId(u32);

impl TypeId {
	pub fn index(self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug, Clone, Copy)]
pub struct ProtoId(u32);

impl ProtoId {
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
	// Which protocol a proto item belongs to, so a flat walk over a chunk's
	// 'proto_items' arena (which does not itself know which 'ModuleItem::Proto'
	// grouped it) can still name the protocol and look up whether the item is
	// required or provided.
	proto_by_proto_item: HashMap<(ChunkId, ProtoItemId), ProtoId>,
}

#[derive(Debug)]
pub struct Type {
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
	pub impls: Vec<ProtoId>,
	// Type this type was declared in; None at the module level.
	pub enclosing: Option<TypeId>,
	// Variants of this type; None if this type is itself a variant.
	pub variants: Option<Vec<TypeId>>,
}

// A declared protocol, its members split by whether they are required to be
// implemented by or provided directly to the implementing type.
#[derive(Debug)]
pub struct Proto {
	pub name: Sym,
	pub chunk: ChunkId,
	pub span: Span,
	pub required: OrderMap<Sym, ProtoItemId>,
	pub provided: OrderMap<Sym, ProtoItemId>,
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
	Declared(ChunkId, TypeItemId),
	Provided(ChunkId, ProtoItemId),
}

#[derive(Debug, Clone, Copy)]
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

	// The protocol a proto item belongs to.
	pub fn get_proto_by_proto_item(&self, chunk: ChunkId, item_id: ProtoItemId) -> ProtoId {
		self.proto_by_proto_item[&(chunk, item_id)]
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
	errs.append(&mut check_protos(syms, sources, &types));

	for module in mods.ids() {
		let chunk_id = mods.chunk(module);
		let chunk = chunks.get(chunk_id);
		for item_id in &chunk.top {
			if let ModuleItem::Type(type_) = chunk.get_module_item(*item_id) {
				let span = chunk.get_module_item_span(*item_id);
				let id = describe_type(
					syms, sources, chunks, pkgs, mods, &mut types, module, chunk_id, span, type_,
					None, None, false, &mut errs,
				);
				types.type_by_item.insert((chunk_id, *item_id), id);
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
	for item_id in &proto.items {
		let item = chunk.get_proto_item(*item_id);
		// A proc is provided if its def block is not empty, otherwise it is
		// required.
		if !chunk.get_block(item.def.body).exprs.is_empty() {
			provided.insert(item.def.name, *item_id);
		} else {
			required.insert(item.def.name, *item_id);
		}
	}
	Proto {
		name: proto.name,
		chunk: chunk_id,
		span,
		required,
		provided,
	}
}

// A protocol must either provide members or only require them: a protocol
// that provides members but requires none has nothing a type must declare to
// implement it.
fn check_protos(syms: &Interner, sources: &Sources, types: &Types) -> Vec<Error> {
	let mut errs = Vec::new();
	for id in types.proto_ids() {
		let proto = types.get_proto(id);
		if !proto.provided.is_empty() && proto.required.is_empty() {
			errs.push(Error::ProvidedWithoutRequired(
				sources.loc(proto.span),
				syms.resolve(proto.name).to_string(),
			));
		}
	}
	errs
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
	types.types.push(Type {
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
	});

	let mut body_fields = OrderMap::new();
	let mut statics = HashMap::new();
	// Every static's span, case and inner types included, kept only to report
	// where a member acquired from a protocol collides with one. The 'statics'
	// map above cannot serve this because a case/inner type's 'Static::Type'
	// is not known until it is described, below.
	let mut static_spans: HashMap<Sym, Span> = HashMap::new();
	let mut declared = HashMap::new();
	for item_id in &type_.items {
		let item_span = chunk.get_type_item_span(*item_id);
		match chunk.get_type_item(*item_id) {
			// Ignore cases and inner types here because they are described below.
			//
			// A variant's members *must* be resolved after their parent's
			// members because they inherit some members from their parent.
			//
			// An inner type inherits nothing but describing here would
			// interleave unrelated work.
			TypeItem::Case(variant) => {
				static_spans.insert(variant.name, item_span);
			}
			TypeItem::Field(field) => {
				body_fields.insert(
					field.name,
					Field {
						span: item_span,
						init: field.init,
					},
				);
			}
			TypeItem::Type(inner) => {
				static_spans.insert(inner.name, item_span);
			}
			TypeItem::Method(syn::nodes::Method::Instance(def)) => {
				declared.insert(def.name, MemberSite::Declared(chunk_id, *item_id));
			}
			TypeItem::Method(syn::nodes::Method::Static(def)) => {
				statics.insert(def.name, Static::Proc(chunk_id, *item_id));
				static_spans.insert(def.name, item_span);
			}
		}
	}

	let impls = resolve_impls(syms, sources, pkgs, mods, types, module, span, type_, errs);
	let acquired = acquire_members(
		syms,
		sources,
		chunks,
		types,
		type_.name,
		span,
		&declared,
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

// Check that the structure of a type is valid. It must not have fields if it is
// a case parent, and it must declare params in the standard
// required-before-optional order.
fn check_structure(syms: &Interner, sources: &Sources, types: &Types) -> Vec<Error> {
	let mut errs = Vec::new();
	for id in types.ids() {
		let type_ = types.get_type(id);

		if let Err(err) =
			check_required_precede_optional(syms, sources, type_.span, &type_.ctor_fields)
		{
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
	type_: &syn::nodes::Type,
	errs: &mut Vec<Error>,
) -> Vec<ProtoId> {
	let mut impls = Vec::with_capacity(type_.impls.len());
	for path in &type_.impls {
		match mods.resolve_path_from(pkgs, module, path) {
			Some((pkg, owner, member, rest)) => {
				if !rest.is_empty() {
					errs.push(Error::NotAProtocol(
						sources.loc(span),
						syms.resolve_path(path),
					));
				}
				let Member::Proto(item_id) = member else {
					errs.push(Error::NotAProtocol(
						sources.loc(span),
						syms.resolve_path(path),
					));
					continue;
				};
				let owner_mods = if pkg == mods.pkg() {
					mods
				} else {
					&pkgs.get(pkg).modules
				};
				let owner_types = if pkg == mods.pkg() {
					types
				} else {
					&pkgs.get(pkg).types
				};
				impls.push(owner_types.get_proto_by_item(owner_mods.chunk(owner), item_id));
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
	chunks: &Chunks,
	types: &Types,
	type_name: Sym,
	type_span: Span,
	declared: &HashMap<Sym, MemberSite>,
	inherited: Option<&Inherited>,
	static_spans: &HashMap<Sym, Span>,
	impls: &[ProtoId],
	errs: &mut Vec<Error>,
) -> HashMap<Sym, MemberSite> {
	let mut acquired: HashMap<Sym, MemberSite> = HashMap::new();
	// The protocol each acquired member came from, kept only to name both
	// sides of a 'ProtocolConflict'.
	let mut from: HashMap<Sym, Sym> = HashMap::new();
	for proto_id in impls {
		let proto = types.get_proto(*proto_id);
		let proto_chunk = chunks.get(proto.chunk);
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
			let item = proto_chunk.get_proto_item(*item_id);
			let def = &item.def;

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
			if let Some(MemberSite::Declared(site_chunk, site_item)) = own {
				let site_chunk = chunks.get(*site_chunk);
				let TypeItem::Method(syn::nodes::Method::Instance(method_def)) =
					site_chunk.get_type_item(*site_item)
				else {
					panic!()
				};
				if !signatures_agree(proto_chunk, site_chunk, &def.params, &method_def.params) {
					errs.push(Error::SignatureMismatch(
						sources.loc(site_chunk.get_type_item_span(*site_item)),
						syms.resolve(type_name).to_string(),
						syms.resolve(proto.name).to_string(),
						syms.resolve(*member).to_string(),
					));
				}
				continue;
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

			if let Some(first) = from.insert(*member, proto.name) {
				errs.push(Error::ProtocolConflict(
					sources.loc(type_span),
					syms.resolve(first).to_string(),
					syms.resolve(proto.name).to_string(),
					syms.resolve(*member).to_string(),
				));
				continue;
			}

			acquired.insert(*member, MemberSite::Provided(proto.chunk, *item_id));
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
