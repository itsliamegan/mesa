#![allow(unused)]

use std::env;
use std::process;

use mesa::intern::Interner;
use mesa::load;
use mesa::rt::Interpreter;
use mesa::sem;

fn main() {
	let (root_dir, _manifest) = match load::find() {
		Ok(found) => found,
		Err(err) => {
			eprintln!("{}", err);
			process::exit(1);
		}
	};

	let tree = match load::collect(&root_dir) {
		Ok(tree) => tree,
		Err(err) => {
			eprintln!("{}", err);
			process::exit(1);
		}
	};

	let mut syms = Interner::new();
	let pkg = match load::parse(&mut syms, tree.files) {
		Ok(pkg) => pkg,
		Err(errs) => {
			for err in errs {
				eprintln!("{}", err);
			}
			process::exit(1);
		}
	};

	let (modules, types) = match sem::check(&mut syms, &pkg, &tree.dirs) {
		Ok(checked) => checked,
		Err(errs) => {
			for err in errs {
				eprintln!("{}", err);
			}
			process::exit(1);
		}
	};

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
