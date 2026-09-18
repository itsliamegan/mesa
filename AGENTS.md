# AGENTS.md

## Project

This repository contains the implementation for a dynamically & strongly typed
scripting language, Mesa. The implementation is hand-written in Rust and uses
few external dependencies.

- `src/` contains the Rust interpreter
- `tests/` contains test fixtures and an orchestration script to run fixture
  files as Rust integration tests
- `doc/` contains documentation on the the language, its tooling, its library,
  and a style guide for writing Mesa code
- `lib/` contains the Mesa standard library
- `pad/` contains a dummy Mesa package for experimenting with language behavior
- `.agents/` contains agent-specific files and configuration

## Commands

- Build: `cargo build`
- Run: `cargo run` to run the package in the current directory
- Test: `cargo test` to run all, `cargo test <fixture>` to run one fixture;
  fixtures listed in `tests/tests.rs`
- Format: `cargo fmt`; ensure `cargo fmt --check` is clean before considering
  any work done

## Workflow

When working on a feature, bug fix, or documentation change, follow this process:

1. Draft a step by step plan and ensuring edge cases are thoroughly considered
2. Create a branch off of `main` and switch to the branch to do the work
3. While working, make WIP commits along the way, keeping commit messages lean
4. If the user requests manual review, stop between WIP commits and let the user
   provide comments and direct file edits, steering the design. If the user
   makes edits, rewrite the WIP commit in-place
3. If the user requests a remote workflow, push the WIP commits to `origin`
4. Once finished, squash into a single commit with a user-provided message and
   rebase `main` on top of the branch
5. Push `main` to `origin` and clean up the local & remote branches

Avoid this planning & branching process if either a) the file you're working on is untracked or b) the user specifically requests a branchless or planless workflow on `main`.

If the user points you to an already-drafted plan, ensure it is correct before executing.

If you need a scratch Mesa package for testing, use the one in `pad/`. Clean up
your work after experimenting.

## Testing

The interpreter is tested exclusively by language fixture files. These are all
the `*.ms` files in the `tests/` directory. The fixtures are grouped by language
feature.

Each fixture contains expected standard output in `#>` lines and expected error
output in `#!` lines. A separate fixture section, run in isolation from the
rest, can be created with a `#---` delimiter, and a `#:` can specify a file for
the current section.

When adding a feature or fixing a bug, add a test to one of these files or (in
rare instances) add a new file. These files all use a common cast of data types,
whose specifics are flexible, so as to keep conceptual overhead minimal. Prefer
to use one of these types, or define one in the same universe, as opposed to
inventing a new one.

## Documentation

`doc/language.md` contains a detailed description of the language and its
features. Consult it to understand how a language feature works, and treat it as
the source of truth. If there is a conflict between the implementation and this
file, the implementation has a bug.

`doc/implementation.md` contains a detailed description of the interpreter,
including its phase structure and architecture conventions. Consult it to
understand where to make changes in the interpreter and how its different pieces
fit together.
