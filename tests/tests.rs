use std::env;
use std::fs;
use std::path::{Path, PathBuf};
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
	test_chars => "char.ms",
	test_strs => "str.ms",
	test_lists => "list.ms",
	test_dicts => "dict.ms",
	test_assigns_locals => "locals.ms",
	test_instantiates_types => "types.ms",
	test_accesses_fields => "fields.ms",
	test_calls_methods => "methods.ms",
	test_skips_comments => "comments.ms",
	test_honors_precedence => "prec.ms",
	test_performs_arithmetic => "arith.ms",
	test_performs_comparisons => "cmp.ms",
	test_performs_boolean_logic => "bool.ms",
	test_iterates => "each.ms",
	test_loops => "loop.ms",
	test_branches => "when.ms",
	test_matches => "match.ms",
	test_escapes_strings => "escapes.ms",
	test_walks_scope => "scope.ms",
	test_shadows_fields => "shadow.ms",
	test_bang_and_huh_names => "names.ms",
	test_calls_static_methods => "statics.ms",
	test_local_types => "local_types.ms",
	test_prelude_names => "prelude.ms",
	test_reports_runtime_errors => "rt_errors.ms",
	test_reports_prelude_shadow => "sem_error_prelude.ms",
	test_body_fields => "body_fields.ms",
	test_honors_significant_newlines => "newlines.ms",
	test_invokes_without_parens => "paren_less.ms",
	test_explicit_returns => "returns.ms",
	test_keyword_args_and_defaults => "kwargs.ms",
	test_reports_syntax_errors => "syn_errors.ms",
	test_reports_param_order => "sem_error_params.ms",
	test_reports_break_outside_loop => "sem_error_break.ms",
	test_mentions_without_invoking => "procs.ms",
	test_case_types => "case_types.ms",
	test_case_types_nested => "case_types_nested.ms",
	test_reports_case_errors => "case_errors.ms",
	test_reports_case_parent_shape => "sem_error_case.ms",
	test_protos => "protos.ms",
	test_reports_proto_errors => "sem_error_proto.ms",
	test_reports_multiple_sem_errors => "sem_error_multi.ms",
	test_reports_duplicate_members => "sem_error_member.ms",
}

#[test]
fn test_reports_errors_across_files() {
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			("package.ms", "module Test\n$print(\n"),
			("codec.ms", "module Test.Codec\ndef f(\n"),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stdout);
	assert_eq!(
		vec![
			"src/codec.ms:3,1: syntax error: unexpected token EOF",
			"src/package.ms:3,1: syntax error: unexpected token EOF",
		],
		stderr.lines().collect::<Vec<_>>()
	);
}

#[test]
fn test_reports_layout_defects() {
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			("package.ms", "module Test\n"),
			("codec.ms", "module Test.Codec\n"),
		],
	);
	fs::create_dir_all(test_dir.join("src/codec/json")).unwrap();
	fs::create_dir(test_dir.join("src/package")).unwrap();

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stdout);
	assert_eq!(
		vec![
			"src/codec/json: semantic error: directory has no sibling module file",
			"src/package: semantic error: 'src/package/' is reserved; the root module's children live in src/",
		],
		stderr.lines().collect::<Vec<_>>()
	);
}

#[test]
fn test_reports_module_header_defects() {
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			("package.ms", "module Test\n"),
			("bare.ms", "def f()\nend\n"),
			("late.ms", "def f()\nend\nmodule Test.Late\n"),
			("twice.ms", "module Test.Twice\nmodule Test.Again\n"),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stdout);
	assert_eq!(
		vec![
			"src/bare.ms: semantic error: file declares no module",
			"src/late.ms:3,1: semantic error: 'module' must be the first item in a file",
			"src/twice.ms:2,1: semantic error: file declares more than one module",
		],
		stderr.lines().collect::<Vec<_>>()
	);
}

#[test]
fn test_reports_misfiled_module() {
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			("package.ms", "module Test\n"),
			("codec.ms", "module Test.Codec\n"),
			("codec/json.ms", "module Test.Json\n"),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stdout);
	assert_eq!(
		vec![
			"src/codec/json.ms:1,1: semantic error: module 'Test.Json' must be declared under 'Test.Codec'",
		],
		stderr.lines().collect::<Vec<_>>()
	);
}

#[test]
fn test_reports_declaration_shadowing_child_module() {
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			("package.ms", "module Test\ntype Codec\nend\n"),
			("codec.ms", "module Test.Codec\n"),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stdout);
	assert_eq!(
		vec!["src/codec.ms:1,1: semantic error: duplicate member 'Codec'"],
		stderr.lines().collect::<Vec<_>>()
	);
}

#[test]
fn test_acquires_a_provided_member_across_files() {
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			(
				"package.ms",
				"module Test\n\
				 import Test.Order.Order\n\
				 type Task(priority)\n\
				 \timpl Order\n\
				 \tdef compare(other)\n\
				 \t\treturn self.priority - other.priority\n\
				 \tend\n\
				 end\n\
				 $print(Task(1).min(Task(5)).priority)\n\
				 $print(Task(5).min(Task(1)).priority)\n",
			),
			(
				"order.ms",
				"module Test.Order\n\
				 proto Order\n\
				 \tdef compare(other) end\n\
				 \tdef min(other)\n\
				 \t\twhen self.compare(other) <= 0 then return self else return other end\n\
				 \tend\n\
				 end\n",
			),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stderr);
	assert_eq!(vec!["1", "1"], stdout.lines().collect::<Vec<_>>());
}

fn assert_eval(file: &str, input: &str) {
	let test_dir = make_test_dir();

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

	let (stdout, stderr) = eval(dir, &src);
	assert_eq!(out, stdout, "stdout of {} case at line {}", file, start + 1);
	for line in err.lines() {
		assert!(
			stderr.contains(line),
			"stderr of {} case at line {}",
			file,
			start + 1
		);
	}
}

fn eval(dir: &Path, src: &str) -> (String, String) {
	let src = format!("module Test\n{}", src);
	write_package(dir, &[("package.ms", &src)]);
	run(dir)
}

fn make_test_dir() -> PathBuf {
	let temp_dir = env::temp_dir();
	let test_dir = temp_dir.join(format!("mesa-test-{:x}", rand::random::<u32>()));
	fs::create_dir(&test_dir).unwrap();
	test_dir
}

fn write_package(dir: &Path, files: &[(&str, &str)]) {
	let src_dir = dir.join("src");
	fs::create_dir_all(&src_dir).unwrap();
	fs::write(dir.join("package.toml"), "").unwrap();
	for (file, src) in files {
		let path = src_dir.join(file);
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(path, src).unwrap();
	}
}

fn run(dir: &Path) -> (String, String) {
	let bin = env!("CARGO_BIN_EXE_mesa");
	let output = Command::new(bin).current_dir(dir).output().unwrap();
	(
		String::from_utf8(output.stdout).unwrap(),
		String::from_utf8(output.stderr).unwrap(),
	)
}
