#![allow(unused)]

use std::env;
use std::path::{Path, PathBuf};
use std::process;

use mesa::intern::Interner;
use mesa::load;
use mesa::pkg::{Package, Packages};
use mesa::rt::{self, build_prelude};
use mesa::sem;
use mesa::syn;

fn main() {
	let mut syms = Interner::new();
	let mut pkgs = Packages::new();

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
	load_package(&mut syms, &mut pkgs, &stdlib_dir);

	let current_dir = env::current_dir().unwrap();
	load_package(&mut syms, &mut pkgs, &current_dir);

	let prelude = build_prelude(&mut syms);

	match rt::eval(&mut syms, &pkgs, &prelude) {
		Ok(()) => {}
		Err((err, mut trace)) => {
			{
				let mut frame = trace.first_mut().unwrap();
				eprintln!("{}: runtime error: {}", frame.1, err);
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

fn load_package(syms: &mut Interner, pkgs: &mut Packages, start_dir: &Path) {
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

	let pkg_id = pkgs.reserve();

	let (modules, types) = match sem::check(syms, pkgs, pkg_id, &sources, &chunks) {
		Ok(checked) => checked,
		Err(errs) => {
			for err in errs {
				eprintln!("{}", err);
			}
			process::exit(1);
		}
	};

	pkgs.insert(
		pkg_id,
		Package {
			manifest,
			sources,
			chunks,
			modules,
			types,
		},
	);
}
