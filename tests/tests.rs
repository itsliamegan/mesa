use std::env;
use std::fs;
use std::process::Command;

use rand;

#[test]
fn test_parses_literals() {
	assert_eval(include_str!("lits.ms"));
}

#[test]
fn test_strs() {
	assert_eval(include_str!("str.ms"));
}

#[test]
fn test_lists() {
	assert_eval(include_str!("list.ms"));
}

#[test]
fn test_dicts() {
	assert_eval(include_str!("dict.ms"));
}

#[test]
fn test_assigns_locals() {
	assert_eval(include_str!("locals.ms"));
}

#[test]
fn test_calls_procs() {
	assert_eval(include_str!("procs.ms"));
}

#[test]
fn test_instantiates_types() {
	assert_eval(include_str!("types.ms"));
}

#[test]
fn test_accesses_fields() {
	assert_eval(include_str!("fields.ms"));
}

#[test]
fn test_calls_methods() {
	assert_eval(include_str!("methods.ms"));
}

#[test]
fn test_skips_comments() {
	assert_eval(include_str!("comments.ms"));
}

#[test]
fn test_honors_precedence() {
	assert_eval(include_str!("prec.ms"));
}

#[test]
fn test_performs_arithmetic() {
	assert_eval(include_str!("arith.ms"));
}

#[test]
fn test_performs_comparisons() {
	assert_eval(include_str!("cmp.ms"));
}

#[test]
fn test_performs_boolean_logic() {
	assert_eval(include_str!("bool.ms"));
}

#[test]
fn test_iterates() {
	assert_eval(include_str!("each.ms"));
}

#[test]
fn test_branches() {
	assert_eval(include_str!("when.ms"));
}

#[test]
fn test_escapes_strings() {
	assert_eval(include_str!("escapes.ms"));
}

#[test]
fn test_walks_scope() {
	assert_eval(include_str!("scope.ms"));
}

#[test]
fn test_shadows_fields() {
	assert_eval(include_str!("shadow.ms"));
}

fn assert_eval(input: &str) {
	let mut output = String::new();
	for line in input.lines() {
		if let Some((_, text)) = line.split_once("#> ") {
			output.push_str(text);
			output.push('\n');
		}
	}
	assert_eq!(output, eval(input));
}

fn eval(input: &str) -> String {
	let suffix = rand::random::<u32>();
	let temp_dir = env::temp_dir();
	let temp_file = temp_dir.join(format!("main.{:x}.ms", suffix));
	fs::write(&temp_file, input).unwrap();

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
