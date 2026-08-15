#![allow(unused)]

use std::process;

use mesa::intern::Interner;
use mesa::rt::Interpreter;
use mesa::sem::{self, load};

fn main() {
	let root_dir = match load::find() {
		Ok(root_dir) => root_dir,
		Err(err) => {
			eprintln!("{}", err);
			process::exit(1);
		}
	};

	let files = match load::collect(&root_dir) {
		Ok(files) => files,
		Err(err) => {
			eprintln!("{}", err);
			process::exit(1);
		}
	};

	let mut syms = Interner::new();
	let (pkg, root_chunk_id) = match load::parse(&mut syms, files) {
		Ok(parsed) => parsed,
		Err(errs) => {
			for err in errs {
				eprintln!("{}", err);
			}
			process::exit(1);
		}
	};

	if let Err(errs) = sem::check(&mut syms, &pkg) {
		for err in errs {
			eprintln!("{}", err);
		}
		process::exit(1);
	}

	match Interpreter::new(&mut syms, &pkg).eval(root_chunk_id) {
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
