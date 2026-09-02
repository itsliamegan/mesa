use std::collections::{HashMap, HashSet};

use ordermap::OrderMap;

use crate::intern::{Interner, Sym};
use crate::pkg::{PackageId, Packages};
use crate::sem::modules::ModuleId;
use crate::sem::protos::{self, Behaviors, ProtoId};
use crate::sem::{Error, check_params};
use crate::src::{Sources, Span};
use crate::syn::nodes::{Def, DefId, Expr, ExprId, Lit, ModuleItem, ModuleItemId, Param, TypeItem};
use crate::syn::{self, Chunk, ChunkId};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TypeId(u32);

impl TypeId {
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

#[derive(Debug)]
pub struct Types {
	types: Vec<Type>,
	// Where a type's declaration lives, so the walk that binds its name arrives
	// at the description built for it. Only top-level types are keyed here; a
	// nested one is reached as a static of the type enclosing it.
	type_by_item: HashMap<(ChunkId, ModuleItemId), TypeId>,
}

#[derive(Debug)]
pub enum Type {
	User(UserType),
	Native(NativeType),
}

impl Type {
	pub fn name(&self) -> Sym {
		match self {
			Type::User(type_) => type_.name,
			Type::Native(type_) => type_.name,
		}
	}

	pub fn methods(&self) -> &HashMap<Sym, MethodSite> {
		match self {
			Type::User(type_) => &type_.methods,
			Type::Native(type_) => &type_.methods,
		}
	}

	pub fn statics(&self) -> &HashMap<Sym, Static> {
		match self {
			Type::User(type_) => &type_.statics,
			Type::Native(type_) => &type_.statics,
		}
	}

