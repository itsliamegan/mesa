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
				assert_fixture_eval($file, include_str!($file));
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
	test_core_order => "core_order.ms",
	test_protocol_identity => "protocol_identity.ms",
	test_reports_runtime_errors => "rt_errors.ms",
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
	test_externs => "extern.ms",
	test_extern_counter => "extern_counter.ms",
	test_reports_proto_errors => "sem_error_proto.ms",
	test_reports_multiple_sem_errors => "sem_error_multi.ms",
	test_reports_duplicate_members => "sem_error_member.ms",
	test_modules => "modules.ms",
	test_packages => "packages.ms",
}

#[test]
fn test_reports_layout_defects() {
	let test_dir = make_test_dir();
	write_package(
		&test_dir,
		&[
			("package.ms", "module Test\n"),
			("codec.ms", "module Test.Codec\n"),
			("package/ext.ms", "module Test.Package.Ext\n"),
			("codec/json/encode.ms", "module Test.Codec.JSON.Encode\n"),
		],
	);
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

fn make_test_dir() -> PathBuf {
	let temp_dir = env::temp_dir();
	let test_dir = temp_dir.join(format!("mesa-test-{:x}", rand::random::<u32>()));
	fs::create_dir(&test_dir).unwrap();
	test_dir
}

fn write_package(dir: &Path, files: &[(impl AsRef<Path>, impl AsRef<[u8]>)]) {
	let src_dir = dir.join("src");
	fs::create_dir_all(&src_dir).unwrap();
	fs::write(
		dir.join("package.toml"),
		"name = \"test\"\nversion = \"0.1.0\"",
	)
	.unwrap();
	for (file, src) in files {
		let path = src_dir.join(file);
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(path, src).unwrap();
	}
}

fn run(dir: &Path) -> (String, String) {
	let bin = env!("CARGO_BIN_EXE_mesa");
	let output = Command::new(bin)
		.current_dir(dir)
		.env("MESA_HOME", env!("CARGO_MANIFEST_DIR"))
		.output()
		.unwrap();
	(
		String::from_utf8(output.stdout).unwrap(),
		String::from_utf8(output.stderr).unwrap(),
	)
}

struct Fixture {
	segments: Vec<Segment>,
}

struct Segment {
	files: Vec<(String, String)>,
	root: String,
	out: String,
	err: Vec<String>,
	start_line: usize,
}

fn trim_padding(s: String) -> String {
	let trimmed = s.trim_matches('\n');
	if trimmed.is_empty() {
		String::new()
	} else {
		format!("{}\n", trimmed)
	}
}

fn parse_fixture(src: &str) -> Fixture {
	let mut segments = Vec::new();
	let lines: Vec<&str> = src.lines().collect();
	let mut i = 0;

	while i < lines.len() {
		let start_line = i;
		let mut root = String::new();
		let mut files: Vec<(String, String)> = Vec::new();
		let mut out = String::new();
		let mut err = Vec::new();
		let mut in_file: Option<usize> = None;

		while i < lines.len() && lines[i] != "#---" {
			let line = lines[i];
			i += 1;

			if let Some(name) = line.strip_prefix("#:") {
				files.push((name.trim().to_string(), String::new()));
				in_file = Some(files.len() - 1);
			} else if let Some((_, text)) = line.split_once("#> ") {
				out.push_str(text);
				out.push('\n');
			} else if let Some((_, text)) = line.split_once("#! ") {
				err.push(text.to_string());
			} else {
				let target = match in_file {
					Some(idx) => &mut files[idx].1,
					None => &mut root,
				};
				target.push_str(line);
				target.push('\n');
			}
		}

		if i < lines.len() {
			i += 1;
		}

		segments.push(Segment {
			files: files
				.into_iter()
				.map(|(name, src)| (name, trim_padding(src)))
				.collect(),
			root: trim_padding(root),
			out,
			err,
			start_line,
		});
	}

	Fixture { segments }
}

fn write_segment_package(dir: &Path, segment: &Segment) {
	let src_dir = dir.join("src");
	if src_dir.exists() {
		fs::remove_dir_all(&src_dir).unwrap();
	}

	let mut package_files: Vec<(String, String)> = Vec::new();
	for (filename, src) in &segment.files {
		package_files.push((filename.clone(), src.clone()));
	}

	let package_src = format!("module Test\n{}", segment.root);
	package_files.push(("package.ms".to_string(), package_src));

	write_package(dir, &package_files);
}

fn assert_fixture_eval(file: &str, src: &str) {
	let fixture = parse_fixture(src);
	let test_dir = make_test_dir();

	for (i, segment) in fixture.segments.iter().enumerate() {
		write_segment_package(&test_dir, segment);
		let (stdout, stderr) = run(&test_dir);
		dbg!(&stderr);

		assert_eq!(
			segment.out,
			stdout,
			"stdout of {} case {} (line {})",
			file,
			i + 1,
			segment.start_line + 1
		);
		if segment.err.is_empty() {
			assert_eq!(
				"",
				stderr,
				"stderr of {} case {} (line {})",
				file,
				i + 1,
				segment.start_line + 1
			);
		}
		for err_line in &segment.err {
			assert!(
				stderr.contains(err_line.as_str()),
				"stderr of {} case {} (line {}) missing: {}\nactual stderr: {}",
				file,
				i + 1,
				segment.start_line + 1,
				err_line,
				stderr
			);
		}
	}

	fs::remove_dir_all(&test_dir).unwrap();
}
