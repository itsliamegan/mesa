use std::env;
use std::fs;
use std::process::Command;

use rand;

#[test]
fn test_parses_literals() {
	assert_eq!(include_str!("lits.out"), eval(include_str!("lits.in")));
}

#[test]
fn test_assigns_locals() {
	assert_eq!(include_str!("locals.out"), eval(include_str!("locals.in")));
}

#[test]
fn test_calls_procs() {
	assert_eq!(include_str!("procs.out"), eval(include_str!("procs.in")));
}

#[test]
fn test_instantiates_types() {
	assert_eq!(include_str!("types.out"), eval(include_str!("types.in")));
}

#[test]
fn test_accesses_fields() {
	assert_eq!(include_str!("fields.out"), eval(include_str!("fields.in")));
}

#[test]
fn test_calls_methods() {
	assert_eq!(
		include_str!("methods.out"),
		eval(include_str!("methods.in"))
	);
}

fn eval(src: &str) -> String {
	let suffix = rand::random::<u32>();
	let temp_dir = env::temp_dir();
	let temp_file = temp_dir.join(format!("main.{:x}.ms", suffix));
	fs::write(&temp_file, src).unwrap();

	let (stdout, stderr) = (|| {
		let bin = env!("CARGO_BIN_EXE_mesa");
		let output = Command::new(bin).arg(&temp_file).output().unwrap();
		(
			String::from_utf8(output.stdout).unwrap(),
			String::from_utf8(output.stderr).unwrap(),
		)
	})();

	fs::remove_file(temp_file).unwrap();

	if stdout.is_empty() { stderr } else { stdout }
}