	pub fn impls(&self) -> &[(PackageId, ProtoId)] {
		match self {
			Type::User(type_) => &type_.impls,
			Type::Native(type_) => &type_.impls,
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
	pub methods: HashMap<Sym, MethodSite>,
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
	pub native_methods: HashMap<Sym, Vec<NativeParam>>,
	pub native_statics: HashMap<Sym, Vec<NativeParam>>,
	// Every member reachable on a value of this type, flattened: Rust's, the
	// declaration's, and what it acquires from the protocols it implements.
	pub methods: HashMap<Sym, MethodSite>,
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
	pub methods: HashMap<Sym, Vec<NativeParam>>,
	pub statics: HashMap<Sym, Vec<NativeParam>>,
}

// A body field's initializer. Carries a span for more precise error reporting.
#[derive(Debug)]
pub struct Field {
	pub span: Span,
	pub init: ExprId,
}

// The declaration implementing a member. Includes a reference to the
// implementer's chunk, which may differ for e.g. an acquired protocol method.
#[derive(Debug, Clone, Copy)]
pub enum MethodSite {
	// A member declared explicitly by an implementing type.
	Declared(ChunkId, DefId),
	// A member provided by a protocol declaration, possibly in another package.
	Provided(PackageId, ChunkId, DefId),
	// An 'extern type' member implemented in Rust. It has no chunk or item to
	// point at; the declaration names the implementation in the native type's
	// member map.
	Native,
	// A member no declaration names and no protocol body provides.
	Derived,
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
	declared: HashMap<Sym, MethodSite>,
	acquired: HashMap<Sym, MethodSite>,
}

impl Types {
	fn new() -> Self {
		Self {
			types: Vec::new(),
			type_by_item: HashMap::new(),
		}
	}

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
}

// Describe every type in the package, resolving 'impl' names and deciding
// conformance as it goes. Unresolved names and conformance failures are
// reported here rather than assumed impossible.
pub fn check(
	syms: &mut Interner,
	pkgs: &Packages,
	pkg_id: PackageId,
	natives: &HashMap<Sym, NativeTypeShape>,
) -> (Types, Vec<Error>) {
	let pkg = pkgs.get(pkg_id);
	let mut types = Types::new();
	let mut errs = Vec::new();

	for module in pkg.modules().ids() {
		let chunk_id = pkg.modules().chunk(module);
		let chunk = pkg.chunks().get(chunk_id);
		for item_id in &chunk.top {
			let span = chunk.get_module_item_span(*item_id);
			match chunk.get_module_item(*item_id) {
				ModuleItem::Type(type_) => {
					let id = describe_type(
						syms, pkgs, pkg_id, &mut types, module, chunk_id, span, type_, None, None,
						false, &mut errs,
					);
					types.type_by_item.insert((chunk_id, *item_id), id);
				}
				ModuleItem::Extern(extern_) => {
					if let Some(id) = describe_extern(
						syms, pkgs, pkg_id, &mut types, natives, module, chunk_id, span, extern_,
						&mut errs,
					) {
						types.type_by_item.insert((chunk_id, *item_id), id);
					}
				}
				_ => continue,
			}
		}
	}

	errs.append(&mut check_structure(syms, pkgs, pkg_id, &types));

	(types, errs)
}

// Describe a type declaration, producing a description that reduces all later
// member resolution to a single table lookup. Resolve the type's 'impl' names
// and decide conformance against each.
fn describe_type(
	syms: &mut Interner,
	pkgs: &Packages,
	pkg_id: PackageId,
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
	let pkg = pkgs.get(pkg_id);
	let chunk = pkg.chunks().get(chunk_id);
	// Claim the id before the body is walked so that a nested type can name the
	// type enclosing it while that type is still being described.
	let id = TypeId(types.types.len() as u32);
	types.types.push(Type::User(UserType {
		name: type_.name,
		chunk: chunk_id,
		span,
		ctor_fields: type_.params.to_vec(),
		body_fields: OrderMap::new(),
		methods: HashMap::new(),
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
	// different maps but share one namespace: all three are what 'instance.x'
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
			// A variant's methods *must* be resolved after their parent's
			// methods because they inherit some of them from their parent.
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
				declared.insert(name, MethodSite::Declared(chunk_id, *def_id));
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
				pkg.sources().loc(item_span),
				syms.resolve(type_.name).to_string(),
				syms.resolve(name).to_string(),
			));
		}
	}

	let direct = protos::resolve_impls(
		syms,
		pkgs,
		pkg_id,
		pkg.protos(),
		module,
		span,
		&type_.impls,
		errs,
	);
	let impls = protos::expand_impls(pkgs, pkg_id, pkg.protos(), &direct);
	let acquired = acquire_methods(
		syms,
		pkgs,
		pkg_id,
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
	// accounted for by acquire_methods. They stay as-is so that 'methods' reads
	// clearly as "these four tiers, most specific wins".
	// Every type has the built-in behaviors that are automatic rather than
	// opted into, below all four tiers.
	let mut methods = HashMap::new();
	methods.insert(syms.intern(Behaviors::EQUAL.1), MethodSite::Derived);
	methods.insert(syms.intern(Behaviors::INSPECT.1), MethodSite::Derived);
	if let Some(inherited) = inherited {
		methods.extend(&inherited.acquired);
	}
	methods.extend(&acquired);
	if let Some(inherited) = inherited {
		methods.extend(&inherited.declared);
	}
	methods.extend(&declared);

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
					pkgs,
					pkg_id,
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
					pkgs,
					pkg_id,
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
	described.methods = methods;
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
	syms: &mut Interner,
	pkgs: &Packages,
	pkg_id: PackageId,
	types: &mut Types,
	natives: &HashMap<Sym, NativeTypeShape>,
	module: ModuleId,
	chunk_id: ChunkId,
	span: Span,
	extern_: &syn::nodes::Extern,
	errs: &mut Vec<Error>,
) -> Option<TypeId> {
	let pkg = pkgs.get(pkg_id);
	let chunk = pkg.chunks().get(chunk_id);
	// Resolved before the implementation is looked up: an unresolvable protocol
	// is a mistake in the declaration either way, and reporting it does not
	// depend on there being anything to describe.
	let direct = protos::resolve_impls(
		syms,
		pkgs,
		pkg_id,
		pkg.protos(),
		module,
		span,
		&extern_.impls,
		errs,
	);
	let impls = protos::expand_impls(pkgs, pkg_id, pkg.protos(), &direct);

	// A declaration with no backing implementation is an implementation error,
	// and leaves nothing to describe it into.
	let Some(native) = natives.get(&extern_.name) else {
		errs.push(Error::UnimplementedExtern(
			pkg.sources().loc(span),
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
					.insert(name, MethodSite::Declared(chunk_id, *def_id))
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
					pkg.sources(),
					&native.methods,
					extern_.name,
					def,
					item_span,
					errs,
				);
				let claimed = declared.insert(def.name, MethodSite::Native).is_some();
				(def.name, claimed)
			}
			syn::nodes::ExternItem::Native(syn::nodes::Method::Static(def_id)) => {
				let def = chunk.get_def(*def_id);
				check_native_signature(
					syms,
					pkg.sources(),
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
				pkg.sources().loc(item_span),
				syms.resolve(extern_.name).to_string(),
				syms.resolve(name).to_string(),
			));
		}
	}

	// The registration is the source of truth for what a member does, and the
	// declaration for whether it exists; an implementation the body does not
	// name is reachable from no mesa source. Reported at the type, which is
	// the only span there is — the registration has none.
	for name in native.methods.keys() {
		if let Some(MethodSite::Native) = declared.get(name) {
			continue;
		}
		if collided.contains(name) {
			continue;
		}
		errs.push(Error::UndeclaredNativeMember(
			pkg.sources().loc(span),
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
			pkg.sources().loc(span),
			syms.resolve(extern_.name).to_string(),
			syms.resolve(*name).to_string(),
		));
	}

	let acquired = acquire_methods(
		syms,
		pkgs,
		pkg_id,
		extern_.name,
		span,
		&declared,
		&native.methods,
		None,
		&static_spans,
		&impls,
		errs,
	);

	let mut methods = HashMap::new();
	methods.insert(syms.intern(Behaviors::EQUAL.1), MethodSite::Derived);
	methods.insert(syms.intern(Behaviors::INSPECT.1), MethodSite::Derived);
	methods.extend(&acquired);
	methods.extend(&declared);

	let id = TypeId(types.types.len() as u32);
	types.types.push(Type::Native(NativeType {
		name: extern_.name,
		chunk: chunk_id,
		span,
		provider: native.provider,
		native_methods: native.methods.clone(),
		native_statics: native.statics.clone(),
		methods,
		statics,
		impls,
	}));
	Some(id)
}

// Check that the structure of a type is valid. It must not have fields if it is
// a case parent, and it must declare params in the standard
// required-before-optional order.
fn check_structure(
	syms: &Interner,
	pkgs: &Packages,
	pkg_id: PackageId,
	types: &Types,
) -> Vec<Error> {
	let pkg = pkgs.get(pkg_id);
	let mut errs = Vec::new();
	for id in types.ids() {
		// A native type has no ctor params or body fields of its own to order
		// or forbid: its declaration only names an implementation.
		let Type::User(type_) = types.get_type(id) else {
			continue;
		};

		if let Err(err) = check_params(syms, pkg.sources(), type_.span, &type_.ctor_fields) {
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
				pkg.sources().loc(type_.span),
				syms.resolve(type_.name).to_string(),
			));
		}
		if let Some((name, field)) = type_.body_fields.iter().next() {
			errs.push(Error::FieldOnCaseParent(
				pkg.sources().loc(field.span),
				syms.resolve(type_.name).to_string(),
				syms.resolve(*name).to_string(),
			));
		}
	}
	errs
}

