# Mesa Implementation Manual

This document provides a high-level overview of the implementation of the Mesa
programming language.

## Introduction

Mesa is implemented in Rust as a tree-walking interpreter. Unlike most dynamic language interpreters, most of the shape of a Mesa program can be determined before the program is run—types have fixed fields,

## Phases

The interpreter is split into four sequential phases: loading, parsing, semantic
analysis, and runtime evaluation. These phases correspond to the `load`, `syn`,
`sem`, and `rt` modules in the Rust source respectively.

Each of these phases can only depend on abstractions from those which come
before it; `rt` depends on `sem`, but `sem` cannot depend on `rt`.

### Loading

Every package begins evaluation by being loaded. The package loader first reads
& parses the package's `package.toml` file. Then it walks the package's `src/`
tree, storing the directories and files as a `src::Sources`.

### Parsing

Once a package is loaded, its `src::Sources` are parsed into `syn::Chunk`s. A
`Chunk` contains all the information for a particular `src::Source` in dense
arenas. AST nodes are stored as as `syn::nodes::Nodes`, grouped by node type,
while tokens and trivia are stored as `syn::lex::Tokens` and
`syn::lex::Trivias`.

A `Chunk` completely represents the parsed result of a given `Source`, which
also corresponds with a particular module in the language sense.

The parser is split into two phases, lexing and parsing; `syn::lex` corresponds
to the former and `syn::parse` to the latter.

#### Lexing

The lexer walks character by character. It produces two arenas. `Tokens` holds
the significant tokens, and is the only arena in the entire interpreter that is
a struct of arrays rather than an array of structs. `Trivias` holds comments and
newlines between nodes.

Every identifier is stored with an associated symbol. Strings and characters are
lexed including escape characters, and there is no string interpolation.

#### Parsing

The parser is a simple hand-written LL(1) parser using a combination of
recursive descent and Pratt precedence climbing. The parser consumes tokens in
sequence until it reaches EOF.

Every syntactic element is given a specific `parse_` function. These should
roughly correspond to the grammar listing in Appendix A of `doc/language.md`.

The `Precedence` struct represents a simple lookup table of tokens to their precedence level. It is consulted in a standard Pratt parsing loop to determine whether a given operator should let the parsing continue or not.

Parsing produces a single `Chunk` for a given set of `Tokens` and `Trivia`,
which it builds up incrementally.

### Analysis

Once a package is parsed into a `Chunk`, it is analyzed to check its adherance
to the language rules. This is by and large the most complex of the phases; it both enforces the semantic invariants of the language and provides a foundation that the runtime builds on top of.

#### Modules

The first phase of analysis builds the module graph. The file tree is walked to
create a file for every module and to check that the module hierarchy
corresponds with the directory hierarchy. Module imports are checked to ensure
they do not form a cycle.

#### Protos

#### Types

### Evaluation

#### `Val` and `Obj`

#### `Scope`, `Member`, and `Local`

## Patterns

### Arenas and Handles

The interpreter makes frequent use of dense **arenas** combined with opaque
**handles** in different contexts to simultaneously preserve cache locality and
avoid borrow-checking headaches.

An arena `Arena<Record>` holds a `Vec` of records and provides an `add(&mut
self, record: Record) -> Record::Id` method. The `Record::Id` struct provides a
handle which points back into a record in the arena, which can be retrieved
using a `get(&self, id: Record::Id) -> &Record` or `get_mut(&mut self, id:
Record::Id) -> &mut Record` method.

The handle ID is an integer rather than an actual reference, so there is no
explicit lifetime management required. This means that such IDs must be
carefully ensured to only ever refer to live records; attempting to retrieve a
record from an arena with an expired or otherwise invalid ID results in a
runtime panic.

A handle stores an index into the arena's records as a `u32` rather than a
`usize`, and provides `index(&self) -> usize` and `from_index(index: usize) ->
Self` methods to cast between the forms. While this is technically incorrect, as
a `usize` is the type that represents an index, using a `u32` instead provides
more than sufficient capacity for arena records while massively reducing memory
usage.

### Interning

The interpreter interns all names in a single symbol interner; `intern` contains
the symbol arena, `Interner` as well as the symbol handle itself, `Sym`. The
reserved words are loaded into the interner on startup, so they have fixed
positions, while all program symbols are loaded when first encountered and
looked up by hash afterwards.

### Matching and Conditionals

### Naming

Names of variables, functions, fields, and parameters are abbreviated where
conventions outside this repository exist (`expr`, `tok`) or where they match up
with a language feature (`proto`). Otherwise, the word is spelled out.

An abbreviated name keeps its form across singular and plural, so `pkg` pairs
with `pkgs` and `desc` with `descs`, never with an elongated plural.

If a reasonable name conflicts with another name in scope, use Rust's local
rebinding if the earlier one isn't needed after declaring the later one. If it
is needed, add a categorical suffix like `_id` or `_expr` to distinguish.

Names of types are not abbreviated (`NativeTypeShape`, `MemberSite`).

Names of modules and files use abbreviated names (`src`, `syn`, `sem`).

### Lifetimes and Borrowing

Passing a value by reference is preferred over unnecessary copying in most
cases, but this necessitates explicitly annotating many reference lifetimes
which can become unwieldy. Lifetimes are named after the variable they borrow; a
`pkg` variable is declared `&'pkg pkg` rather than `&'a pkg`.
