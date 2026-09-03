use std::collections::{HashMap, HashSet};

use ordermap::OrderMap;

use crate::intern::{Interner, Sym};
use crate::pkg::{PackageId, Packages};
use crate::sem::modules::{Member, ModuleId, Resolved};
use crate::sem::{Error, Visit};
use crate::src::Span;
use crate::syn::nodes::{DefId, ModuleItem, ModuleItemId, ProtoItem};
use crate::syn::{self, Chunk, ChunkId};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ProtoId(u32);

impl ProtoId {
	pub fn index(self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug)]
pub struct Protos {
	protos: Vec<Proto>,
	// Where a protocol's declaration lives, so an 'impl' name resolved through
	// the module map arrives at the description built for it.
	proto_by_item: HashMap<(ChunkId, ModuleItemId), ProtoId>,
	// Which protocol a proto item's def belongs to, so a walk that reaches a
	// def without going through its declaring 'Proto' can still name the
	// protocol and look up whether the item is required or provided.
	proto_by_proto_item: HashMap<(ChunkId, DefId), ProtoId>,
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
	pub derived: OrderMap<Sym, DefId>,
	// Every protocol reachable through this one's prerequisites, itself
	// included, ordered so that a protocol always follows everything it
	// requires.
	pub impls: Vec<(PackageId, ProtoId)>,
	// Every member name declared anywhere in 'impls'.
	pub effective: HashSet<Sym>,
}

// One built-in behavior: the protocol governing it, and the method carrying it.
#[derive(Debug)]
pub struct Behavior {
	pub proto: (PackageId, ProtoId),
	pub method: Sym,
}

// Access is one behavior with separate read and write methods.
#[derive(Debug)]
pub struct AccessBehavior {
	pub proto: (PackageId, ProtoId),
	pub access: Sym,
	pub store: Sym,
}

// Every built-in behavior the interpreter dispatches through.
#[derive(Debug)]
pub struct Behaviors {
	pub equal: Behavior,
	pub hash: Behavior,
	pub order: Behavior,
	pub display: Behavior,
	pub inspect: Behavior,
	pub append: Behavior,
	pub access: AccessBehavior,
}

impl Behaviors {
	// The protocol governing each built-in behavior and the method it dispatches
	// through, both fixed by the language.
	pub const EQUAL: (&str, &str) = ("Equal", "equal");
	pub const HASH: (&str, &str) = ("Hash", "hash");
	pub const ORDER: (&str, &str) = ("Order", "order");
	pub const DISPLAY: (&str, &str) = ("Display", "display");
	// Automatic: every type has this method without opting in, so the type pass
	// seeds it long before the protocols can be resolved.
	pub const INSPECT: (&str, &str) = ("Inspect", "inspect");
	pub const APPEND: (&str, &str) = ("Append", "append");
	pub const ACCESS: (&str, &str, &str) = ("Access", "access", "store");
}

// Resolve the protocols of 'Core.Behaviors' and the method each behavior
// dispatches through. A missing or misnamed protocol is a broken build.
pub fn build_behaviors(syms: &mut Interner, pkgs: &Packages, stdlib: PackageId) -> Behaviors {
	let path = [syms.intern("Core"), syms.intern("Behaviors")];
	let Some(module) = pkgs.get(stdlib).modules().by_path(&path) else {
		panic!("stdlib does not declare a 'Core.Behaviors' module");
	};
	Behaviors {
		equal: build_behavior(syms, pkgs, stdlib, module, Behaviors::EQUAL),
		hash: build_behavior(syms, pkgs, stdlib, module, Behaviors::HASH),
		order: build_behavior(syms, pkgs, stdlib, module, Behaviors::ORDER),
		display: build_behavior(syms, pkgs, stdlib, module, Behaviors::DISPLAY),
		inspect: build_behavior(syms, pkgs, stdlib, module, Behaviors::INSPECT),
		append: build_behavior(syms, pkgs, stdlib, module, Behaviors::APPEND),
		access: build_access_behavior(syms, pkgs, stdlib, module, Behaviors::ACCESS),
	}
}

