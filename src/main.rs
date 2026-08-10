#![allow(unused)]

use std::env;
use std::fs;
use std::process;

use mesa::intern::Interner;
use mesa::rt::Interpreter;
use mesa::syn::{Lexer, Package, Parser};

fn main() {
	let args = env::args().collect::<Vec<_>>();
	if args.len() != 2 {
		eprintln!("usage: mesa <file>");
		process::exit(1);
	}
	let file = args[1].clone();
	let Ok(text) = fs::read_to_string(&file) else {
		eprintln!("error: cannot read file '{}'", file);
		process::exit(1);
	};

	let mut syms = Interner::new();
	let mut pkg = Package::new();
	let src_id = pkg.add_src(file, text);
	let src = pkg.get_src(src_id);

	match Lexer::new(&mut syms, src).lex() {
		Ok(toks) => match Parser::new(src, toks).parse() {
			Ok(chunk) => match Interpreter::new(&mut syms, &pkg).eval(chunk) {
				Ok(()) => {}
				Err((err, loc)) => {
					eprintln!("{}: runtime error: {}", loc, err);
					process::exit(1);
				}
			},
			Err(err) => {
				eprintln!("{}", err);
				process::exit(1);
			}
		},
		Err(err) => {
			eprintln!("{}", err);
			process::exit(1);
		}
	}
}
