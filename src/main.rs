#![allow(unused)]

use std::env;
use std::path::{Path, PathBuf};
use std::process;

use mesa::intern::Interner;
use mesa::load;
use mesa::pkg::{Package, Packages};
use mesa::rt::Interpreter;
use mesa::sem;

fn main() {
	let mut syms = Interner::new();
	let mut pkgs = Packages::new();

	let home_dir = env::home_dir().unwrap();
	let mesa_home = match env::var("MESA_HOME") {
		Ok(path) => PathBuf::from(path),
		Err(_) => home_dir.join(".mesa"),
	};
	if !mesa_home.exists() {
		eprintln!("error: cannot locate standard library");
		process::exit(1);
	}
	let stdlib_dir = mesa_home.join("lib");
	let (stdlib_dir, manifest) = match load::find(&stdlib_dir) {
		Ok(found) => found,
		Err(err) => {
			eprintln!("[Core] {}", err);
			process::exit(1);
		}
	};
	let sources = match load::collect(&stdlib_dir) {
		Ok(sources) => sources,
		Err(err) => {
			eprintln!("[Core] {}", err);
			process::exit(1);
		}
	};

	let chunks = match load::parse(&mut syms, &sources) {
		Ok(chunks) => chunks,
		Err(errs) => {
			for err in errs {
				eprintln!("[Core] {}", err);
			}
			process::exit(1);
		}
	};

	let (modules, types) = match sem::check(&mut syms, &sources, &chunks) {
		Ok(checked) => checked,
		Err(errs) => {
			for err in errs {
				eprintln!("[Core] {}", err);
			}
			process::exit(1);
		}
	};

	let current_dir = env::current_dir().unwrap();
	let (root_dir, manifest) = match load::find(&current_dir) {
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

	let chunks = match load::parse(&mut syms, &sources) {
		Ok(chunks) => chunks,
		Err(errs) => {
			for err in errs {
				eprintln!("{}", err);
			}
			process::exit(1);
		}
	};

	let (modules, types) = match sem::check(&mut syms, &sources, &chunks) {
		Ok(checked) => checked,
		Err(errs) => {
			for err in errs {
				eprintln!("{}", err);
			}
			process::exit(1);
		}
	};

	let pkg = Package::new(manifest, sources, chunks);

	match Interpreter::new(&mut syms, &pkg, &types, &modules).eval() {
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