fn build_behavior(
	syms: &mut Interner,
	pkgs: &Packages,
	stdlib: PackageId,
	module: ModuleId,
	names: (&str, &str),
) -> Behavior {
	let (proto, method) = names;
	let pkg = pkgs.get(stdlib);
	let Some(Member::Proto(item_id)) = pkg.modules().member(module, syms.intern(proto)) else {
		panic!(
			"stdlib does not declare a 'Core.Behaviors.{}' protocol",
			proto
		);
	};
	let chunk_id = pkg.modules().chunk(module);
	Behavior {
		proto: (stdlib, pkg.protos().get_proto_by_item(chunk_id, item_id)),
		method: syms.intern(method),
	}
}

fn build_access_behavior(
	syms: &mut Interner,
	pkgs: &Packages,
	stdlib: PackageId,
	module: ModuleId,
	names: (&str, &str, &str),
) -> AccessBehavior {
	let (proto, access, store) = names;
	let pkg = pkgs.get(stdlib);
	let Some(Member::Proto(item_id)) = pkg.modules().member(module, syms.intern(proto)) else {
		panic!(
			"stdlib does not declare a 'Core.Behaviors.{}' protocol",
			proto
		);
	};
	let chunk_id = pkg.modules().chunk(module);
	AccessBehavior {
		proto: (stdlib, pkg.protos().get_proto_by_item(chunk_id, item_id)),
		access: syms.intern(access),
		store: syms.intern(store),
	}
}

impl Protos {
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
		for member_item in proto
			.required
			.values()
			.chain(proto.provided.values())
			.chain(proto.derived.values())
		{
			self.proto_by_proto_item.insert((chunk, *member_item), id);
		}
		self.protos.push(proto);
		self.proto_by_item.insert((chunk, item_id), id);
		id
	}
}