// Determine every member that a type acquires from the protocols it implements,
// deciding conformance against each protocol's methods as it goes. Flattened to
// just the *actual* protocol provisions, because every conformance error is a
// statement about what the flattening would otherwise produce.
fn acquire_methods(
	syms: &Interner,
	pkgs: &Packages,
	pkg_id: PackageId,
	type_name: Sym,
	type_span: Span,
	declared: &HashMap<Sym, MethodSite>,
	native_methods: &HashMap<Sym, Vec<NativeParam>>,
	inherited: Option<&Inherited>,
	static_spans: &HashMap<Sym, Span>,
	impls: &[(PackageId, ProtoId)],
	errs: &mut Vec<Error>,
) -> HashMap<Sym, MethodSite> {
	let pkg = pkgs.get(pkg_id);
	let mut acquired: HashMap<Sym, MethodSite> = HashMap::new();
	// The protocol each acquired member came from, kept so the next provider
	// of the same name can ask whether it is related to this one.
	let mut from: HashMap<Sym, (PackageId, ProtoId)> = HashMap::new();
	for (proto_pkg, proto_id) in impls {
		let proto = pkgs.get(*proto_pkg).protos().get_proto(*proto_id);
		let proto_chunk = pkgs.get(*proto_pkg).chunks().get(proto.chunk);
		// Which map a member came from already answers where its implementation
		// would come from. A required member has no site of its own; the type
		// must supply one.
		let required = proto
			.required
			.iter()
			.map(|(member, item)| (member, item, None));
		let provided = proto.provided.iter().map(|(member, item)| {
			let site = MethodSite::Provided(*proto_pkg, proto.chunk, *item);
			(member, item, Some(site))
		});
		let derived = proto
			.derived
			.iter()
			.map(|(member, item)| (member, item, Some(MethodSite::Derived)));
		for (member, item_id, site) in required.chain(provided).chain(derived) {
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
				Some(MethodSite::Declared(site_chunk, site_def)) => {
					let site_chunk = pkg.chunks().get(*site_chunk);
					let method_def = site_chunk.get_def(*site_def);
					if !signatures_agree(proto_chunk, site_chunk, &def.params, &method_def.params) {
						errs.push(Error::SignatureMismatch(
							pkg.sources().loc(site_chunk.get_def_span(*site_def)),
							syms.resolve(type_name).to_string(),
							syms.resolve(proto.name).to_string(),
							syms.resolve(*member).to_string(),
						));
					}
					continue;
				}
				Some(MethodSite::Provided(..)) => panic!(),
				Some(MethodSite::Native) => {
					let params = &native_methods[member];
					if !native_signature_agrees(&def.params, params) {
						errs.push(Error::SignatureMismatch(
							pkg.sources().loc(type_span),
							syms.resolve(type_name).to_string(),
							syms.resolve(proto.name).to_string(),
							syms.resolve(*member).to_string(),
						));
					}
					continue;
				}
				// The implementation is provided by the language, and thus
				// always has the correct signature.
				Some(MethodSite::Derived) => continue,
				None => {}
			}

			let Some(site) = site else {
				errs.push(Error::MissingMethod(
					pkg.sources().loc(type_span),
					syms.resolve(type_name).to_string(),
					syms.resolve(proto.name).to_string(),
					syms.resolve(*member).to_string(),
				));
				continue;
			};

			if let Some(static_span) = static_spans.get(member) {
				errs.push(Error::MethodCollision(
					pkg.sources().loc(*static_span),
					syms.resolve(type_name).to_string(),
					syms.resolve(proto.name).to_string(),
					syms.resolve(*member).to_string(),
				));
				continue;
			}

			if let Some(first) = from.insert(*member, (*proto_pkg, *proto_id)) {
				let first_proto = pkgs.get(first.0).protos().get_proto(first.1);
				// Related protocols do not conflict; the nearer body refines
				// the farther one, and the traversal order guarantees this
				// write is the nearer. Unrelated ones do, and the type
				// resolves it by declaring the member.
				let related = first_proto.impls.contains(&(*proto_pkg, *proto_id))
					|| proto.impls.contains(&first);
				if !related {
					errs.push(Error::ProtocolConflict(
						pkg.sources().loc(type_span),
						syms.resolve(first_proto.name).to_string(),
						syms.resolve(proto.name).to_string(),
						syms.resolve(*member).to_string(),
					));
					continue;
				}
			}

			acquired.insert(*member, site);
		}
	}
	acquired
}

