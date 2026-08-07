#![allow(unused)]

mod intern;
mod rt;
mod syn;

use std::env;
use std::fs;
use std::process;

use crate::intern::Interner;
use crate::rt::Interpreter;
use crate::syn::{Lexer, Package, Parser};

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
		Ok(toks) => match Parser::new(&mut syms, src, toks).parse() {
			Ok(chunk) => match Interpreter::new(&mut syms, &pkg).eval(chunk) {
				Ok(()) => {}
				Err(err) => eprintln!("{}", err),
			},
			Err(err) => eprintln!("{}", err),
		},
		Err(err) => eprintln!("{}", err),
	}
}