pub fn check(syms: &Interner, pkgs: &Packages, pkg_id: PackageId) -> (Protos, Vec<Error>) {
	let pkg = pkgs.get(pkg_id);
	let mut protos = Protos {
		protos: Vec::new(),
		proto_by_item: HashMap::new(),
		proto_by_proto_item: HashMap::new(),
	};
	let mut errs = Vec::new();

	// Describe protocols first, package-wide, so a type can implement one
	// declared in any file.
	for module in pkg.modules().ids() {
		let chunk_id = pkg.modules().chunk(module);
		let chunk = pkg.chunks().get(chunk_id);
		for item_id in &chunk.top {
			if let ModuleItem::Proto(proto) = chunk.get_module_item(*item_id) {
				let span = chunk.get_module_item_span(*item_id);
				let desc = describe_proto(chunk, chunk_id, span, proto);
				protos.add_proto(chunk_id, *item_id, desc);
			}
		}
	}
	// Resolve after every protocol is described, so a prerequisite may be
	// declared in any file of the package, and after every dependency package
	// is built, so a prerequisite may cross a package boundary.
	let mut impls: Vec<Vec<(PackageId, ProtoId)>> = vec![Vec::new(); protos.protos.len()];
	for module in pkg.modules().ids() {
		let chunk_id = pkg.modules().chunk(module);
		let chunk = pkg.chunks().get(chunk_id);
		for item_id in &chunk.top {
			if let ModuleItem::Proto(proto) = chunk.get_module_item(*item_id) {
				let span = chunk.get_module_item_span(*item_id);
				let id = protos.get_proto_by_item(chunk_id, *item_id);
				impls[id.index()] = resolve_impls(
					syms,
					pkgs,
					pkg_id,
					&protos,
					module,
					span,
					&proto.impls,
					&mut errs,
				);
			}
		}
	}
	errs.append(&mut close_protos(syms, pkgs, pkg_id, &mut protos, &impls));
	errs.append(&mut check_protos(syms, pkgs, pkg_id, &protos));

	(protos, errs)
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
	let mut derived = OrderMap::new();
	for item in &proto.items {
		match item {
			ProtoItem::Declared(def_id) => {
				let def = chunk.get_def(*def_id);
				// A proc is provided if its def block is not empty, otherwise
				// it is required.
				if !chunk.get_block(def.body).exprs.is_empty() {
					provided.insert(def.name, *def_id);
				} else {
					required.insert(def.name, *def_id);
				}
			}
			// A derived member has an empty block just as a required one does,
			// so only the variant tells the two apart.
			ProtoItem::Derived(def_id) => {
				derived.insert(chunk.get_def(*def_id).name, *def_id);
			}
		}
	}
	Proto {
		name: proto.name,
		chunk: chunk_id,
		span,
		required,
		provided,
		derived,
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
	pkgs: &Packages,
	pkg_id: PackageId,
	protos: &Protos,
) -> Vec<Error> {
	let pkg = pkgs.get(pkg_id);
	let mut errs = Vec::new();
	for id in protos.proto_ids() {
		let proto = protos.get_proto(id);
		// A protocol satisfies the rule if anything it requires, *directly or
		// through a prerequisite*, has a required member.
		let has_requirement = proto.impls.iter().any(|(pkg, id)| {
			let owner = owner_protos(pkgs, pkg_id, protos, *pkg);
			!owner.get_proto(*id).required.is_empty()
		});
		if !proto.provided.is_empty() && !has_requirement {
			errs.push(Error::ProvidedWithoutRequired(
				pkg.sources().loc(proto.span),
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
	pkgs: &Packages,
	pkg_id: PackageId,
	protos: &mut Protos,
	impls: &[Vec<(PackageId, ProtoId)>],
) -> Vec<Error> {
	let mut errs = Vec::new();
	let mut visits = vec![Visit::Unseen; protos.protos.len()];
	let mut path = Vec::new();
	let ids: Vec<ProtoId> = protos.proto_ids().collect();
	for id in ids {
		close_proto(
			syms,
			pkgs,
			pkg_id,
			protos,
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
	pkgs: &Packages,
	pkg_id: PackageId,
	protos: &mut Protos,
	impls: &[Vec<(PackageId, ProtoId)>],
	id: ProtoId,
	visits: &mut [Visit],
	path: &mut Vec<ProtoId>,
	errs: &mut Vec<Error>,
) {
	let pkg = pkgs.get(pkg_id);
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
				pkg.sources().loc(protos.get_proto(id).span),
				render_proto_cycle(syms, protos, path, *dep_id),
			));
			continue;
		}
		close_proto(
			syms, pkgs, pkg_id, protos, impls, *dep_id, visits, path, errs,
		);
	}
	// A cut back edge leaves its protocol's closure empty, contributing
	// nothing here, which is what keeps a reported cycle from expanding
	// forever. Nothing else can reach back to this protocol, so it is never
	// already present in its own expansion.
	let mut closure = expand_impls(pkgs, pkg_id, protos, direct);
	closure.push((pkg_id, id));

	path.pop();
	visits[id.index()] = Visit::Done;
	let effective = effective_members(pkgs, pkg_id, protos, &closure);
	let proto = &mut protos.protos[id.index()];
	proto.impls = closure;
	proto.effective = effective;
}

// Every member name a closure declares, required or provided, which is exactly
// what a body written in any of those protocols may reach for on 'self'.
fn effective_members(
	pkgs: &Packages,
	pkg_id: PackageId,
	protos: &Protos,
	closure: &[(PackageId, ProtoId)],
) -> HashSet<Sym> {
	let mut effective = HashSet::new();
	for (pkg, id) in closure {
		let proto = owner_protos(pkgs, pkg_id, protos, *pkg).get_proto(*id);
		effective.extend(proto.required.keys().copied());
		effective.extend(proto.provided.keys().copied());
		effective.extend(proto.derived.keys().copied());
	}
	effective
}

// Render a protocol cycle as the walk found it, e.g. 'Hash → Equal → Hash',
// rather than naming only the back edge.
fn render_proto_cycle(syms: &Interner, protos: &Protos, path: &[ProtoId], back: ProtoId) -> String {
	let start = path.iter().position(|id| *id == back).unwrap();
	let mut names: Vec<String> = path[start..]
		.iter()
		.map(|id| syms.resolve(protos.get_proto(*id).name).to_string())
		.collect();
	names.push(syms.resolve(protos.get_proto(back).name).to_string());
	names.join(" → ")
}

// The 'Protos' a package reference points into. The package being checked has
// not published its protocols yet, so a reference to it reads the arena being built.
fn owner_protos<'protos>(
	pkgs: &'protos Packages,
	checked: PackageId,
	protos: &'protos Protos,
	owner: PackageId,
) -> &'protos Protos {
	match owner == checked {
		true => protos,
		false => pkgs.get(owner).protos(),
	}
}

// Resolve the protocols an 'impl' line names against the symbols that the
// declaring file binds. A name that fails to resolve contributes no 'ProtoId'.
pub(super) fn resolve_impls(
	syms: &Interner,
	pkgs: &Packages,
	pkg_id: PackageId,
	protos: &Protos,
	module: ModuleId,
	span: Span,
	paths: &[Vec<Sym>],
	errs: &mut Vec<Error>,
) -> Vec<(PackageId, ProtoId)> {
	let pkg = pkgs.get(pkg_id);
	let mut impls = Vec::with_capacity(paths.len());
	for path in paths {
		match pkg.modules().resolve_path_from(pkgs, module, path) {
			Some(Resolved::Member(owner_pkg, owner, Member::Proto(item_id))) => {
				let id = owner_protos(pkgs, pkg_id, protos, owner_pkg)
					.get_proto_by_item(pkgs.get(owner_pkg).modules().chunk(owner), item_id);
				// One protocol named twice in one clause is a typo with no
				// reading that the author meant. Implication is silent, but
				// repetition is not: a name reached through a prerequisite is a
				// choice, a name written twice is a slip. Compared by id rather
				// than by path, so two spellings of one protocol still collide.
				if impls.contains(&(owner_pkg, id)) {
					errs.push(Error::DuplicateImpl(
						pkg.sources().loc(span),
						syms.resolve_path(path),
					));
					continue;
				}
				impls.push((owner_pkg, id));
			}
			Some(Resolved::Member(..) | Resolved::Module(..) | Resolved::Partial(..)) => {
				errs.push(Error::NotAProtocol(
					pkg.sources().loc(span),
					syms.resolve_path(path),
				));
			}
			None => {
				errs.push(Error::UnknownProtocol(
					pkg.sources().loc(span),
					syms.resolve_path(path),
				));
			}
		}
	}
	impls
}

// Expand resolved prerequisites into everything they imply. The return value
// contains no duplicates and is ordered so that a prerequisite's own
// implication comes before it. This latter guarantee ensures that, when
// iterating this result, a body of a nearer impl can refine a farther one.
pub(super) fn expand_impls(
	pkgs: &Packages,
	pkg_id: PackageId,
	protos: &Protos,
	direct: &[(PackageId, ProtoId)],
) -> Vec<(PackageId, ProtoId)> {
	let mut expanded = Vec::new();
	for (pkg, id) in direct {
		let owner = owner_protos(pkgs, pkg_id, protos, *pkg);
		expanded.extend(owner.get_proto(*id).impls.iter().copied());
	}
	let mut seen = HashSet::with_capacity(expanded.len());
	expanded.retain(|item| seen.insert(*item));
	expanded
}