fn signatures_agree(
	proto_chunk: &Chunk,
	type_chunk: &Chunk,
	proto_params: &[Param],
	type_params: &[Param],
) -> bool {
	if proto_params.len() != type_params.len() {
		return false;
	}
	for (proto_param, type_param) in proto_params.iter().zip(type_params) {
		if proto_param.name != type_param.name {
			return false;
		}
		if !defaults_agree(
			proto_chunk,
			type_chunk,
			proto_param.default,
			type_param.default,
		) {
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
	proto_default: Option<ExprId>,
	type_default: Option<ExprId>,
) -> bool {
	let (proto_default, type_default) = match (proto_default, type_default) {
		(Some(proto_default), Some(type_default)) => (proto_default, type_default),
		(None, None) => return true,
		_ => return false,
	};
	// Only literals are compared; anything else is taken to agree until there
	// is a structural comparison over the arena.
	match (
		proto_chunk.get_expr(proto_default),
		type_chunk.get_expr(type_default),
	) {
		(Expr::Lit(proto_lit), Expr::Lit(type_lit)) => lits_agree(proto_lit, type_lit),
		_ => true,
	}
}

fn lits_agree(proto_lit: &Lit, type_lit: &Lit) -> bool {
	match (proto_lit, type_lit) {
		(Lit::Str(proto_lit), Lit::Str(type_lit)) => proto_lit == type_lit,
		(Lit::Char(proto_lit), Lit::Char(type_lit)) => proto_lit == type_lit,
		(Lit::Num(proto_lit), Lit::Num(type_lit)) => proto_lit == type_lit,
		(Lit::Bool(proto_lit), Lit::Bool(type_lit)) => proto_lit == type_lit,
		(Lit::List(..), _) | (_, Lit::List(..)) => true,
		(Lit::Dict(..), _) | (_, Lit::Dict(..)) => true,
		(Lit::Nil, Lit::Nil) => true,
		_ => false,
	}
}
