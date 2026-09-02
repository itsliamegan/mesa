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
	test_lexical => "lexical.ms",
	test_num => "num.ms",
	test_bool => "bool.ms",
	test_equal => "equal.ms",
	test_str => "str.ms",
	test_list => "list.ms",
	test_dict => "dict.ms",
	test_type => "type.ms",
	test_control_flow => "control_flow.ms",
	test_errors => "errors.ms",
	test_scope => "scope.ms",
	test_reports_syntax_errors => "syn_error.ms",
	test_proc => "proc.ms",
	test_case_type => "case_type.ms",
	test_proto => "proto.ms",
	test_extern => "extern.ms",
	test_module => "module.ms",
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
