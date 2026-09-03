#![allow(unused)]

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process;

use mesa::intern::Interner;
use mesa::load;
use mesa::pkg::{PackageId, Packages};
use mesa::rt::{
	self, CORE_TYPES, NativeTypeSpec, Natives, Raise, Raised, build_errors, build_prelude,
	inspect_val,
};
use mesa::sem;
use mesa::sem::protos::build_behaviors;
use mesa::syn;

// The native types the stdlib provides: Rust code linked into this binary, and
// the implementations that the stdlib's 'extern type' declarations name.
const STDLIB_NATIVE_TYPES: &[NativeTypeSpec] = rt::STDLIB_NATIVE_TYPES;

fn main() {
	let mut syms = Interner::new();
	let mut pkgs = Packages::new();
	let mut natives = Natives::new();

	let stdlib_dir = match env::var("MESA_HOME") {
		Ok(path) => PathBuf::from(path).join("lib"),
		Err(_) => {
			let home = env::home_dir().unwrap().join(".mesa").join("lib");
			if home.exists() {
				home
			} else {
				PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib")
			}
		}
	};
	let stdlib_pkg_id = load_package(
		&mut syms,
		&mut pkgs,
		&mut natives,
		&stdlib_dir,
		&[CORE_TYPES, STDLIB_NATIVE_TYPES],
		None,
	);

	let current_dir = env::current_dir().unwrap();
	let current_pkg_id = load_package(
		&mut syms,
		&mut pkgs,
		&mut natives,
		&current_dir,
		&[],
		Some(stdlib_pkg_id),
	);

	let prelude = match build_prelude(&pkgs, &mut syms, stdlib_pkg_id, &natives) {
		Ok(prelude) => prelude,
		Err(err) => {
			eprintln!("error: {}", err);
			process::exit(1);
		}
	};
	let behaviors = build_behaviors(&mut syms, &pkgs, stdlib_pkg_id);
	let errors = build_errors(&pkgs, &mut syms, stdlib_pkg_id);
	let mut rt = rt::Runtime::new(&pkgs, natives, behaviors, errors);

	match rt::eval(&mut syms, &mut rt, &prelude) {
		Ok(()) => {}
		Err(Raise(err, mut trace)) => {
			{
				let message = match &err {
					Raised::Native(err) => err.message(&syms, &rt),
					Raised::Val(val) => inspect_val(&syms, &rt, val),
				};
				let mut frame = trace.first_mut().unwrap();
				eprintln!("{}: runtime error: {}", frame.1, message);
			}
			{
				let mut frame = trace.last_mut().unwrap();
				frame.0.push_str("<main>");
			}
			for (proc_name, loc) in trace {
				eprintln!("\tat {} ({})", proc_name, loc);
			}
			process::exit(1);
		}
	}
}

fn load_package(
	syms: &mut Interner,
	pkgs: &mut Packages,
	natives: &mut Natives,
	start_dir: &Path,
	native_type_lists: &[&[NativeTypeSpec]],
	stdlib: Option<PackageId>,
) -> PackageId {
	let (root_dir, manifest) = match load::find(start_dir) {
		Ok(found) => found,
		Err(err) => {
			eprintln!("{}", err);
			process::exit(1);
		}
	};

	let sources = match load::collect(&root_dir) {
		Ok(sources) => sources,
		Err(err) => {
			eprintln!("{}", err);
			process::exit(1);
		}
	};

	let chunks = match syn::parse(syms, &sources) {
		Ok(chunks) => chunks,
		Err(errs) => {
			for err in errs {
				eprintln!("{}", err);
			}
			process::exit(1);
		}
	};

	let pkg_id = pkgs.open(manifest, sources, chunks);

	let mut native_shapes = HashMap::new();
	for specs in native_type_lists {
		native_shapes.extend(natives.register(syms, specs));
	}

	match sem::check(syms, pkgs, pkg_id, &native_shapes, stdlib.unwrap_or(pkg_id)) {
		Ok(()) => {}
		Err(errs) => {
			for err in errs {
				eprintln!("{}", err);
			}
			process::exit(1);
		}
	};

	pkg_id
}
