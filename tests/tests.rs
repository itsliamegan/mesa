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

#[test]
fn test_reaches_a_def_and_a_var_through_a_module_form_import() {
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			(
				"package.ms",
				"module Test\nimport Test.Utils\n$print(Utils.count)\n$print(Utils.helper())\n",
			),
			(
				"utils.ms",
				"module Test.Utils\ncount := 3\ndef helper()\n\treturn 9\nend\n",
			),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stderr);
	assert_eq!(vec!["3", "9"], stdout.lines().collect::<Vec<_>>());
}

#[test]
fn test_uses_a_member_form_import_as_a_plain_value() {
	// The only other cross-file fixture uses a member-form import solely as
	// an 'impl' target; this reaches the same proto and prints it directly.
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			(
				"package.ms",
				"module Test\nimport Test.Protos.Order\n$print(Order)\n",
			),
			(
				"protos.ms",
				"module Test.Protos\nproto Order\n\tdef compare(other) end\nend\n",
			),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stderr);
	assert_eq!(vec!["proto Order"], stdout.lines().collect::<Vec<_>>());
}

#[test]
fn test_reaches_a_child_module_nobody_imported() {
	// Only 'Test.Codec' is imported; 'Codec.Json' is reachable as a member of
	// the module actually imported, not through an import of its own.
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			(
				"package.ms",
				"module Test\nimport Test.Codec\n$print(Codec.Json.decode(4))\n",
			),
			("codec.ms", "module Test.Codec\n"),
			(
				"codec/json.ms",
				"module Test.Codec.Json\ndef decode(x)\n\treturn x + 1\nend\n",
			),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stderr);
	assert_eq!(vec!["5"], stdout.lines().collect::<Vec<_>>());
}

#[test]
fn test_reads_a_module_member_live_rather_than_a_snapshot() {
	// A member read goes straight to the owning module's scope each time, so
	// a call that mutates a top-level var is visible on the very next read.
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			(
				"package.ms",
				"module Test\nimport Test.Counter\nCounter.bump()\n$print(Counter.n)\n",
			),
			(
				"counter.ms",
				"module Test.Counter\nn := 0\ndef bump()\n\tn := n + 1\nend\n",
			),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stderr);
	assert_eq!(vec!["1"], stdout.lines().collect::<Vec<_>>());
}

#[test]
fn test_refuses_to_write_a_module_member() {
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			(
				"package.ms",
				"module Test\nimport Test.Codec\nCodec.limit := 5\n",
			),
			("codec.ms", "module Test.Codec\nlimit := 8\n"),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stdout);
	assert!(
		stderr.contains("member 'limit' on module Test.Codec is read-only"),
		"stderr: {}",
		stderr
	);
}

#[test]
fn test_reports_a_missing_module_member_against_the_modules_path() {
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			(
				"package.ms",
				"module Test\nimport Test.Codec\n$print(Codec.missing)\n",
			),
			("codec.ms", "module Test.Codec\n"),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stdout);
	assert!(
		stderr.contains("module Test.Codec has no such member 'missing'"),
		"stderr: {}",
		stderr
	);
}

#[test]
fn test_does_not_leak_an_imported_module_as_a_member() {
	// codec.ms imports Test.Utils for its own use; that must not make
	// 'Utils' reachable as a member of Codec from anywhere else.
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			(
				"package.ms",
				"module Test\nimport Test.Codec\n$print(Codec.Utils)\n",
			),
			("codec.ms", "module Test.Codec\nimport Test.Utils\n"),
			("utils.ms", "module Test.Utils\n"),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stdout);
	assert!(
		stderr.contains("module Test.Codec has no such member 'Utils'"),
		"stderr: {}",
		stderr
	);
}

#[test]
fn test_prints_a_module_value() {
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			(
				"package.ms",
				"module Test\nimport Test.Codec\n$print(Codec)\n",
			),
			("codec.ms", "module Test.Codec\n"),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stderr);
	assert_eq!(
		vec!["module Test.Codec"],
		stdout.lines().collect::<Vec<_>>()
	);
}

#[test]
fn test_reports_an_import_cycle_against_the_whole_cycle() {
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			("package.ms", "module Test\n"),
			("a.ms", "module Test.A\nimport Test.B\n"),
			("b.ms", "module Test.B\nimport Test.A\n"),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stdout);
	assert_eq!(
		vec!["src/b.ms:2,1: semantic error: import cycle: Test.A → Test.B → Test.A"],
		stderr.lines().collect::<Vec<_>>()
	);
}

#[test]
fn test_evaluates_every_module_in_import_order() {
	// D is a dependency of both B and C, so it prints first; the root imports
	// nothing and never claimed a print, so only the four children show up,
	// in dependency-before-dependent order.
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			("package.ms", "module Test\n"),
			(
				"a.ms",
				"module Test.A\nimport Test.B\nimport Test.C\n$print(\"A\")\n",
			),
			("b.ms", "module Test.B\nimport Test.D\n$print(\"B\")\n"),
			("c.ms", "module Test.C\nimport Test.D\n$print(\"C\")\n"),
			("d.ms", "module Test.D\n$print(\"D\")\n"),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stderr);
	assert_eq!(vec!["D", "B", "C", "A"], stdout.lines().collect::<Vec<_>>());
}

#[test]
fn test_calls_a_sibling_proc_in_a_non_root_module() {
	// Every module's own top level now runs, not just the root's, so a proc
	// in a non-root file can call another declared in that same file.
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			("package.ms", "module Test\n"),
			(
				"codec.ms",
				"module Test.Codec\n\
				 def helper()\n\
				 \treturn 1\n\
				 end\n\
				 def main()\n\
				 \treturn helper()\n\
				 end\n\
				 $print(main())\n",
			),
		],
	);

	let (stdout, stderr) = run(&test_dir);
	fs::remove_dir_all(&test_dir).unwrap();

	assert_eq!("", stderr);
	assert_eq!(vec!["1"], stdout.lines().collect::<Vec<_>>());
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
