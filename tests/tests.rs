use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use rand;

macro_rules! test_files {
	($($name:ident => $file:literal,)*) => {
		$(
			#[test]
			fn $name() {
				assert_eval($file, include_str!($file));
			}
		)*
	};
}

test_files! {
	test_parses_literals => "lits.ms",
	test_strs => "str.ms",
	test_lists => "list.ms",
	test_dicts => "dict.ms",
	test_assigns_locals => "locals.ms",
	test_calls_procs => "procs.ms",
	test_instantiates_types => "types.ms",
	test_accesses_fields => "fields.ms",
	test_calls_methods => "methods.ms",
	test_skips_comments => "comments.ms",
	test_honors_precedence => "prec.ms",
	test_performs_arithmetic => "arith.ms",
	test_performs_comparisons => "cmp.ms",
	test_performs_boolean_logic => "bool.ms",
	test_iterates => "each.ms",
	test_branches => "when.ms",
	test_escapes_strings => "escapes.ms",
	test_walks_scope => "scope.ms",
	test_shadows_fields => "shadow.ms",
	test_bang_and_huh_names => "names.ms",
	test_calls_static_methods => "statics.ms",
	test_prelude_names => "prelude.ms",
	test_reports_runtime_errors => "rt_errors.ms",
	test_reports_prelude_shadow => "sem_error_prelude.ms",
	test_body_fields => "body_fields.ms",
	test_honors_significant_newlines => "newlines.ms",
	test_explicit_returns => "returns.ms",
}

fn assert_eval(file: &str, input: &str) {
	let temp_dir = env::temp_dir();
	let test_dir = temp_dir.join(format!("mesa-test-{:x}", rand::random::<u32>()));
	fs::create_dir(&test_dir).unwrap();

	let lines = input.lines().collect::<Vec<_>>();
	let mut start = 0;
	while start < lines.len() {
		let mut end = start;
		while end < lines.len() && !lines[end].starts_with("#---") {
			end += 1;
		}
		assert_case(&test_dir, file, &lines, start, end);
		start = end + 1;
	}

	fs::remove_dir_all(&test_dir).unwrap();
}

fn assert_case(dir: &Path, file: &str, lines: &[&str], start: usize, end: usize) {
	let mut src = String::new();
	let mut out = String::new();
	let mut err = String::new();
	for (i, line) in lines.iter().enumerate() {
		if i >= start && i < end {
			src.push_str(line);
			if let Some((_, text)) = line.split_once("#> ") {
				out.push_str(text);
				out.push('\n');
			} else if let Some((_, text)) = line.split_once("#! ") {
				err.push_str(text);
				err.push('\n');
			}
		}
		src.push('\n');
	}

	let (stdout, stderr) = eval(dir, file, &src);
	assert_eq!(out, stdout, "stdout of {} case at line {}", file, start + 1);
	assert_eq!(err, stderr, "stderr of {} case at line {}", file, start + 1);
}

fn eval(dir: &Path, file: &str, src: &str) -> (String, String) {
	fs::write(dir.join(file), src).unwrap();

	let bin = env!("CARGO_BIN_EXE_mesa");
	let output = Command::new(bin)
		.arg(file)
		.current_dir(dir)
		.output()
		.unwrap();

	(
		String::from_utf8(output.stdout).unwrap(),
		String::from_utf8(output.stderr).unwrap(),
	)
}
