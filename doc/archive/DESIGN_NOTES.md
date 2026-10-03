# Mesa design notes — a thinking session

> **Archived 2026-10-03.** This document is no longer maintained. It records
> the reasoning behind design decisions as they were made, and many of its
> conclusions have since been revised or superseded. `doc/language.md` is the
> source of truth for the language and `doc/implementation.md` for the
> interpreter.

This is not a spec and not a set of decisions. It's a record of working
through how mesa's stated design values interact with each other and with
what's already built, in the spirit of a logic game: given these constraints,
what follows, and where do the constraints actually pull against each other.

Most of it stays close to the current implementation (`src/syn.rs`,
`src/rt.rs`, `src/intern.rs`). Some goes further out; that's flagged where it
happens. It's grounded in what's already been worked out in prior sessions
(the sym table refactor, GC-by-another-name, the native-data feature, and the
multi-file/module design) so this extends that thinking rather than
re-litigating it.

**On confidence.** Every behavioral claim below about what mesa does *today*
was checked by running the interpreter against a probe script, not inferred
from reading. Where something is a guess about intent, or an extrapolation
past what exists, it says so. Where two options look genuinely balanced, it
says that too rather than manufacturing a winner.

*Amended.* One claim did not hold up: §4's truthiness finding was inferred
rather than probed, and was wrong (see the correction there, and §0.3 item
22). The failure mode is worth naming, since it is the one this document is
most exposed to — a probe was run, but it tested the uncontested half of the
claim and left the contested half unexamined. Read "verified" as "a probe was
run," not as "the probe discriminated." The re-check that found this covered
the §0.2 table, every `rt.rs` line citation, and the claims about prefix
operators; it did not re-probe every behavioural claim in §§1–12, so the
others carry their original confidence, not a renewed one.

*Later corrections are marked where they occur* rather than collected here.
The recurring pattern is worth knowing: the errors have been claims of
exhaustiveness — "the only fixture that does," "the only forcing function,"
"uniformly `Not*`" — each true when written and each falsified by a later
addition rather than by having been wrong. Treat any superlative in this
document as scoped to the section that makes it.

**Structure.** §§1–12 are the first pass, written before any of it was
discussed. §13 records the directions taken in response and works through
what they open, including a protocol system the first pass didn't anticipate.
§14 designs paren-less calls in full. §15 covers the move to significant
newlines, which invalidates several arguments made earlier. §16 settles proc
literals, which closes the block question §6.2 opened and answers §0.3's item
3 from an unexpected direction. §17 settles case types, which touch four more
items on the working list. §18 settles the nil policy, which was the item
everything else was waiting on. §19 redesigns the built-in error taxonomy,
§20 collects six working-list resolutions, and §21 adds static methods, local
types, and keyword arguments. §22 settles strings, the largest functional
gap, and adds the only type since the core set. §23 closes three smaller
items outright, §24 closes seven threads that had been left inside items
already marked closed, §25 settles the two printing modes, and §26 adds fields
a type declares for itself rather than taking from its caller. §27 closes the
last of the large open items by removing visibility entirely, §28 makes modules
values and their graph acyclic, and §29 gives the native boundary an error
channel. §30 reverses §16.2's escape prohibition — the document's largest
revision — by making proc literals capture by value. §31 gives protocols a
declaration form and §32 settles the set at nine, which empties the working
list. §33 steps outside the language to price the resolver, which nine separate
decisions have been quietly adding jobs to.

**Read §0 first.** It's the working index: what's settled (§0.1), what's
accepted but unbuilt (§0.2), the original working list (§0.3, now fully struck
through), and — in §0.4 — what's still unfinished, which is the only live list
left. The sections after it are the reasoning, kept
because the *why* is often the useful part, but §0 is authoritative wherever
they disagree.

---

# 0. Where things stand

## 0.1 Settled

Decisions taken, with pointers to the full reasoning.

### Language surface

**Strings** (§22, closing item 15 and most of item 14). Immutable, because
`Str` hashes by content and so cannot opt out of item 9's mutable-key hazard.
`size` counts characters, which closes item 15 by making `Str` agree with
`List` and `Dict` rather than by renaming anything. A new immediate type
**`Char`** holds one validated scalar and is spelled `'a'` — the ninth prelude
name, taken knowingly, because a one-scalar `Str` costs two allocations. `Str`
is **not indexable**: `s.chars` returns a fresh `List` of `Char` and is
deliberately not cached, iteration yields `Char`s without materializing
anything, and `s.slice(from, to)` stands in for substrings until a `Range` type
exists. `+` concatenates two `Str`s and nothing else, hardcoded per item 13.
Deferred: interpolation, which item 14 narrows to and which is now purely a
question of spelling, since §22.5 settled concatenation and §25 settled the
display form; byte access, which waits for an opaque `Bytes` type; and `Range`.

**Fields declared in the type body** (§26). `name := expr` at the top of a type
body declares a per-instance field the constructor doesn't take — evaluated at
construction in declaration order, able to reference constructor fields and
earlier body fields. Position is the whole rule: in the parens means the caller
supplies it, in the body means the caller can't. They are ordinary public
members otherwise, part of `Eq` and printed in the debug form as `name: value`.
No `self.x := …` type-level constant, since `:=` has no free slot the way `def`
did, and no reassignment of a constructor field. It gives `rt.rs:579`'s
`todo!()` a meaning, completing the pair §21.2 started.

**Two printing modes** (§25, closing item 12). `Display` for someone reading
output, `Inspect` for someone reading values, with the verbs `display` and
`inspect` — named for real in §39, where the `Debug` placeholder was dropped. No
round-trip guarantee, declined rather than aspired to, since procs, types, and
cyclic values have no literal form. Containers print their elements in the
inspect form, which is the concrete reason one mode can't work. Cycles print as
`...`. `Inspect` is automatic and structural for every user type, `Display`
opted into with `impl` and falling back to `Inspect`, mirroring item 9's
`Equal`/`Hash` shape.

**Structural equality is coinductive, and hashing is bounded** (§24.7, closing
item 9). Comparison carries a set of the object *pairs* under comparison and
assumes equality on revisit, so a cyclic value compares in finite time and
`==` stays total — sharing is invisible, and `a.x := a` equals a two-node
cycle. Hashing can't use the same trick, so it stops after a fixed number of
nodes, and the bound may not read identity or reentry, since equal values must
hash equally. *Extended by §49*, which settles the half item 9 never scoped: **`List` and
`Dict` compare structurally too, and are unhashable.** Forced rather than chosen —
a body field holds whatever it holds, so the first user type carrying a list
decides what `==` on a list means. Unhashability is what keeps the mutable-key
hazard visible at a declaration for values that have no declaration site, and it
keeps the bound a plain counter: §49.2 shows that canonically truncating an
unordered collection is impossible, so a hashable dict would have cost O(n) in
entries or collided on every same-size dict, with nothing in between. The
identity-and-reentry rule generalises with it — the bound may read only what
equality can see, of which identity is one instance and insertion order another.

**Mesa is a blocking language** (§24.1, closing item 26). No concurrency
design, deliberately, and now decided rather than merely absent — the runtime
already assumes single-threaded execution throughout, and later work can stop
asking whether it has to accommodate anything else.

**Numbers stay `f64`, and indices are validated** (§23.1, closing item 17). No
`Int` or `Float`, and modulo goes unwritten until something needs it. A list
index must be a whole non-negative number, so `list[1.7]` and `list[-1]` raise
instead of truncating — negatives reserved so that Python-style negative
indexing and ranges stay a pure extension rather than a change of meaning.
`IndexError` becomes a group of `OutOfRange` and `NonIntegral`, with a wrong
*type* of subscript staying `TypeError.IndexNonNum`; dict keys are exempt from
all of it, being keys rather than indexes.

**`and` and `or` return their operands** (§23.2, closing item 23), which gives
mesa the defaulting idiom it has lacked and spends §4's one surviving argument,
that a condition's type is always `Bool`. The dedicated nil-defaulting operator
is retired. The residue: `x or default` still defaults on `false`.

**Paren-less procs, at both definition and invocation.** Full rules in §14.8.
In brief: referencing an invocable — a `Proc` or bound `Method`, never a type
— invokes it, whether the reference is a bare name or a member expression,
and whatever the name resolves to. Zero arity invokes; any other arity is an
arity error. `&expr` yields an invocable without invoking it, and errors on
anything that isn't one. Callee position suppresses invocation, so `f`, `f()`,
`x.foo`, and `x.foo()` all behave. Places never invoke, so the left of `:=`
always designates the slot. Types are callable but not invocable: `Marker()`
constructs, `Marker` yields the type, `&Marker` is an error. A `def` may omit
an empty parameter list, matching `type`, which already allows it.

**Static methods, local types, and keyword arguments with defaults** (§21).
`def self.name(…)` declares a method on the type, reached as
`Amount.of_dollars(…)`; `self` is chosen over a `static` keyword because
§20.5 made keywords unrecoverable, and it forces item 28's resolution as a
side effect. Zero-required-arity statics give mesa constants free
(`Amount.zero`). A type body may contain nested types (`LinkedList.Iter`),
which the parser already accepts and the runtime currently panics on, and
which pull iteration (§20.8) needs somewhere to put iterator structs. Any
parameter may be passed by name, and parameters may have defaults —
evaluated **per call**, not once at declaration, which is where Python is
deliberately not followed. Fields take defaults too, since fields are the
constructor signature.

**Five working-list resolutions** (§20). Structural `Eq` for all user types
with `Hash` opt-in and `Eq` overridable, declared with **`impl`** rather than
`is` so the keyword reads correctly for protocols that grant a capability and
for those that override a default (item 9). Subscript is one protocol; a
read-only type raises `ProtocolError.NotImplemented` from the half it declines
(item 11). Arithmetic is not overloadable — *structure and access, yes;
algebra, no* — open to revision (item 13). `List` gets a `<<` append operator
that user types may overload (item 16), evaluating to its receiver (§24.3). An
infinite `loop do … end` with `break`, and no `while` (items 20, 21). `break`
takes a value, making a loop an expression; `next` is dropped and the continue
concept waits for a better word (§24.5).

**There is no visibility** (§27, closing item 24). Not public-by-default — no
mechanism: every module name is importable, every type member readable, no
export keyword, no private members, no convention. Modules are for naming and
imports for scope and provenance. This reverses §20.4's direction rather than
completing it, and it removes a keyword, an error variant, three unanswered
requirements, and §21.3's private-field leak. The cost is that every helper is
a public module name.

**The protocol set is nine** (§32, closing item 2). Equality and debug printing
are automatic and overridable; hashing and display are opt-in; ordering,
subscript, and append each require one verb with operators wired onto it; and
iteration is two protocols. Truthiness is explicitly *not* a protocol, and a
sequence protocol waits for demand. Callability isn't user-extensible after
§31.4, so `NotCallable` moves to `TypeError` and `NotInvokable` joins it — under
a rule that decides future cases too: a failure belongs in `ProtocolError` only
if some declaration a user could write would make it succeed. The nine names are
deliberately open.

**Protocols get a declaration form** (§31, settling half of item 2).
`proto Name` with a body of `def` signatures. ~~No default bodies, since
derivation can't be written as one anyway and a protocol carrying
implementations is the mixin §2 ruled out.~~ *Reversed by §34: **a member may
carry a body**, empty meaning required and non-empty meaning provided, with every
member closed by `end` so the distinction stays decidable.* The first half of the
struck reason had already been withdrawn in §31.1 itself, once introspection was
clarified; the mixin half is priced rather than defeated in §34.2, and the price
is that a type declaration no longer lists the type's members. The built-in
provided bodies are **mesa source**, loaded into the prelude tier (§34.8).
Signatures fix parameter *names*, not just arity, because §21.3 makes names
public API. Conformance is testable, which
answers §2's interface question; the mechanism — an operator against a
`$protos`-style builtin — is open, and choosing it reopens §17's unaskability of
parent membership. Operator-named members are declined: they'd be opt-in by
magic name, they'd need a blocklist for arithmetic, and a callable instance
would break §14.8. `?` and `!` become name suffixes, with a lexer rule mesa can
state cleanly because it has no bare `=`.

**The abbreviation rule** (§31.6): abbreviate frequent things and core types,
let infrequent things be longer, but nothing as long as `continue` or
`implements`. Settles `impl` and `proto`, and explains `def`, `Str`, and `Num`
after the fact.

**The protocol-naming rule** (§39.1): `impl X` reads "supports being X'd", so a
protocol name is a verb with the implementing type as its *object*. Composes
with the abbreviation rule — one sets part of speech, the other length — and
together they name all nine protocols, closing `L1`. `Equal` is the stated
exception, taken on length where no verb form was available.

**The native member gains an error channel** (§29, closing item 32).
`NativeField` and `NativeMethod` fold into one `NativeMember` — §14.8 merged
the concepts and only the native representation still had two — whose call
signature becomes `fn(&Val, Vec<Val>) -> Result<Val, Error>`. `Error` rather
than `Signal`, since a native member has no frame to return from and no loop to
break, and §16.4 already confines a callback's `return`. It depends on §19
moving `Location` out of the error variants, so a native member can raise
without knowing its call site.

**Modules are values** (§28). Bindable and passable, with their top-level
bindings as members reached through §17.3's existing machinery and §14.8's
existing rules. Members are read-only from outside, raising
`MemberError.ReadOnly`. Imports take a dotted path whose last segment is the
bound name — `import IO` and `import IO.File` are one syntax — and both forms
are needed, since qualified access can't bring a module into scope and
name-by-name can't hand you the module value. Imports are **static** (literal
path, top level only) and the graph is **acyclic**, with Go, OCaml, and JPMS as
precedent. A module is evaluated exactly once and shared by all importers.

**A prelude tier, unshadowable** (§20.6, closing item 4). Built-in names live
above the module scope and binding one is a declaration-time error, so the
resolution order completes as `locals → self's members → module names →
prelude`. This also settles §10's `$`-boundary on a better line than
shadowability, since both tiers are now unshadowable: `$` marks names that are
*not values*, while the prelude holds names that are.

**Iteration is pull** (§20.8, closing item 10). A collection yields an
iterator; an iterator yields a step. Two protocols and a per-loop state object,
against push's one protocol and none — but neither of the costs §13.4 priced
for push arises: `break` needs no unwinding through user code, and the
`RefCell` reentrancy hazard disappears because a borrow is released before the
body runs. *Amended by §37*: the exhaustion sentinel was recorded here as
plausibly §18.5's `Maybe`, and is not. A step is a `Bool` with the element in a
slot on the per-loop state object, which keeps the property §13.4's objection to
nil-as-sentinel required — signal and value on separate channels — without a
case type in the loop. The iterator's method was
barred from being called `next` by items 21 and 25 together; §24.5 dropped the
keyword, so the conventional name is available again and the choice is open.

**The prelude holds the core types only** (§20.7). `Str` and `Num` are always
visible and never imported; `Error`, its variants, and most protocols are
ordinary importable names living in built-in modules, shadowable and aliasable
because a name you never imported is just a free name. Eight names are
permanently spent rather than twenty-two — §10's discipline for `$` ("keep the
privileged tier to primitives") applied to the tier that was about to break it.

**Keywords are never member names** (§20.5, closing item 25). Ruby's
accept-any-keyword-after-`.` fix is declined, so `x.type` stays a syntax error
and `$type(x)` is the accessor. The keyword set is permanently unavailable for
members, bare or dotted, which makes every future keyword an unrecoverable cost
— `type` and `end` are the recurring casualties. `next` was the third until
§24.5 dropped it, which is the one case where a casualty was recovered, and
only because the keyword had not yet been spent in code.

*Qualified by §53.* "Permanently unavailable for members, bare or dotted" is
true of the mechanism item 25 declined, not of the keyword set. A parser-side
widening at the six positions where nothing but a name may appear recovers
`x.type` *and* `type R(start, end)` for every keyword at once; only the bare
form is genuinely structural, and only for words that lead an
expression-position construct.

**Nil means absence, and only absence** (§18, closing §0.3 item 1). An
out-of-bounds list read and a missing dict key raise rather than returning
nil. Nil remains the value of a proc that returns nothing, a search that finds
nothing, an unmatched `when`, and `each` — so it stays common, in one meaning
rather than six. Optionals, where wanted, are case types, which get §17.6's
coverage checking at the use site; that is opt-in per API rather than a
guarantee, since mesa has no return types to enforce it.

**Truthiness: `Bool` answers for itself; every other type is truthy except
`nil`** (§18.4). Ruby's and Lua's rule. `0`, `""`, and `[]` are truthy, so only
`Num` changes. The point is that truthiness now asks one question — *is this
present* — rather than a different one per type, which makes it the same
concept as nil's single job rather than a second concept interacting with it.

**Case types** (§17). A type may declare a closed set of variants. Variants
come first in the parent body and every `case` clause is closed by `end`; the
parent body may hold ordinary methods, which is how common behaviour is
expressed, and it reaches variants through verbs rather than fields. Variant
names are always qualified (`Expr.Ident`), which requires types to have
members but no new rule beyond §14.8. Consumption is primarily ordinary method
dispatch; `when e case Expr.Ident then … else … end` covers operations that
don't belong on the type. No union field access and no destructuring — inside
an arm, `e.name` is ordinary member access. Coverage is checked from the arms,
not the scrutinee, and an unreachable `else` is an error. The parent takes no
parameters, so shared fields repeat on each variant and a case-type parent is
not constructible. `$type` returns the variant, not the parent, so "is this
some kind of `Expr`" is deliberately unaskable — though §31.3 reopens that,
since a conformance mechanism could answer it and the choice is undecided.
Required verbs get no special clause — declare a protocol, which opens the
protocol set (§17.9).

**Conditional chaining is `else when`** (§6.1, §13.2 option A, closing §0.3
item 19). Three shapes, no more: `when c then … end`, `when e case … then …
end`, and `when c1 then … else when c2 then … end`. The third is not a new
form — it is the first with an `else` whose block is another `when`, so the
parser collects arms in a loop and builds the same right-nested tree it builds
for hand-nested code today. §13.2's multi-arm option C is dropped: it would
have been the only one of the three distinguished by layout rather than by a
keyword.

**A raised value is an instance of a case type** (§13.3, closing §0.3 items 6
and 7). Error types become ordinary user types, and `rescue` selects with the
matching §17.5 already provides rather than needing its own mechanism.

**Interpreter errors are rescuable** (§13.3, closing §0.3 item 8). Ruby's
answer rather than Go's, so mesa has one error system. Rescue arms reuse
§17.5's syntax but invert its totality: an unhandled error propagates rather
than requiring an `else`. `else` is allowed, on Python's precedent that a bare
`except` catches `NameError` too. Raise-site `Location` stays out of band —
diagnostic only, never a field on the value, never reachable from script.

**The built-in errors are restructured** (§19), grouped by *why* the operation
failed rather than by which operation caught it. Seven groups: `ProtocolError`,
`ArgumentError`, `TypeError`, `MemberError`, `IndexError`, `NameError`,
`KeyError`. Naming follows Python's conventions except where mesa has its own
concept — `MemberError` because mesa's types have members rather than
attributes, `ProtocolError` because mesa declares conformance in the source,
and `ArgumentError` (Ruby's name) because three of its four variants involve no
type at all. `TypeError` had been the residual category twice over; after these
splits it held two variants that say the same thing — four since §23.1 and
§32.2, which added `NotCallable` and `NotInvokable` once §31.4 made callability
non-extensible. Grouping is by *nesting*, a variant wrapping another case type,
since case types are one level deep by construction. `Error` is an
ordinary case type with no privilege; user error types are raised and caught by
the same rules, and catching *everything* is deliberately reachable only
through `else` rather than by naming one more arm.

**Proc literals, and no block construct** (§16, revised by §30). Procs gain a
literal form so an unnamed body can be written at the call site; there is no
separate block type. A literal captures `self` lexically and `return` inside one
returns from the literal. **A literal captures by value and may outlive its
frame** — §16's prohibition on escape is reversed by §30, because stored
callbacks were a population it never examined. Free variables resolving to
enclosing locals are snapshotted at creation; module and prelude names are not,
and an `Obj` capture copies the `Rc`, so object state is still shared. Frames
remain plain stack frames with no upvalue conversion, since what escapes carries
copies. What stops working is mutating an enclosing *local* through a closure.

**Significant newlines** (§15). A newline terminates an expression when the
preceding token can end one; otherwise the expression continues. Implemented
as one `nl_before` flag on `Token` plus one `TokenTag` table, consulted only
in `parse_expr_prec`'s loop. `return` is a special case: it terminates at a
newline, so bare `return` is an early exit.

**Instance method replacement becomes impossible** (§2.1, closed by §14).
`x.foo := v` has no reading once `x.foo` invokes, so `Member::set` rejects
names that resolve to methods. This also makes name resolution fully static
(§3.4).

**A protocol system for built-in behaviour** (§13.4), deliberately between
total flexibility and total rigidity: operator overloading in small, named
chunks rather than arbitrary dispatch hooks. Declared with `impl`, opt-in by
declaration rather than by magic name, and the set is **open** to
user-declared protocols (§17.9). Note the protocols do not all mean the same
thing — some grant a capability the type lacked, some replace a default it
already had (§20.1). Which protocols the language itself defines was item 2's
last question, closed by §32 at nine.

**No explicit declaration syntax** (§3.2). Bare `x := v` continues to declare
on first assignment. Block scoping stays as it is; loop scope is fixed so
each iteration gets a fresh scope (§3.3, §13.5).

**Error recovery is `raise`/`rescue`, with no postfix `?`** (§13.3, §18.6,
closing items 5 through 8). The `?` was only ever justified as rescue-to-nil,
which §18's absence-only nil forbids. The error model is now settled end to
end: a raised value is a case-type instance, `rescue` selects with §17.5's
matching, interpreter errors are catchable, and §19 gives the built-in set its
shape.

**A `$type` builtin** as *the* answer to the type-test question (§2.3), no
longer an interim one — item 25 declined the `x.type` spelling permanently, so
nothing is waiting to replace it. Its prerequisite is that native type names
be bound, which §20.7 settles as the prelude.

**Loop scope gets a fresh scope per iteration** (§3.3, §13.5), replacing the
single shared scope reused across iterations.

### Implementation consequences

**A new name-resolution order** (§14.5, completed by §20.6):
`locals → self's members → module names → prelude`. No tier is open. This is
step one of the sym table refactor rather than separate work.

**The resolver acquires three semantic jobs**, where its original brief was
name-to-slot assignment alone: capture-set computation for proc literals
(§16.2, §30.3 — an escape *check* until §30 removed the prohibition), match
coverage for case types (§17.6), and the unshadowable-prelude check (§20.6).
§3.4's claim that mesa is fully statically resolvable is what makes
all three possible, and each was adopted for its own reasons rather than to
justify the pass.

*Amended by §33: it is nine, not three.* Six more settled since — a parameter
default referencing rightward (§24.4), a body field doing the same (§26.1),
`Eq`/`Hash` pairing (§24.6), protocol conformance (§31.2), `break`'s proc
boundary (§29.2), and the import graph (§28.4). §33 prices the phase they add
up to, deflates most of them — only §17.6's coverage check genuinely requires a
pass, since the rest can run in `eval_decl` — and gives the phase an admission
rule, *questions about declarations, never about values*, so the next candidate
is decided rather than absorbed.

*Still nine after §34.* Default bodies add four checks, two at a protocol's own
declaration — a provided body referencing outside its protocol (§34.4), and
provided members with no required member beside them (§34.10) — and two at an
implementing type's — two unrelated protocols providing the same name (§34.6),
and a required signature's defaults disagreeing with the implementor's (§34.7).
All four are clauses on the conformance job already counted rather than a tenth.
Each asks only about declarations, so §33.6's admission rule admits them; and like
the rest of the deflated set they can live in `eval_decl` until the pass
exists.

*Named by §35: the phase is **semantic analysis**, and it is larger than §33's
pass.* Four carriers — the parser, the loader, `eval_decl`, and the resolver pass
— share one property (§33.3), one admission rule (§33.6), and one error type,
`sem::Error`. A check running in `eval_decl` is in the phase, not demoted out of
it, so an obligation's carrier is an implementation choice — wherever that
carrier reaches everything the obligation covers, which §35.2 finds is true of
all but two — while its membership is not. §35 also writes down the declared-entity model all ten obligations
quantify over — types, case parents and variants, protocols with required and
provided members, `impl` edges, module surfaces, the prelude tier — which had
grown across §§17, 21, 26, 28, 31 and 34 without being collected anywhere, and
states why a set of checks this size is still not a type system: none of them
asks what type an expression has, and none is a step toward asking.

## 0.2 Accepted but unbuilt

A to-do list. Nothing here depends on an open question any longer, and none
depend on each other except where noted. Five rows want a protocol *named*
rather than designed — pull iteration, `<<`, ordering on `Str` and `Char`, `Str`
iterability, and `Display`/`Debug`. §32 settled the set; the names are a
bikeshed held in §0.4.

| Item | Notes |
| --- | --- |
| Non-zero exit status on error (§5) | `main.rs` currently exits 0 on a runtime error |
| Call-location traceback (§5) | Best value-per-unit-of-work here; wanted either way — §5 argues it matters most while errors are fatal, §13.3 that `rescue` makes it *more* valuable, not less |
| Insertion-ordered dicts (§8) | Needs a dependency or a hand-rolled map; currently nondeterministic across runs |
| Grouping parentheses (§14.8) | `(a + b) * c` is currently unwritable |
| `&` (§14.3) | Prefix parsing now exists (§14.3); `&` is the last of the three items that shared it |
| Optional `return` operand (§9) | Needs the §15.4 special case, or it recreates JavaScript's `return` hazard |
| Bind native type names in the prelude (§13.6, §20.6) | Prerequisite for `$type`; also makes `Str()`/`List()` constructible, currently dead code |
| Prelude tier + unshadowable check (§20.6) | A `Scope` above the module's, plus a declaration-time error on binding a prelude name — the resolver's third semantic job |
| `$type` builtin (§2.3) | Inert until native type names are bound, two rows above |
| Paren-less procs (§14.8) | Depends on the resolution order above |
| Significant newlines (§15) | No existing fixture relies on the current behaviour |
| Proc literals (§16) | An `Expr` arm. *Amended by §30:* a literal does **not** hold the enclosing scope. It carries the capture set of the row below, and module and prelude names resolve at call time, so frames stay poppable — holding the current scope would be the by-reference design §30 reversed, and would contradict this row's neighbour. Spelling deferred |
| Capture-set computation for literals (§16.2, §30) | Copies each literal's free enclosing locals (plus `self`) into the `Proc` at creation; module and prelude names excluded. Replaces the upvalue analysis the GC refactor would otherwise have needed, and replaces §16.2's escape check, which §30 removed |
| Case types (§17.1) | `UserType` gains `parent` and `variants`; method lookup gains one fallback step. `TypeId` stays flat |
| Members on type values (§17.3) | Needed for qualified `Expr.Ident`; no new rule, since §14.8 clause 5 already excludes types from invocable |
| `when … case` matching (§17.5) | One branch in `parse_when_expr` where there is currently an unconditional `take(Then)` |
| Coverage check for matches (§17.6) | Reads the arms, not the scrutinee; the resolver's second semantic job after §16.2's escape check |
| Abstract (non-constructible) types (§17.9) | A case-type parent isn't callable — an `Expr::Call` arm rejecting it, and the one exception to §14.8 clause 5 |
| `else when` chaining (§13.2 option A) | One branch in `parse_when_expr`; no new keyword, no AST change, no runtime change |
| Raise on out-of-bounds list read (§18.2) | New `rt::Error` variant; closes the read/write asymmetry that was item 30 |
| Raise on missing dict key (§18.2) | New `rt::Error` variant; closes item 18. `main.ms:30` depends on the old behaviour |
| Truthiness change (§18.4) | `Val::is_truthy`'s `Num` arm becomes unconditionally true. `tests/when.ms:21` pins the old rule — and so does `tests/bool.ms:21` (`not 0`, expecting `true`), which §18.4's "the only fixture that does" missed |
| Dict `get(key, default)` and `has(key)` (§18.2) | Obligatory once a miss raises, not optional; joins `pop` as a native-method forcing function, `push` having been replaced by `<<` |
| Restructure the built-in errors (§19) | Eleven ad-hoc variants become seven groups; the enum becomes public surface. `Location` moves out of the variants and onto `Signal::Error` — **§29 depends on that half**. Amended since: `IndexError` splits (§23.1), `NotCallable` moves to `TypeError` and `NotInvokable` joins it (§32.2), `ConcatNonStr` is added (§22.5) |
| `<<` append operator (§0.3 item 16) | `TokenTag`, two-char lexing beside `<` and `<=`, a left-associative `Precedence::of` mapping, and a protocol name. Evaluates to the receiver (§24.3) |
| `loop do … end` and `break` (items 20, 21) | One `Signal` variant carrying a `Val`, two keywords, no new machinery class. `next` is not built (§24.5) |
| `ProtocolError.NotImplemented(val, name)` (§20.2) | Raised by the declined half of a one-protocol subscript, and by any user code meaning "deliberately unimplemented" |
| Pull iteration: two protocols + sentinel (§20.8) | Collection yields an iterator, iterator yields a step; `each` desugars onto it. *Amended by §37*: the sentinel is **not** `Maybe` — a step is a `Bool`, with the element in a slot on the per-loop state object. Both protocols exist (§32.1); their names and the step method's are open, `next` included again since §24.5 |
| `raise` / `rescue` (§13.3, §19) | The error model is settled end to end but nothing here built it. Arms reuse §17.5's syntax with inverted totality; `Signal::Error` carries a `Val` plus an out-of-band `Location` |
| Static methods (§21.1) | `def self.name` — needs `self` as a real `TokenTag`, which closes item 28. Lives in the same type-member namespace as variants and nested types |
| Local types (§21.2) | The parser already accepts them; `rt.rs:569` is a `todo!()`. Needed by §20.8's iterator structs |
| Keyword arguments + defaults (§21.3, §24.4) | Defaults evaluate per call, in parameter order, and may reference parameters to their left. Amends §14.8 clause 1 to *zero required arity*. `NativeMember`'s fixed `arity: usize` becomes a range — same struct §29 folds, so the two want one pass over `CORE_TYPES`. §19.3 gains an `ArgumentError` group of four, retiring `WrongArgCount` |
| `NativeMember` + error channel (§29) | `NativeField` and `NativeMethod` fold into one struct with one map on `NativeType` and one array per entry in `CORE_TYPES`; `call` becomes `fn(&Val, Vec<Val>) -> Result<Val, Error>`. **Sequenced after** §19's `Location` relocation, or every native member needs a location parameter |
| The module system (§10, §27, §28) | Entirely unbuilt — nothing in the tree is multi-file. `Obj::Module` and a `Module` type, a per-module `Scope`, the dotted-path import form, a static import graph with cycle detection, and once-only evaluation with the module value cached |
| Body fields (§26) | `UserType` gains a second field list plus their initializer `ExprId`s, evaluated after the constructor fields bind. A parser arm for `Ident :=` in a type body, giving `rt.rs:579` a meaning; a bare expression there becomes a syntax error. Resolver check for rightward references |
| `proto` declarations (§31.1, §34) | A `Decl` arm, `def` members inside each closed by `end`, conformance recorded on `UserType` by `impl`. Signature checking compares parameter names, not counts. **A member with an empty body is required; one with a body is provided and implementors acquire it** (§34.3) |
| The derived library, in mesa source (§34.8) | A loader evaluating embedded mesa source into the prelude tier before the user's module — not §28's module system. Needs a synthetic source identity for `Location`, and §20.6's unshadowable check extends over it. Scaffolding: moves into built-in modules when §28 lands |
| `?` and `!` in names (§31.5) | Lexer: consume a trailing `!` unless followed by exactly one `=`; `?` unconditionally. Suffix only, not on type names |
| `Display` and `Debug` protocols (§25) | Two protocols with structural `Debug` derived for user types and `Display` falling back to it. Fixes `rt.rs:1233`, where `Obj::Instance` prints fields in the display form while `List` and `Dict` use debug. Needs a cycle marker (`...`) in the printer. Both protocols exist (§32.1); names open |
| Coinductive `Eq` + bounded `Hash` (§24.7) | A lazily-allocated `Vec` of pointer pairs threaded through comparison, an `Rc::ptr_eq` fast path, and a node counter for hashing. ~~Lands with item 9's structural `Eq`; unreachable before then, since only `Str` is structural today~~ — *split by §49.5*: the **equality** half lands with structural `List`/`Dict` equality, which is not item 9 and depends on nothing, since `a := []; a << a` is a cycle with no user type in it; the **hashing** half still waits on item 9, nothing being hashed structurally until a type declares `impl Hash` |
| Ordering on `Str` and `Char` (§24.2) | Scalar-sequence comparison in the `<`/`<=`/`>`/`>=` arms, mixed operands rejected. The ordering protocol that lets user types join exists (§32.1); its name is open |
| Immutable `Str` + cached scalar count (§22.1, §22.2) | `Str::size` counts characters, computed once and stored beside `chars: Box<str>`. `tests/str.ms:6` is ASCII and unaffected |
| `Char` immediate, `'a'` literals (§22.3) | A `Val` variant beside `Num` and `Bool`, a ninth `CORE_TYPES` entry and prelude name, and a `lex_char` mirroring `lex_str` with `\'` added. The multi-scalar error message should name double quotes |
| `Str.chars` and `Str.slice` (§22.4) | `chars` builds a fresh `List` of `Char` per call, by design; `slice` is the stopgap until `Range`. Both need item 32's error channel |
| `Str` iterability (§22.4) | Yields `Char`s under §20.8's protocol without materializing; that protocol's name is open (§32.3) |
| `+` on `Str` (§22.5) | Left-operand dispatch in the `+` arm, plus `TypeError.ConcatNonStr(val)` beside `ArithNonNum` |
| Index validation (§23.1) | `list[1.7]` and `list[-1]` raise rather than truncating. `IndexError` splits into `OutOfRange` and `NonIntegral`; `TypeError.IndexNonNum` keeps its name and its job. Applies to `slice` bounds too, and never to dict keys |
| `and`/`or` return operands (§23.2) | `rt.rs:945-962`, one line per branch. `tests/bool.ms` uses `Bool` operands throughout, so no expected output moves |
| Zero arity prints without parens (§23.3) | `rt_print_proc` |
| The resolver phase (§33) | Nine jobs on one traversal, between `Parser` and `Interpreter`, with side tables keyed by `ExprId`. Two sub-passes — declarations, then bodies — per module in import-DAG order. Its own error type, not rescuable. **Forced only by** the coverage-check row above; the other jobs can run in `eval_decl` until it exists, and are cheaper here than scattered |

*Re-checked against the tree at `14fccbb`.* `not` and unary minus have landed
since the first pass and are dropped from this table; they built the
prefix-parsing infrastructure all three of those items shared, leaving `&` as
the only piece of it outstanding (§4, §14.3). Every other row above was
re-verified as still unbuilt.

*Two sequencing constraints* are the only ordering this list has. §29's
`NativeMember` error channel needs §19's `Location` relocation first, or every
native member grows a location parameter; and §21.3's arity-as-a-range is an
edit to the same struct §29 folds, so those two want one pass over
`CORE_TYPES`. Everything else is independent.

*A third arrives with §33.* The resolver row is forced by the coverage-check row
and wanted by the capture-set row — the two entries that name it as their
mechanism — so it precedes both. It is a weaker constraint than the other two,
since those two are the only rows that cannot be written without it.

## 0.3 Open — the working list

Ordered roughly by how much each one unblocks.

**Nothing here is open.** Item 2, the last one, closed with §31 and §32; item 27
is settled-for-now and explicitly revisitable rather than undecided. The list is
kept in full, with each item's resolution at its own number, because the
resolutions are what the numbers are referenced by. What remains anywhere is in
§0.4, and all of it is naming or shape rather than design.

### Blocking others

1. ~~**The nil policy** (§13.7).~~ **Closed by §18: absence only.**
   Out-of-bounds reads and missing dict keys raise; nil keeps a proc that
   returns nothing, a search that finds nothing, an unmatched `when`, and
   `each`. Truthiness collapses to the same question (§18.4). Of the four
   things this blocked: the postfix `?` design is settled by killing it
   (item 5), dict absent-vs-nil is settled (item 18), the iteration protocol
   is settled as pull (item 10), and the defaulting idiom was reframed and then
   closed by §23.2 (item 23). Number kept for reference stability.
2. ~~**Protocol dispatch shape**~~ (§13.4). **Closed by §31 and §32.** Opt-in
   by declaration — settled from the start. The
   set is **open**: §17.9 routes case-type required verbs through an ordinary
   user-declared protocol rather than a header clause, which is the "later"
   §13.4 preserved the option for. ~~What remains is which protocols the
   language itself defines, and — newly — a *declaration form*.~~ **The
   declaration form is settled by §31**: `proto Name` with bodiless `def`
   signatures, ~~no default bodies,~~ parameter names fixed and not only arity.
   (*§34 reverses the default-bodies half*: a member may carry a body, empty
   meaning required and non-empty meaning provided, with every member closed by
   `end`. The built-in defaults are written in mesa source and loaded into the
   prelude tier. Nothing else in this item's closure moves.)
   Conformance *is* testable, which answers §2's interface question; whether by
   an operator or a `$protos`-style builtin is open, and that choice reopens
   §17's deliberate unaskability of parent membership. Operators stay wired
   through protocols rather than through operator-named members. **And the set
   is settled by §32: nine** — equality, hashing, ordering, subscript, append,
   the two iteration protocols, display, and debug. Truthiness is explicitly
   declined; a sequence protocol is deferred until there is demand;
   callability moves out of `ProtocolError` entirely, taking `NotCallable` to
   `TypeError` and adding `NotInvokable` beside it. **Closed.** Only the nine
   *names* are open, and they sit in §0.4 with the other spelling threads.
32. ~~**The native-method error channel** (§5, §20.3).~~ **Closed by §29.**
    `NativeField` and `NativeMethod` fold into one `NativeMember`, since §14.8
    already made field and zero-arity method one idea and only the native
    representation still had two. Its signature becomes
    `fn(&Val, Vec<Val>) -> Result<Val, Error>` — `Error` rather than `Signal`,
    because a native member has no frame to return from and no loop to break,
    and §16.4 already confines a callback's `return` to the literal. **It
    depends on §19's `Location` relocation**: a native member can't know its
    call site, so it returns a location-free error and the interpreter attaches
    the location at the call boundary. Those two want to land together.
    *Numbered out of sequence deliberately:* this list is ordered by how much
    each item unblocks, and this one was never a numbered item at all — it
    lived inside item 16, whose closure took its only home with it.
3. ~~**Can a block contain a declaration?** (§3.1)~~ **Closed by §16,
   *revised* by §30.** The question was really whether a closure can escape its
   frame, not whether `Decl::Def` can appear in a block — proc literals reach
   the same place by the other path. §16 answered that a literal may capture but
   may not outlive its frame; **§30 reverses that**: a literal captures **by
   value** and may outlive its frame, because stored callbacks (test
   registration, routing tables, event handlers, factories) are a population §16
   never examined. The GC refactor still needs no upvalue analysis — escaping
   literals carry copies, so frames stay plain stack frames — and the resolver's
   job becomes computing capture sets rather than rejecting escapes.
   `Decl::Def` in a block stays disallowed. Number kept for reference
   stability.
4. ~~**The prelude tier** (§14.5, §13.6, §10).~~ **Closed: a tier above
   modules, and unshadowable.** The resolution order completes as
   `locals → self's members → module names → prelude`, with no parenthetical
   left. Unshadowable is the load-bearing half: because the prelude sits
   *above* the module, ordinary scoping would let a module-level `type Str`
   win, so the prohibition is an explicit declaration-time check rather than
   a consequence of the ordering — the resolver's third semantic job, after
   §16.2's escape check and §17.6's coverage check. It also answers §10's
   `$`-boundary question, which this item noted was the same question: both
   tiers are unshadowable, and what separates them is that prelude names are
   **values** while `$` names are **syntax** (§20.6). And the prelude holds
   **the core types only** (§20.7) — `Error`, its variants, and most
   protocols are ordinary importable names in built-in modules, so eight names
   are permanently spent rather than twenty-two. (**Nine since §22.3**, which
   added `Char` and was the first thing to test the budget; §28.1 leaves
   `Module` undecided between the two tiers.)

### Error model (§13.3)

5. ~~**`raise`/`rescue`, postfix `?`, or both**~~ **Closed by §18.6:
   `raise`/`rescue`, no `?`.** The distinct job `?` was being kept for was
   rescue-to-nil, and under absence-only that is exactly the conflation being
   removed. §13.3's own analysis then applies — exceptions propagate
   automatically, so `?` is "redundant by construction." ~~Revisitable only if
   `?` is given a third job.~~ **No longer revisitable** — §31.5 makes `?` a
   name suffix, and a character cannot be a name character and a postfix
   operator at once. Spent knowingly: the postfix form's only remaining
   justification died with §18's absence-only nil, and `x.empty?` is worth more
   than a hypothetical.
6. ~~**What is a raised value?**~~ **Closed: an instance of a case type.**
   Error types are ordinary user types, nothing new is needed to define one,
   and a closed error alternation gets §17.6's coverage checking for free.
   Forces §13.3's fourth question — `Signal::Error` must carry a `Val` rather
   than an `rt::Error` once errors are script-visible.
7. ~~**How does `rescue` select?**~~ **Closed, by (6) plus §17.5.** That
   matching already exists for independent reasons, so `rescue` embeds arms
   directly (`rescue case Err.NotFound then …`) rather than nesting a match
   in its body, which would be the pyramid §13.3 objected to. **But rescue
   arms are not a match**, and (8) makes the difference load-bearing:
   §17.6 requires a match to be total, so an uncovered variant demands an
   `else`. A rescue must default the *other* way — an unhandled error
   propagates, which is the entire point of exceptions. Applying §17.6's rule
   unchanged would make every partial rescue swallow everything. So arms
   share §17.5's syntax and invert its totality. `else` **is** allowed, on
   Python's precedent — a bare `except` there catches `NameError` too — with
   the typo hazard mitigated by convention rather than mechanism, and made
   rarer by §19's grouping, which lets a broad catch name a group instead.
8. ~~**Are interpreter errors catchable?**~~ **Closed: yes** (§13.3
   question 3). Mesa answers Ruby's way, not Go's, so there is one error
   system. The error set gets restructured and renamed on Python's
   conventions (§19), becomes public surface, and adding a variant becomes a
   compatibility question. Grouping is by *nesting* — a variant wrapping
   another case type — since case types are one level deep by construction.
   Raise-site `Location` stays out of band: diagnostic only, never reachable
   from script, so it is not a field on the error value.

### Protocols (§13.4), once (2) is decided

9. ~~**Eq/Hash coupling.**~~ **Closed as proposed, plus override.**
   Structural `Eq` automatic for all user types, `Hash` opt-in so the
   mutable-key hazard is visible, and a type may **override** structural
   equality with its own. The declaration keyword becomes **`impl`**, not
   `is`: `impl` asserts an implementation, so its absence reads correctly
   both for protocols that *grant* a capability and for those that *override*
   a default, where `is` would have claimed the type "is not" something it
   still is (§20.1). **Both remaining threads are now closed.** §24.6: an
   overridden `Eq` invalidates the derived `Hash`, so a type overriding one and
   declaring the other must supply both, checked at declaration. §24.7: the
   cycle answer is coinductive — an identity fast path, then a set of pairs
   under comparison, assuming equality on revisit. Hashing is bounded by node
   count instead, and the bound may not read sharing.
10. ~~**Iteration: push or pull.**~~ **Closed: pull.** §13.4 chose push on two
    arguments, both since spent — §16.5 retired "it keeps blocks off the
    language surface" once the body became an ordinary proc, and §17.8
    retired the exhaustion-sentinel objection once case types supplied a
    sentinel for unrelated reasons. Re-decided rather than inherited, and
    it costs more mechanism: two protocols (a collection yielding an
    iterator, an iterator yielding a step) and a state object per loop.
    What it buys is that **both of the costs §13.4 priced for push simply
    don't arise** — `break` needs no unwinding through user code, and the
    `RefCell` reentrancy hazard disappears because a borrow is released
    before the body runs (§20.8). Two things it forces: the sentinel is
    plausibly `Maybe` rather than a new type (§18.5), and the iterator's
    method **cannot be called `next`**, since item 21 made that a keyword
    and item 25 bars keywords as member names. *Both were reversed later* —
    §24.5 refunded `next`, and §37 declined `Maybe` in favour of a `Bool`
    plus a slot on the per-loop state object.
11. ~~**Subscript: one protocol or two.**~~ **Closed: one.** A read-only type
    implements both halves and raises a predefined "not implemented" error
    from the half it declines. Fewer protocols, one mechanism. The cost is
    that read-only-ness moves from *declared* to *discovered at runtime* —
    §13.4's argument for splitting was that a type could be "readable but not
    writable, which is a real and common shape," and that shape is still
    expressible but no longer visible in the type declaration. The error is
    **`ProtocolError.NotImplemented(val, name)`**, general rather than
    subscript-specific — "this operation exists and is deliberately
    unimplemented," raisable by any user code (§20.2).
12. ~~**Show: one printing mode or two.**~~ **Closed by §25: two, affirmatively.**
    `Display` and `Debug` as protocols, `show` and `inspect` as verbs — initial
    candidates held for a later bikeshed, not settled spelling (§0.4). **No
    round-trip guarantee**, declined outright rather than aspired to, since
    procs, types, and cyclic values have no literal form; the fact that core
    debug output happens to be valid mesa syntax stays true and stays
    unpromised. Containers print their elements in the debug form, which is
    already true in the tree and is the concrete argument against one mode.
    Cycles print as `...`. `Debug` is automatic and structural for every user
    type while `Display` is opted into with `impl`, mirroring item 9's
    `Eq`/`Hash` shape, so `$print` always has something to print. No second
    builtin: the debug form is reached as `$print(x.inspect)`.
13. ~~**Is arithmetic overloadable?**~~ **Closed: no.** The proposed line
    stands as the rule — *structure and access, yes; algebra, no* — with the
    system explicitly open to revision if a real need appears. Two
    consequences: `ArithNonNum` stays in `TypeError` rather than joining
    `ProtocolError` (§19.3), and **(14) inherits a constraint** — with `+`
    permanently non-overloadable, string concatenation must be hardcoded
    native behaviour or a different operator, exactly as §13.4 warned while
    that decision was still open.

### Values and data — the largest functional gap

14. **Strings** (§8). ~~No concatenation, no interpolation, no indexing, no
    iteration.~~ **Narrowed by §22 to interpolation alone.** Strings are
    immutable, `+` concatenates two `Str`s, characters come from `s.chars` as a
    `List` of the new `Char` immediate, `Str` iterates under §20.8, and
    `s.slice` covers substrings. What remains open is the interpolation
    *spelling* — `"$(expr)"` against Ruby's `"#{expr}"` — which is deferred
    rather than blocked: §22.5 settled concatenation first, so interpolation
    now has a desugaring to land on, and §25 has since settled which printing
    mode it uses. This item's guess that interpolation would make the
    `+`-versus-`..` question moot ran the other way in the end; `+` was decided
    on its own and interpolation inherits it, as it now inherits `Display`.
15. ~~**`Str.size` counts bytes** while `List.size` and `Dict.size` count
    elements.~~ **Closed by §22.2: it counts characters.** Fixed by making
    `Str` agree with the other two rather than by renaming, so one name keeps
    one meaning across three types. Scalars, not graphemes — `size` still isn't
    "what you see" for a combining accent, which is where Python and Ruby also
    sit.
16. ~~**Lists cannot grow** (§8).~~ **Closed by `<<`**, an append operator
    that user types may overload — structure, not algebra, so it sits on the
    right side of (13)'s line. It needs a `TokenTag`, two-char lexing beside
    `<` and `<=`, a `Precedence::of` mapping, and a protocol name.
    **But it bypasses what remains of the forcing function this item
    described.** The field-versus-method half of that plan no longer exists —
    §14 deleted the boundary, per §2.2 — so only the native-method error
    channel was still live, and an operator dispatches through `Expr::Binary`
    without crossing the native boundary. `pop` was then the only thing forcing
    it (§20.3), until §18.2 and §22.4 added several and §29 closed it. ~~Also
    open: what `<<` evaluates to.~~ **Closed by §24.3: the
    receiver**, so `l << 1 << 2` chains and `Precedence::of` needs left
    associativity. The protocol requires the same of user implementations,
    rather than leaving the result to convention.
17. ~~**Numbers** (§8). f64-only, no modulo, silent index truncation.~~
    **Closed by §23.1, one answer per part.** f64-only stays, and is now a
    decision on the record rather than an inheritance — no `Int`, no `Float`.
    Modulo goes unwritten until something needs it, which is cheap because
    nothing is designed around its absence. Silent truncation ends: a list
    index must be a whole non-negative number, and `list[1.7]` and `list[-1]`
    both raise. Negatives raise *in order to* reserve them, so
    Python-style negative indexing and ranges remain a pure extension (§22.6).
    The taxonomy splits along the type/value line rather than by operation:
    `TypeError.IndexNonNum` for a subscript that isn't a `Num`, and a newly
    grouped `IndexError` of `NonIntegral` and `OutOfRange` for `Num`s that
    aren't positions — which amends §19.3, where `IndexError` was flat because
    it then had only one *why*. Dict keys are exempt throughout, being keys
    rather than indexes.
18. ~~**Dict conflates absent with nil.**~~ **Closed by §18.2.** A miss
    raises, so a stored nil reads back as nil and the two are distinct with no
    further rules — §8's `has` becomes obligatory rather than the thing that
    would have made the conflation observable. Lua's assign-nil-deletes
    uniformity isn't needed.

### Control flow

19. ~~**Conditional chaining** (§6.1, reopened by §15.1).~~ **Closed as
    `else when`** — §13.2 option A, no new keyword, no AST change, no runtime
    change. The multi-arm option C is dropped: with `when e case … then`
    settled it would have been the only one of three `when` shapes
    distinguished by layout rather than by a keyword. Number kept for
    reference stability.
20. ~~**Loops beyond `each`** (§6).~~ **Closed: an infinite `loop do … end`,
    and nothing else.** No `while`; a condition-driven loop is `loop do` with
    a guarded `break`, which is Ruby's answer rather than Lua's. The form
    obeys §6's own rule — *the opener word names the relationship*, and `do`
    introduces repeated action — so `loop do` was the only spelling
    consistent with what that section had already articulated.
21. ~~**`break` and `continue`** (§6).~~ **Closed: `break` only, and it takes
    a value** (§24.5). `break x` makes the loop an expression yielding `x`;
    bare `break` yields nil. One `Signal` variant, no new machinery class.
    **`next` is dropped** — the continue concept waits for a word shorter and
    less common than `continue`, and nothing is reserved meanwhile. That
    returns the keyword, which was the costliest of §20.5's three: `node.next`
    works again, and §20.8's iterator step method can take the conventional
    name after all. The caveat this item used to
    carry — that (10) makes `break` from inside a user-defined `each` part of
    the protocol's contract, so an implementation must tolerate abandonment —
    **no longer applies**: that was a cost of *push*, and item 10 closed as
    pull, where breaking is simply not calling the iterator again (§20.8).

### Booleans

22. ~~**Truthiness** (§4).~~ **Closed — the premise was wrong.** This claimed
    `Num` truthiness was `n > 0.0`, making negative numbers falsy. It is
    `num.0 != 0.0`, the near-universal rule, and has been since `is_truthy`
    was introduced in `0b04b91` — which predates this document. So the
    finding was never true, rather than having gone stale. Verified:
    `when -5 then` takes the *then* branch, `when 0 then` takes the else.
    Number kept so the references above stay stable. **The rule has since
    changed anyway** (§18.4), for a reason unrelated to the bogus finding:
    `Num` becomes unconditionally truthy so that truthiness asks one question
    across all types. The corrected `!= 0.0` above describes the tree, not the
    design.
23. ~~**The defaulting idiom** (§4).~~ **Closed by §23.2: `and` and `or`
    return their operands.** Both change, not just `or`. §4's first argument
    for strict boolean had already expired with §18.4's truthiness; the second,
    that a condition's type is always `Bool`, is what this spends, and it is
    spent knowingly. The dedicated nil-defaulting operator this item preferred
    is retired rather than deferred — a value-returning `or` covers the same
    ground with no new token, failing only on a genuinely `false` left operand,
    which under §18.4 is the only non-`nil` falsy value. Short-circuiting is
    unchanged and `not` still returns `Bool`.

### Modules (§10)

24. ~~**Default export visibility.**~~ **Closed by §27: there is no
    visibility.** Not "public by default" — no mechanism at all. Every module
    name is importable, every type member is readable, and there is no export
    keyword, no private member, and no naming convention standing in for one.
    Modules are for naming and imports are for scope and provenance; neither is
    about access. The three requirements that made this item bigger than its
    framing (§20.4) evaporate rather than being answered, as does §21.3's leak
    of a private field's name through the constructor, and §20.4's promised
    private-access error. §20.5's keyword list loses the visibility keyword
    permanently. **The cost:** every helper is a public module name, sharpened
    by §3.1's ban on nested `def` — blunted by proc literals, by name-by-name
    imports making the effective interface what importers name, and by the
    proc-local helper being a want that rarely materializes. Precedent is
    Smalltalk, which has no private methods either.

### Deferred by agreement

25. ~~**Keyword namespace** (§9).~~ **Closed: not supported.** Keywords are
    not accepted as member names after `.`, so Ruby's one-line fix is
    declined and the keyword set stays permanently unavailable for members —
    bare access too, since a bare keyword-named member lexes as its keyword
    either way. `x.type` remains a syntax error and `$type(x)` is the type
    accessor, which §13.6 had already settled independently. The recurring
    casualties are `type` (`event.type`, `token.type`) and `end`
    (`range.end` — and `type R(start, end)` still fails to declare). `next`
    was the third until §24.5 dropped it, which refunds `node.next` and
    §20.8's iterator method — the one recovery available, and only because the
    keyword had not been spent in code. This makes §20.5's keyword budget a
    permanent cost rather than a deferred one.
26. ~~**Concurrency** (§11). Nothing exists.~~ **Closed by §24.1: mesa is a
    blocking language, and the design is deferred.** This item asked for the
    decision rather than the design, and that is what it gets — the runtime
    already assumes single-threaded execution everywhere, and now nothing else
    has to ask. Threads later would mean `Rc` becomes `Arc` and every
    `RefCell` becomes a lock: expensive and mechanical, changing no semantics
    settled here.
27. **The spelling of `&`** (§14.6). Settled for now, explicitly revisitable.

### Minor

28. ~~`self` has three different statuses across interner, parser, and runtime
    (§9). Making it a real `TokenTag` costs almost nothing.~~ **Closed by
    §21.1** — and no longer minor. Static methods spell as `def self.name`, so
    the parser must recognise `self` immediately after `def` and it becomes a
    real `TokenTag` by necessity rather than tidiness. §9's wanted side effect
    arrives with it: `self := 9` becomes a syntax error instead of declaring
    an ordinary local.
29. ~~`rt_print_proc` always prints parens, so a proc declared `def render end`
    would print as `def render()` (§14.8).~~ **Closed by §23.3: zero arity
    prints without parens.** One printed form per proc rather than a record of
    what was typed, since §14.8 makes `def render` and `def render()` the same
    declaration.
30. ~~Read/write asymmetry on out-of-bounds list access (§12): reads return
    nil, writes error.~~ **Closed by §18.2** — both raise. The asymmetry was
    an artifact of having nowhere to put a read failure.
31. ~~Ordering is `Num`-only (§12), so `"a" < "b"` doesn't work.~~ **Closed by
    §24.2: `Num`, `Str`, and `Char`.** Strings and characters compare by scalar
    sequence — code-point lexicographic, not locale collation, which is the
    same table §22.2 declined for graphemes. Mixed operands don't compare.
    Whether user types join is item 2's `Order` protocol, and §19.3's
    `NotOrderable` arrives with it.

## 0.4 Loose threads inside settled work

With §0.3 fully struck through, **this is the live list.** It collects what is
still unfinished inside items and sections that are otherwise closed — where a
strikethrough or a settled section heading would otherwise hide it. Nothing here
blocks anything, and after §24.7 nothing here is a correctness question: what
remains is naming, spelling, and two small mechanisms.

*Amended by §34.* Still no correctness question, but "nothing here blocks
anything" now has one exception: **whether the derived library is lazy or eager**
gates writing that library, since the two shapes produce different code rather
than differently-spelled code. It is a mechanism thread, not a naming one, and it
is the only row here with work waiting behind it. §34.7 closes the
signature-defaults row in exchange.

*Amended by §36.* One row added, and it is the first here that is a question
about what users may write rather than about spelling or mechanism: **whether a
native type may implement a user protocol**. §36.2 finds that the answer falls
out of existing decisions as *no*, and lists it because a consequence that
narrows the language should be accepted rather than inherited.

*Amended by §37.* The exhaustion-sentinel row closes, leaving `D7` with naming
threads only. It is the second row here to be settled rather than spelled, and
like the signature-defaults row it was closed by finding that its premise had
lapsed: §20.8 justified `Maybe` by an economy with §18.5's optionals, and §37.3
declines the optionals, so the economy was carrying the choice rather than
supporting it. One thread is added in exchange and immediately deferred —
**generators**, §37.6 — with a stated revisit condition rather than an open
question, since nothing about it is decidable until the evaluator's frames
change.

*Amended by §42.* The `Module`-in-the-prelude row closes, and it is the third
here settled rather than spelled — again by finding that the question had
changed underneath itself. §42.6 gives the prelude tier a membership rule (the
native types, and nothing else, once §34.8's scaffolding expires), which decides
the row without anyone weighing a name against a budget. The row's *alternative*
did not lose so much as dissolve: it offered "an ordinary importable name in a
built-in module, as `Error` and the protocols are," and those are precisely the
names §42.6 moves out of the prelude. Nothing is added in exchange — §42's one
opened thread is a harness question and lives in the roadmap's §1.5, not here.

*Amended by §43.* One row added, and it is the smallest kind that belongs here —
not a question that was always open, but a choice a decision made *visible*. §43
settles that `Order` is coarser than equality, which turns what `min` and `max`
return on a tie from an unobservable detail into two different values. Nothing
closes in exchange, and §43's own subject was never a row here: it sat inside
§24.2's one-line handoff to the protocol, which is where a clause hides when the
built-in types all satisfy it by accident.

| Thread | Lives in | Note |
| --- | --- | --- |
| ~~Names for the nine protocols, and the printing verbs~~ | item 2, §32.3, §25.1, §39 | **Closed: `Equal`, `Hash`, `Order`, `Access`, `Append`, `Iterate`, `Advance`, `Display`, `Inspect`**, with the printing verbs `display` and `inspect`. Decided by a rule rather than a preference — §39.1 reads `impl X` as "supports being X'd", which rejects nouns (`Subscript`, `Item`) and verbs whose object is not the type (`Debug`), while §31.6 rejects the truncations (`Eq`, `Iter`). `Equal` is the stated exception, an adjective taken on length so it is not the set's only short name. The subscript protocol's *error* name resolves to nothing: §20.2 already made the declined half raise the general `ProtocolError.NotImplemented` |
| The iterator step method's name | item 10, §20.8, §24.5, §39.4 | `next` became available again when item 21 dropped it; taking it is not automatic. §39 names the protocol `Advance` and **declines to let that close this row** — `advance` is now a second free candidate, not a decision |
| ~~Whether the exhaustion sentinel is `Maybe`~~ | §20.8, §18.5, §37 | **Closed: no.** A step is a `Bool`, with the element in a slot on the state object §20.8 already allocates per loop. Signal and value on separate channels keeps the property §13.4 required, at no allocation per element; the cost accepted in exchange is that reading the slot after a false is an invalid-state read. §18.5's optionals are demoted with it — still expressible, no longer the language's answer to absence |
| Interpolation's spelling | item 14, §22.6 | `"$(expr)"` against `"#{expr}"`. Both its dependencies — concatenation and the display form — are now settled, so only the spelling is left |
| The import keyword's spelling | §27.3, §31.6 | `impl` and `proto` are settled by the abbreviation rule; the import keyword is what's left of this thread |
| ~~Whether protocol signatures may carry defaults~~ | §31.2, §34.7 | **Closed: yes, and implementors repeat them**, checked at declaration. The back-door objection is void once §34 makes bodies the front door; repeating follows from §21.3 making a default part of the signature an implementor must match |
| Whether the derived library is lazy or eager | §34.11 | Lazy needs an adapter type per combinator (§21.2 local types) and an interpreted step per element per stage; eager returns a `List`, which has native members, so a chain lands back on native ground after one stage. Eager cannot express an infinite source. Wants deciding before the library is written |
| Which combinators the derived library has, and their names | §34.11 | A list rather than a mechanism, and it blocks nothing: a protocol may gain provided members later without breaking implementors |
| Conformance: operator or `$` builtin | item 2, §31.3 | `x is Serialize` against `$protos(x)`. Choosing also decides whether §17's parent membership becomes askable |
| ~~Whether `Module` sits in the prelude~~ | §28.1, §20.7, §42.6 | Prelude (a tenth permanently-spent name) against an ordinary importable name in a built-in module, as `Error` and the protocols are. Nameable either way; the only use is a `$type` comparison. **Closed by §42.6: the prelude**, which after §34.8's expiry holds the native types and nothing else — a rule `Module` satisfies by construction, since §36.2 leaves a native type nowhere to declare itself in mesa. The alternative half of the question dissolved rather than lost: `Error` and the protocols are exactly the names §42.6 moves *out* |
| A word for the continue concept | item 21, §24.5 | Deferred as a feature, not reserved as a name. Shorter and less common than `continue` |
| `break` crossing a proc boundary | §29.2, §24.5 | Intended not to work; the mechanism is undecided. A lexical loop-depth check per proc body would make it a parse error and need no `Signal` variant — but that argument partly leaned on §16.2's escape prohibition, which §30 removed, so it is weaker than when it was made and wants re-examining |
| A failure path for `NativeType.new` | §29.4 | Same gap §29 closed for members. Inert until something constructible can fail — `Str(c)`, or `Str` from a list of `Char`s |
| Whether a native type may implement a user protocol | §36.2 | *(new with §36.)* `impl` attaches to a type declaration and native types have none, so their conformance is declared in Rust and the table is closed — which means `proto Json` can never be implemented by `List`. Consistent with §0.1's no-reopening and §27's no-visibility, and additive to reverse later. The fallback is a wrapper type, which §34.1's free-function objection does not reach. Listed because it narrows the language and should be accepted deliberately |
| Generators, as the authoring form for iterators | §37.6 | *(new with §37.)* Not a question about the language as it stands, and listed with a **revisit condition** rather than as an open choice: reconsider if the evaluator ever moves to explicit heap frames for another reason, since that is what makes suspension cheap and nothing else does. They would remove the hand-written slot-and-flag §37.5 charges against the two-verb form, and they are orthogonal to §37.4, which is already shaped like a resumption boundary |
| What `min` and `max` return on a compare-equal tie | §43.4, §34.1, §36.4 | *(new with §43.)* Ties became observable when §43 made compare-equal weaker than `==`: under the strong contract the two answers are indistinguishable, under this one they are different values, so the provided bodies must say which operand wins. Receiver-wins pairs with §24.3's `<<` evaluating to its receiver, but that is a resemblance and not an argument, which is why it is a row here rather than a decision there. §36.4's widened receiver means one answer covers `Num` and user types alike |

---

The stated values, as I understand them going in:

- A love letter to Ruby's expressiveness, Java's structural clarity, and
  Lua/Smalltalk's minimalism.
- Nouns have verbs. Types matter; "everything is a map" is a false economy.
- Names must read fluently and consistently. Not like English, but
  internally coherent.
- Metaprogramming is uninteresting. Local reasoning is paramount. Implicit
  behavior is discouraged.
- Simple abstractions: types and values, no higher-order machinery.
- Conceptual cohesion (few ideas in view at once) and visual cohesion (how
  the code looks on the page) are real design values, sometimes worth
  trading against airtight correctness.

*Three clarifications received later, and each matters more than it looks.*

**"Metaprogramming is uninteresting" means no `method_missing`, no dynamically
defining methods, no DSLs** — things of that nature. It is a statement about
*programs modifying themselves*, not about what may be a value. I read it
more broadly than intended in several places below, most consequentially in
§13.4, and used it to rule things out that it does not rule out.

**The word was doing two jobs, and the third clarification separates them:
introspection is supported; self-modification is discouraged.** Reading what
the source declares — looping over a type's fields, asking whether a type
implements a protocol — is *introspection*, and mesa is for it. Creating names
or behaviour the source doesn't contain — `method_missing`, `instance_eval`,
defining a method at runtime — is *self-modification*, and that is what the
original value rules out.

The test that separates them, and the reason the line sits there: **local
reasoning is paramount — you should never have to think hard about where a name
came from.** A field enumeration hands back names that some `type` declaration
in some file declares, and a reader can go and find it. `method_missing` hands
back a call whose target is written nowhere. One keeps the question answerable
from the source; the other doesn't. That is the whole distinction, and it is
checkable rather than a matter of degree.

*Two things follow from stating it this way.* The positive principle below is
not a separate commitment — **"a name always resolves to a value" falls out of
believing in introspection**, since holding what a name points to is already a
query about it. And "metaprogramming" should probably stop being the operative
word here, because it names the prohibition rather than the belief, and the
belief is the part that decides cases.

I used the loose reading to decline things it does not decline, in §13.4's
third condition (retracted there), in §17.7's refusal of conformance testing
(collapsed, see the note there), and in §31.1's claim that a structural `Hash`
could not be written by hand. **No mechanism for introspection is designed
here, deliberately.** The principle is recorded; the interface waits for
something that needs it.

**A positive principle sits alongside it: wherever you have a name, you can
hold the value it points to.** This is the reason procs and types are already
values (§1) and it extends to protocols, which are therefore not opposed in
principle even though nothing currently needs them to be values. It is a
better tool than the negative rule for deciding these cases, because it says
what mesa *is* rather than what it isn't.

One boundary the principle draws sharply, and in mesa's favour: `$print` is a
name you cannot hold, because it is syntax rather than a binding (§10). That
looked like an exception; it is better read as the sigil's whole job. `$`
marks precisely the names that stand outside the name/value correspondence,
which makes the boundary more principled than §10 claimed, not less.

That last one does a lot of work below, because it's the axis most in tension
with "implicit behavior is discouraged." Concision and explicitness pull in
opposite directions more often than language designers like to admit, and
mesa has already made several calls on that axis that are worth surfacing.

---

## 1. What mesa has already earned

Worth starting here, because most of the sections below are about open
questions, and it's easy to read a long list of open questions as a list of
problems. Several properties are already true of mesa, structurally, and they
are the things most worth *protecting* as new features land. Naming them
makes it possible to notice when a proposed addition would quietly cost one.

**One construction path.** Calling a `Type` value constructs an instance, for
both native and user types, through the same `Expr::Call` arm
(`rt.rs:680-736`), branching only on `TypeId::User` vs `TypeId::Native`.
There is one mental model for "make a thing," not two. The native-data
feature (`Obj::Native(Box<dyn Any>)`, per the recorded design) is exactly the
kind of addition that could grow a second, special-cased construction path
for extension types; it's worth holding it to this shape deliberately.

*Correction, second pass:* this is true of the interpreter but not currently
reachable from script. Native type names are never bound in any scope, so
`Str` is `unbound ident` and `Str()` cannot be written; `NativeType.new` is
dead code from a program's point of view. The unification is real and worth
protecting, but it's currently a property of `rt.rs` rather than of mesa. See
§13.6, where this turns out to gate the `$type` decision.

**Procs are already values, with no separate closure feature.**
`tests/procs.ms` passes `id` to a call of itself. A bound method is a value
too: `f := p.show; f()` works and prints as `def show()`. Mesa didn't need to
add closures; it got them from "procs are `Obj` variants like everything
else." This is a real piece of conceptual economy, and it's why §6 concludes
the remaining block-ergonomics question is much narrower than it first looks.

**An instance's shape is bounded by its type.** `Val::member` (`rt.rs:128`)
returns `None` unless the name is in `inst.fields` or `typ.methods`, and
`Place::Member` turns that `None` into `NoSuchMember`. So you cannot bolt an
arbitrary key onto an instance. `acct.whatever := 5` is an error, not a new
property. That is precisely the "not everything is a map" guarantee, already
enforced by construction rather than by convention, and it's stronger than
what Ruby, Python, or JS give you. (There is one hole in it, discussed in §2,
and the hole is on the method side rather than the field side.)

**No monkey-patching of types.** `UserType.methods` is populated once when
the `type` decl is evaluated and never mutated afterward. There is no
`add_method`, no reopening a type, no extending a native type from script.
Combined with the absence of `eval`, this means the full method surface of
every type is determined by the source text. That's a stronger "no
metaprogramming" property than most dynamic languages have, and it's what
makes the static-resolution property below possible.

**`self` doesn't leak dynamically.** `eval_proc_call` saves and restores
`self.inst` around every call (`rt.rs:1124-1127`), setting it to `None` for a
plain proc. So a proc called from inside a method does not see the caller's
`self`. Mesa avoided the `instance_eval`-shaped confusion by default. Worth
noting because it's an easy thing to accidentally give up later when adding
callbacks.

**Scope resolution is, today, fully computable ahead of time.** This one is
subtle and is the most valuable of the five, because it's a precondition for
planned work rather than just a nicety. §3 works through why, and what two
specific things would break it.

## 2. Types, members, and the one hole in "no metaprogramming"

`UserType.methods` is `HashMap<Sym, Rc<RefCell<Proc>>>`: a flat namespace, no
supertype, no interface, nothing but a name and a body. That's about as
minimal as method dispatch gets, and it's the right minimal for now. But it
means mesa has no notion of "this type conforms to that shape" beyond
structural coincidence: nothing stops two unrelated types from both having
`.balance`, and nothing lets code say "I need something with a `.balance`."

Ruby doesn't say this either, but Ruby has `method_missing` and mixins as
pressure-release valves, both of which mesa has ruled out. Java says it with
`interface`. Given "nouns have verbs" as a stated value — the type *is* the
contract — I'd guess mesa eventually wants something interface-shaped, if
only because "no metaprogramming" removes the usual dynamic-language excuse
for skipping it. But adding `interface` before there's a second real user of
it would be exactly the premature machinery that the minimal-machinery
instinct warns against. Flat structural conformance is a coherent place to
sit while the type system is this young.

*Answered by §31, and the guess above was right about the destination.* Mesa
gets something interface-shaped — protocols, declared with `proto` and asserted
with `impl` — and conformance is testable, so code *can* say "I need something
with this shape." Two departures from what this section anticipated. It arrived
as a generalisation of operator overloading (§13.4) rather than as a type-system
feature, so the "second real user" the minimal-machinery instinct wanted existed
before the mechanism did. And conformance is **declared, not structural**: two
unrelated types with a `.balance` still have nothing in common unless both say
so. That is the stronger reading of "nouns have verbs" — the contract is
asserted by the noun rather than inferred from its shape.

### 2.1 The hole: instance-level method replacement

Here is a behavior that's easy to miss and that `main.ms` is already
exercising deliberately. Because `Val::member` admits any name in
`fields ∪ methods`, and `Member::set` writes unconditionally into
`inst.fields` (`rt.rs:476-481`), and `Member::get` checks `inst.fields`
*before* `typ.methods` (`rt.rs:444-447`), you can replace a method on a
single instance by assigning a proc into its name:

```
type Dog(name)
    def speak()
        "woof"
    end
end

d := Dog("rex")
d.speak := quiet     # quiet is some top-level def
d.speak()            # calls quiet, not Dog#speak
```

Verified: this runs, and the replacement sticks for that instance only. Two
observations follow, and they pull in different directions.

First, this is monkey-patching. Not type-level monkey-patching — the type is
untouched, other instances are unaffected — but per-object behavior
replacement is the thing "metaprogramming is uninteresting" is meant to rule
out. It also directly costs local reasoning: reading `d.speak()` at a call
site, you can no longer tell from `type Dog` what runs, because any code that
touched `d` between construction and here could have swapped it.

Second, and worse for cohesion: **the two cases don't behave the same way**.
A real method called through `Member::get` becomes `Obj::Method(Method::User(
inst, proc))` and `eval_proc_call` receives `Some(inst)`, so `self` is bound.
A proc *stored in a field* is `Obj::Proc`, and `Expr::Call` hands it
`inst: None`, so `self` is not bound. Verified:

```
type P(n)
    def show()
        n            # reads self's field n
    end
end
p := P(7)
p.show()             # => 7
p.show := bare       # bare is `def bare() n end`
p.show()             # => runtime error: unbound ident 'n'
```

So `p.show()` means two structurally different things depending on a runtime
condition that isn't visible at the call site. That's the sharpest
local-reasoning violation I found in the current language, and it isn't the
kind that a careful programmer routes around, because the two spellings are
identical.

**Settled: the hole closes.** `Member::set` rejects a name that resolves to a
method, so instance method replacement becomes impossible. This falls out of
paren-less calls rather than needing a separate decision — once `x.foo`
invokes, `x.foo := v` has no coherent reading, since you can't assign into a
call (§14.6). `main.ms`'s `acct.name := name` stops working, which is the
intended outcome.

The consequence worth carrying forward is in §3.4: this was the single
feature blocking fully static name resolution, so closing it is what lets the
sym table refactor start from a stronger position.

### 2.2 Fields, methods, and the parens question

The field-vs-method boundary is flagged in project memory as "ad hoc." It's
worth pushing on, because it's a naming question as much as a mechanism
question, and naming is where mesa claims to care most.

`Str.size` and `List.size` are *fields* (`NativeField { get: fn(&Val) -> Val
}`) — zero-arg, no parentheses. If a user type wants the equivalent, say
`Account.overdrawn` meaning `balance < 0`, there is no way to express it.
`UserType` has exactly two member kinds: `fields: Vec<Sym>` (always stored,
always constructor-supplied) and `methods` (always called with parens).

So the honest state of this question isn't "undecided," it's **decided one
way for native types and structurally unavailable for user types**. The
computed-parenless-member concept already exists in mesa; script code just
can't reach it. `Str.size` isn't stored state, it's `str.chars.len()`
computed on access (`rt.rs:224-230`), which is precisely a computed property.

That asymmetry is bigger than a convention gap, because it means `Str.size`
and a hypothetical `Account.overdrawn` can't be spelled the same way even if
you want them to look identical on the page.

**Settled: zero-arity methods drop their parens** (§14). That deletes the
question rather than answering it — "field" and "zero-arity method" become
one syntactic idea, and `NativeField` becomes an implementation detail of how
*native* zero-arity members happen to be stored. One mechanism traded for
zero.

*And then the implementation detail went too* (§29.1). `NativeField` and
`NativeMethod` fold into one `NativeMember`, because giving methods an error
channel while leaving fields without one would re-separate at the
representation level exactly what this paragraph merged at the language level.

Its one real cost, accepted knowingly: `x.foo` becomes ambiguous to a
*reader* between "read stored state" and "run arbitrary code." That's the
thing Ruby is regularly criticized for, and §14.4 discusses what bounds it
here.

Worth separating out one thing that reads like part of this question but
isn't: *fields are also the constructor signature*. `rt_print_obj` prints
`type Account(balance)` by walking `typ.fields`, and construction zips args
against that same list. So "field" already means something more specific than
"zero-arg getter" — it means **constructor-provided state**, the noun's
declared shape. That distinction is worth keeping regardless of how the
parens question lands. The two questions just look like one.

*Amended by §26, and the distinction it identifies is what made the amendment
safe.* "Field" no longer means constructor-provided state alone: a body field is
per-instance stored state the constructor doesn't take. Everything else in this
paragraph survives, because §26 puts the line exactly where this one drew it —
the parenthesized list stays the constructor signature and stays what
`rt_print_obj` prints, and body fields are the members that deliberately aren't
in it.

### 2.3 There is no way to ask what type a value is

`Val::type_id` exists internally, `Obj::Type` values are first-class and
compare by identity (so `Account == Account` is true), but there is no
script-level accessor. You cannot write a type test, cannot branch on a
value's type, cannot write a defensive check.

For a dynamically-typed language whose central claim is that types matter,
that's a notable gap. And it collides with something in §9: the most natural
spelling, `x.type`, is a **syntax error today**, because `parse_member_expr`
takes `TokenTag::Ident` and `type` lexes as `TokenTag::Type`. Verified.

So the keyword `type` currently blocks the most natural name for the
type-of-a-value accessor. That's a small, concrete instance of a general
problem worth handling deliberately rather than working around case by case
(§9).

*Settled, and the block is permanent* (item 25). Keywords are never accepted
as member names, so `x.type` stays a syntax error and `$type(x)` is the
accessor (§13.6). The general problem was handled deliberately — by deciding
the keyword set is simply not available for members, rather than by reclaiming
it.

*Reopened as a question of mechanism by §53*, which finds the block cheaper to
lift than item 25 assumed: the sym survives on every keyword token, so the fix
is a parser widening rather than the context-sensitive lexer §50.4 priced.
`$type(x)` stays the accessor on §13.6's own merits either way.

## 3. Declarations, scope, and the shape of a block

This section has the highest ratio of consequence to visibility in the whole
document, because it turns on a structural fact about the grammar that isn't
obvious from reading any single function.

### 3.1 Blocks contain expressions; declarations are top-level only

`parse_def_decl`, `parse_each_expr`, and `parse_when_expr` all build their
bodies by looping on `parse_expr()`. `parse_expr_unit` has no arm for
`TokenTag::Def` or `TokenTag::Type`. Only `parse` (top level) and
`parse_type_decl` (type bodies) call `parse_decl`. So:

```
def outer()
    def inner()      # syntax error: unexpected token DEF
        1
    end
end
```

Verified. There are no local helper procs, and no nested `def` anywhere.
Several things follow that are much larger than the syntax restriction
itself:

**Every `Proc` in mesa closes over the root scope.** `eval_decl` is only ever
reached from `eval`'s top-level loop, at which point `self.scope` is the root
scope, so `Proc.scope` is that same root scope for every proc and every
method in the program. The closure field exists but currently captures
nothing distinguishing.

**The "GC by another name" upvalue analysis is currently vacuous.** That
memo's hard part is identifying which locals are captured by an escaping
closure, so only those get extracted to heap cells before a frame pops. Today
*no* closure escapes, because no closure is created anywhere but the root
scope, which never pops. If nested `def` is never added, the frame-arena
refactor needs no upvalue conversion at all: every frame can be a plain stack
frame, popped unconditionally.

That reframes the sequencing. The memo treats upvalue analysis as work the
refactor requires. It's more accurate to say the refactor requires it *only
if* mesa decides blocks can contain declarations. That decision is a
language-semantics question, it costs nothing to make now, and it determines
whether a substantial chunk of planned interpreter work exists at all. Worth
answering before starting the refactor, not during.

**Whether to allow nested `def` is a real fork, not an oversight.** Arguments
each way, since I don't think it's obvious:

- *For allowing it:* a proc that needs a small helper currently has to
  promote it to top level, which pollutes the module namespace with a name
  that's only meaningful in one place. That's a real cost to "names read
  fluently." It's also the thing that would make procs-as-values genuinely
  useful rather than merely possible — passing a proc is much less
  interesting when every proc must be declared at file scope. (*This argument
  weakened twice.* §16's proc literals took the second half of it, and §27.2
  reports the first half rarely materializes: a helper that recurs belongs in a
  library, and the proc-local helper is a want that mostly doesn't arise.)
- *Against:* it keeps the reasoning above true, keeps `Proc.scope` trivial,
  keeps the sym table refactor simple, and matches the "flat over nested"
  instinct that already shows up in the control-flow preferences. Lua allows
  local functions and pays for it with full upvalue machinery; Java (pre-8)
  didn't, and was fine for a long time.

If nested `def` does land, the interaction to watch is with §3.3: a nested
`def` inside a loop body would capture the *shared* loop scope, which is the
classic JavaScript `var`-in-a-loop bug, and mesa's current loop-scope reuse
would produce exactly that behavior.

**Settled, though not on the axis this section was measuring** (§16). Nested
`def` stays disallowed, and the fork above stays resolved the way it leans.
But proc literals — an `Expr`, not a `Decl` — deliver the "for allowing it"
bullet's benefit anyway: a one-use helper can be written where it is used
without taking a module-level name. What this section got right is that the
*declaration* question was never the real one. The real one was whether a
closure can escape its frame, and §16.2 answers it directly: a literal may
capture but may not outlive its frame, so "the frame-arena refactor needs no
upvalue conversion" stays true. The loop-scope interaction above is also
already defused, since §13.5 gives each iteration a fresh scope.

*Amended by §30, conclusion intact.* A literal now captures **by value** and
may outlive its frame. "The frame-arena refactor needs no upvalue conversion"
is still true, but for the other reason — what escapes carries copies, so no
local is ever boxed. The loop interaction is defused twice over now, since each
iteration's literal holds its own snapshot regardless of scope.

*The closure field outlives its justification, doing a different job than its
name.* §26 gives `UserType` the same field, set from `self.scope` at
declaration, so there are two of them now, and one question about both: what is
a stored scope reference *for* once literals capture by value? Two jobs are
currently indistinguishable in it, because there is one module and one scope
object.

**Reaching the enclosing frame** is the job this section called vacuous, and it
stays vacuous. Named `def`s, methods, and `type`s are creatable only at module
level, so none of them has an enclosing frame; §16's literals do, and §30
answers them with a capture set rather than with the field. That job is deleted,
not migrated.

**Reaching the module and prelude at call time** is the job that survives, and
§30.2 is what keeps it alive: module-level and prelude names are excluded from
capture precisely so they resolve later "through scopes that never pop." A call
frame still has to be routed to the right module scope, and `Proc.scope` is what
routes it — as `eval_proc_call`'s `outer`, the only place it is read.
`UserType.scope` is read once too, for the identical reason: a body-field
initializer runs at construction, arbitrarily far from the declaration, and must
not resolve against the caller's frame. §21.2 doesn't complicate this. Nesting
is namespacing, and the top-level-only rule above is what guarantees a nested
type's declaration scope is the module's, transitively.

So the field is a **module handle** wearing a closure's name, and it is
per-module rather than per-proc or per-type — today every copy of it is the same
`Rc`, and one handle on the `Interpreter` would cover every use.

**What the resolver does to it.** §33's pass replaces the `outer`-chain walk
with static slot assignment, which removes the `Rc<RefCell<Scope>>` shape but
not the question of *whose* slots: an index means nothing without the array it
indexes. With one module that array can be interpreter-global and both fields
vanish outright. After §28 they cannot — a `Proc` or `UserType` reached from an
importing module has to name its own module — so what is left is a `ModuleId`.
That is cheap and carries none of §30's hazard, since module scopes are
evaluated once, cached (§28.4), and never pop.

Related, smaller: an expression in a `type` body parses to `Decl::Expr` and
then hits `todo!()` at `rt.rs:579`, so `type T(a) 1 + 1 end` panics with "not
yet implemented" rather than producing a syntax error. Nested `type` inside
`type` does the same at `rt.rs:569`. Not urgent, but these are the two places
where the parser accepts something the runtime can't express, and both would
be cheap to make syntax errors instead if the answer is "never."

*Both resolved, and neither by "never."* §21.2 gave nested `type` a meaning as
local types; §26 gives the expression slot a meaning as body fields, restricted
to `name := expr`. `type T(a) 1 + 1 end` does become the syntax error suggested
here, since a bare expression has no reading under that restriction.

### 3.2 Implicit declaration, and why it costs twice

Reading a bare identifier (`rt.rs:1047-1063`) checks, in order: is it
literally `self`; then is it a lexical local; then does `self` have a member
by that name. Assigning to one (`rt.rs:848-860`) mirrors it: existing local →
reassign; else `self` has that member → write through; else **declare a new
local in the current scope**.

The upside is genuine. `balance := balance - 10` inside an `Account` method
reads and writes the instance field with no sigil at all. That's the
Smalltalk/Ruby instance-variable feel without Ruby's `@` ceremony, and it
reads beautifully.

The cost is usually described as the typo problem: misspell a field name in a
method and you silently get a fresh local that shadows nothing, evaporates at
return, and leaves the real field untouched. Ruby has exactly this bug class
with `@ivar`, so mesa isn't worse than its ancestor. But mesa claims to be
Ruby *without* the historical warts, and this is plausibly one of them.

What makes it more interesting is that the same rule causes a *second*,
unrelated-looking problem, and one fix addresses both. Because `when` and
`each` bodies each get a fresh `Scope` (`rt.rs:616-619`, `rt.rs:657-660`), a
variable first assigned inside a conditional does not survive it:

```
when true then
    found := 1
end
$print(found)        # runtime error: unbound ident 'found'
```

Verified. The fix a programmer learns is "declare it before the `when`" —
`found := false` first, so `Scope::local` finds it in the outer scope and
writes through. That works, and it's the correct discipline. But note what's
happened: mesa *requires* a declaration-before-use discipline for correctness
in this case, while providing no way to *express* declaration, and no error
when you forget. The language has the obligation without the notation.

So the two problems are one problem, and a distinct first-declaration form
would have addressed both.

**Settled: no declaration syntax.** Bare `x := v` continues to declare on
first assignment, and block scoping stays as it is — conditional expressions
(`x := when c then 1 else 2 end`) are the answer for conditional assignment,
which makes the conditional's result a value rather than an effect.

What that leaves is the accumulator case, which conditional expressions don't
cover: `each` returns nil and there's no fold, so `total := 0` before the
loop stays mandatory. The declare-before-the-block discipline is therefore
required in two places with no notation for it and no error when it's
forgotten. §13.5 proposes the mitigation that remains available: better
diagnostics, where an `UnboundIdent` for a name assigned inside an inner
block earlier in the same proc says so.

### 3.3 Loop scope is shared across iterations

`Expr::Each` creates one `Scope` before the loop and reuses it for every
iteration, re-inserting the loop variable each time (`rt.rs:616-623`). So a
local first assigned inside a loop body persists into the *next* iteration,
then vanishes when the loop ends:

```
each n in [1, 2, 3] do
    when n > 1 then
        $print(acc)    # prints 1, then 2
    end
    acc := n
end
```

Verified. This is a third behavior, distinct from both of mesa's ancestors:
Ruby gives block-locals a fresh binding per iteration; Python has no loop
scope at all, so the variable outlives the loop entirely. Mesa's rule is
"visible for the rest of the loop, gone afterward."

I don't think that's wrong, and it's arguably the most useful of the three
for accumulator patterns. It's worth knowing it's a deliberate-looking third
answer rather than a match for either ancestor, since anyone arriving from
Ruby or Python will guess wrong in one direction or the other. It also
becomes load-bearing if nested `def` ever lands, per §3.1.

### 3.4 Everything above is statically resolvable, today

Pulling the threads together: mesa's name resolution *looks* dynamic (walk
the `outer` chain at runtime, fall back to `self`, declare on demand) but is
in fact fully decidable before execution, given the current feature set.

- A method's `self` type is statically known: methods only exist inside a
  `type` decl.
- A type's member surface is statically known: `fields` comes from the
  constructor signature, `methods` from the decl body, and neither is ever
  mutated afterward.
- Every proc's enclosing scope is the root scope (§3.1), whose names are the
  top-level decls plus top-level assignments, all enumerable from the source.
- Block scopes nest lexically and are never captured, because nothing can
  capture them.

So a resolver could assign every identifier one of {local slot, field of
self, method of self, module-level name, error} at compile time. That's
exactly what the sym table refactor wants, and it's a stronger position than
that memo assumes it's starting from.

**Two things would break it, and both are live questions elsewhere in this
document:**

1. **Instance-level method replacement (§2.1).** Once `p.show` can be either
   a method or a field holding a proc, depending on runtime history, "field
   of self" vs "method of self" is no longer statically decidable. This is
   currently the *only* thing standing between mesa and full static
   resolution, which is a considerably better reason to resolve §2.1 than the
   local-reasoning argument alone.
2. **Any future dynamic member addition or `eval`.** Ruled out by stated
   values, but worth writing down as a property that's being protected rather
   than an accident.

Implicit declaration (§3.2) notably does *not* break it — a resolver can see
every assignment site and compute the same answer the interpreter would. It
makes the resolver's job less pleasant, not impossible.

## 4. Booleans, truthiness, and the missing half of the operator set

Several small things here that only look small individually.

**`and` and `or` return `Bool`, not their operands.** `rt.rs:945-962` returns
`Val::Bool(...)` in every branch. So `5 or 3` is `true`, and `nil or "dflt"`
is `true`, not `"dflt"`. Verified. This diverges from Ruby, Lua, Python, and
JS, all of which return the selected operand and all of which build their
standard defaulting idiom (`name = name or "anon"`) on top of that.

I think this is the right call, and the reason is a nice interaction rather
than a matter of taste. Mesa's truthiness rule is unusual (below), so a
value-returning `or` would compound one surprise with another:
`count or 10` would yield `10` whenever `count` is `0`, which is the exact
bug that makes value-returning `or` a known hazard in JS and Python. Strict
boolean `or` means a condition's type is always `Bool` no matter what flows
into it, which is a straightforward win for local reasoning.

The cost is that mesa has no defaulting idiom at all, and it will feel that
soon, because `nil` is everywhere: a dict miss returns `nil`, an
out-of-bounds list read returns `nil`, an `each` returns `nil`, a `when` with
no matching branch returns `nil`. If defaulting is wanted, the coherent move
given the above is a **dedicated nil-defaulting operator** rather than
overloading `or` — it keeps `or` boolean, and it makes the operation
explicitly about absence rather than about truthiness, which is both more
explicit and more locally readable. That's a strictly better fit for the
stated values than the Ruby/Python idiom is.

*One leg of this gave way* (§18.6). The argument above has two parts, and the
first — that value-returning `or` would compound one surprise with another,
since `count or 10` yields `10` whenever `count` is `0` — depends on `0` being
falsy, which §18.4 ends. The second part, that strict boolean keeps a
condition's type always `Bool`, survives untouched. So the conclusion holds on
one argument rather than two, and the recommendation of a dedicated
nil-defaulting operator over a value-returning `or` is now an ergonomic
preference rather than a hazard-avoidance one. Also worth noting the list of
nil sources this paragraph gives as motivation — dict miss, out-of-bounds
read, `each`, unmatched `when` — is down to the last two.

*And then the conclusion went the other way* (§23.2). Both `and` and `or`
return their operands, so the second leg is spent too: a condition's type is
no longer always `Bool`. What this section wanted — an operation explicitly
about absence rather than truthiness — is given up for the idiom every
ancestor language has, and the dedicated nil-defaulting operator proposed
below is retired rather than deferred. The cost this section would have
objected to survives in exactly one case: `x or default` still defaults when
`x` is `false`.

**Truthiness for numbers is `num.0 != 0.0`** (`Val::is_truthy`, `rt.rs:119`),
so `0` is falsy and every other number, negative included, is truthy. That is
the near-universal rule and needs no defending.

*Superseded by choice* (§18.4). `Num` becomes unconditionally truthy, so the
rule across the language is: `Bool` answers for itself, everything else is
truthy except `nil`. Not because `!= 0.0` was wrong — it isn't — but because
it makes truthiness ask a different question per type, and §18 gives nil a
single job that truthiness can then share rather than interact with.

*Correction.* This section originally claimed the rule was `n > 0.0`, making
negative numbers falsy, and called it the one semantic every ancestor
language would lead a reader to guess wrong. That was wrong, and instructively
so: the stated verification — `when 0 then` takes the else branch — is
consistent with *both* rules, so it confirmed the part that was already true
and never tested the part that was in doubt. `is_truthy` has read `!= 0.0`
since it was introduced in `0b04b91`, which predates this document. Verified
properly: `when -5 then` takes the *then* branch. §0.3 item 22 is closed.

Note also that `Val::Obj(_)` is unconditionally
truthy, so unlike Python/JS an empty list and an empty string are both true.
That part is Ruby-shaped and defensible.

**There is no unary operator at all.** No negation, no logical not.
Consequences, both verified:

- `x := -5` is a **syntax error**. `lex_num` reads digits only, and
  `parse_expr_unit` has no arm for `TokenTag::Minus`. Negative literals must
  be written `0 - 5`.
- There is no `not`. Negating a condition means `when x == false then`, or
  the empty-then trick `when c then else ... end`, which does parse.

*Since built,* in the two commits landed after this section was written:
`a906736` (`not`) and `14fccbb` (unary minus). `parse_expr_unit` now has arms
for `TokenTag::Not` and `TokenTag::Minus`, both routed through one
`parse_unary_expr` helper, with `Expr::Unary(Unary(UnaryOp, ExprId))` as the
node and `Precedence::NOT` (4) and `Precedence::NEG` (8) as the levels.
Verified: `-5`, `-x`, `not true`, and `not not true` all evaluate.

Worth noting the second bullet was answered by a different mechanism than the
one it named. `-5` is unary negation applied to a literal, not a negative
literal — `lex_num` still reads digits only. Nothing observable turns on that
today, but it will if a numeric tower ever arrives, since `-` then applies to
an already-constructed value.

The argument below stands as written; it is recorded here because its
conclusion was acted on, not because it is still open.

The second is the more interesting one for cohesion, because `and` and `or`
are *word* keywords. Mesa currently has a clean and defensible split:
comparisons that *produce* booleans are symbols (`==`, `!=`, `<`, `>`, `<=`,
`>=`), and connectives that *combine* booleans are words (`and`, `or`). That
split is a real rule, and it reads well. But the word-connective set is
missing exactly one member, and it's the one that appears constantly in
conditionals. Given the recorded preference for structural symmetry — matches
over the same enum listing variants in the same order, everywhere — a
two-thirds-complete connective set is the kind of asymmetry that's likely to
grate once noticed.

Also worth noting: `!` exists in the lexer *only* as part of `!=`; a bare `!`
is `UnexpectedChar`. So the one place `!` appears is spelling a concept
(`not`) that the language otherwise expresses in words. Now that `not` has
landed, `!=` *is* the single symbol-spelled negation in a language that
otherwise spells negation as a word — a hypothetical when this was written,
and actual since. Lua's answer was `~=`; Ruby carries both `!=` and `not`.
Neither is wrong, but it's worth picking on purpose, and this question is
raised only here — §0.3 never picked it up.

**Chained comparison degrades safely.** `1 < 2 < 3` parses as `(1 < 2) < 3`
and produces `comparison with non-number true` rather than silently
misbehaving. Verified, and a nice consequence of ordering being `Num`-only.
`a == b == c` does *not* error, since `==` is universal; it silently yields a
comparison against a boolean. A minor wart, and one that would disappear if
`==` were ever restricted, though I don't think it should be.

## 5. Errors and failure

`Signal::Return(Val) | Error(Error)` (`rt.rs:532-535`) reads like a `Result`
in interpreter clothing. `return` unwinds via `Err`; an error unwinds via
`Err`; nothing in the language can catch either. `eval` treats a top-level
`Signal::Return` as early exit and any `Signal::Error` as fatal. So mesa's
error story today is closest to "let it crash," not Ruby's `rescue` and not
Go's returned error.

That's worth sitting with rather than assuming Ruby ancestry means `rescue`
is coming, because "implicit behavior is discouraged" cuts hard against
Ruby's model specifically. A `raise` three frames down, caught by a `rescue`
somewhere lexically unrelated, is close to the least local control flow a
language can have. Ruby-style rescue and local reasoning are in genuine
tension, not merely different tastes.

The Go-shaped alternative fits "explicit" much better but doesn't come free.
Every `Signal::Error` carries a `Location` from the raise site, which is
exactly what you want for crash-and-report and exactly what you lose once an
error is a first-class value that can be returned, stored, and re-raised
elsewhere. Error values would need their own identity (an `Instance`-shaped
thing, implying at least one built-in error type) — *both of which happened*
(§13.3, §19), though the location objection was answered by keeping the
`Location` out of band rather than by making it part of the value, so it is
never reachable from script. What survives of the objection is only the
re-raise case: a caught error stored and raised again loses its original site
unless `raise` is built to carry it, which is the traceback problem and is
deferred. Call sites would need
sugar for "propagate or unwrap" — otherwise every call becomes
`when result.ok then ... else ... end`, which is legible but carries a lot of
visual weight for the common path. That sugar is Rust's `?`, and adopting it
would be an odd note in a language partly motivated by Rust's visual density.
Worth naming as an irony if it comes up, not as a refutation.

The third option costs nothing extra, and I'd argue it deserves to be treated
as the current position rather than as an unfinished state: **errors are
fatal, and that's the design.** For a language aimed at small web apps and
CLI tools rather than systems that must survive partial failure, fail-fast
with a precise `file:line,col` message is a legitimate destination. Framing
it that way changes what the native-method error-channel gap (recorded in
project memory: `fn(&Val, Vec<Val>) -> Val` can only panic) actually needs to
close. It doesn't need a rescue mechanism; it needs a way for a native method
to produce the same fatal, well-located error `Error::ArithNonNum` already
produces. That's a much smaller problem than "mesa needs try/catch" suggests.

Two concrete things that matter more if errors stay fatal, not less:

**There is no traceback.** An `Error` carries one `Location`, the innermost.
`eval_proc_call` has the call-site `Token` in hand and discards it as the
error propagates. For a language where a runtime error ends the program, the
call chain is the primary debugging affordance, and a `Vec<Location>` pushed
as frames unwind is cheap relative to how much it's worth. This is arguably
the highest value-per-unit-of-work item anywhere in this document.

**A failing script exits 0.** `main.rs` prints the error to stderr and falls
off the end of `main`, so the process exit status is success. Verified. For a
language whose stated use case includes CLI tools and shell scripting, that
breaks `set -e`, `&&` chaining, CI steps, and every other convention built on
exit status. Small fix, and unambiguous.

## 6. Iteration and control flow

**`each` is the only loop.** No `while`, no `for`, no `loop`, no `break` or
`continue` anywhere in `TokenTag`. That's a strong and coherent stance: Ruby
accumulated five roughly-equivalent looping forms; mesa has one.

Whether one construct carries the whole load is the real question, and I
think the honest answer is "not forever." `each` iterates a collection, but
plenty of loops aren't over a collection: poll until a condition, retry with
backoff, read until EOF. Ruby's answer is `loop do ... end` plus `break`;
Lua's is `while true do`. Mesa has neither yet. That's scope, not a flaw.

If a condition-driven loop lands, the fork is whether it gets its own keyword
or whether `each` stretches to cover it. I'd lean toward a second keyword,
even though it's one more concept, because visual and conceptual cohesion are
not the same axis here: one clearly-named keyword per clearly-distinct idea
reads better than one keyword whose meaning depends on what follows `in`,
even if the keyword count is higher. There's a related consistency rule
already implicit in the grammar that's worth articulating so future forms can
follow it: **the opener word names the relationship.** `then` introduces a
consequence, `do` introduces repeated action, and a declaration needs no
opener because the body *is* the definition. A `while`-shaped form should
therefore take `do`, and any future multi-branch conditional should take
`then`. That's a real rule, currently unstated, and stating it makes several
future decisions automatic.

**`break` and `continue` are the cheap, obviously-coherent addition.**
They're control-flow escapes exactly like `return`, and `return` already has
a working unwind mechanism they can sit beside: two more `Signal` variants,
two more keywords, no new machinery class. Notably this doesn't depend on
resolving the `each`-versus-`while` question first. It does depend on §5's
framing, in one small way: adding `Signal` variants for loop control means
`Signal` stops being "return or error" and becomes "any non-linear exit,"
which is the natural shape for it, and is also where a future `rescue` would
have to live if one is ever wanted.

### 6.1 There is no `elsif`, and the cost is structural

`When(cond, then_block, Option<else_block>)` has no chain form. Multi-way
branching requires literal nesting, and every level adds an `end`:

```
when x == 1 then
    $print("one")
else
    when x == 2 then
        $print("two")
    else
        $print("other")
    end
end
```

Verified — this runs, and it needs two `end`s for two conditions, four for
four. This is a direct hit on a stated value. The recorded preference for
flat control flow ("flatter loops: fewer concepts, more cohesion") is exactly
what nesting-per-branch destroys, and it's visible in the *shape* of the code
on the page, which is the other half of the same value. A four-way branch in
mesa currently looks like a pyramid with a stack of `end`s at the bottom.

The cheapest fix needs no new keyword and no new AST node: when parsing the
`else` branch, if the next token is `when`, parse a nested `when` expression
as the entire else-block and **do not** consume a second `end`. The tree
stays exactly as it is today; only the concrete syntax flattens. That gives
`else when cond then ...` chains that read fluently, reuse existing
vocabulary, and cost one branch in `parse_when_expr`.

The alternative is a distinct keyword (`elsif`, `elif`). It saves two
characters and adds a word to the keyword namespace, which §9 argues is
already under some pressure. `else when` seems clearly better here, and it's
the rare case where the more cohesive option is also the smaller change.

The related absent feature is value-dispatch (`case`/`switch`). I'd hold that
until §2.3's type-test question is settled, since a large fraction of what
`case` gets used for in Ruby is type dispatch, and mesa can't express the
test at all yet. Solving the smaller problem may make the larger one look
different.

*It did* (§17.5). Type dispatch got its own form — `when e case Expr.Ident
then …` — arriving with case types rather than as a general `case`/`switch`.
So the "large fraction" is served and general **value** dispatch stays held,
now as a genuinely separate feature rather than the same one deferred. Worth
noting the guess above was right about the mechanism and wrong about the
direction: solving the smaller problem didn't reshape the larger one, it
removed most of its motivation.

### 6.2 Blocks as values, and where the block question actually lives

Ruby blocks are its most joyful feature and its most implicit one: implicit
`self` inside a block, `yield` reaching into a method's block without being a
named parameter, and procs/blocks/lambdas as three subtly different things.
Everything about the stated values argues against importing that apparatus.
But the *ergonomic* win blocks chase doesn't require the implicit parts, and
`each x in list do ... end` already gets most of it.

Worth being precise about where the remaining gap actually is, because it's
narrower than "mesa needs blocks." Procs are already first-class (§1). What's
missing is only the *native* boundary: a method implemented in Rust
(`List.each`, `List.map`) cannot invoke a mesa `Proc` mid-call, because
`fn(&Val, Vec<Val>) -> Val` receives no interpreter. That's already in the
known-limitations memory, along with the `RefCell` reentrancy hazard if such
a method holds a `borrow_mut()` across the callback
(`list.each(|x| list.push(x))` being the canonical case).

So Ruby-style block ergonomics on native collections is a scoped Rust
plumbing problem to solve once, not a language-design gap needing new syntax.
And the keyword `each` sidesteps it entirely today precisely because it's
handled inside `eval_expr` (`rt.rs:613-654`) and never crosses the
native-method boundary — which is arguably an independent justification for
`each` being a keyword rather than a method, separate from the readability
argument.

When that native form does arrive, "don't mutate the receiver from inside its
own callback" should be a real runtime error at the point of double-borrow,
not undefined behavior. Rust's borrow checker refusing to compile exactly
this pattern is a decent hint that mesa's dynamic equivalent should refuse it
loudly. That's a case of the implementation language's constraint validating
a language design choice rather than merely limiting it.

*One justification here has since expired* (§20.8). The claim that `each`
"sidesteps [the native boundary] entirely today precisely because it's handled
inside `eval_expr` and never crosses the native-method boundary" stops being
true under pull iteration: `each` will call a user type's iterator methods, so
it crosses the boundary by design. The readability argument for `each` being a
keyword is untouched; the independent one offered here is spent. The
double-borrow point below survives in weakened form — §20.8 notes pull mostly
dissolves the hazard, since a borrow is released before the loop body runs.

**Settled: no block construct, and one piece was missing from this account**
(§16). The conclusion holds — mesa gets block ergonomics without importing
Ruby's apparatus — but the gap was not *only* the native boundary. Two halves
had to close, and this section saw one of them. Paren-less invocation closed
the calling half without being aimed at it: a callback parameter invokes when
referenced (§14.8 clause 1), so the parameter name is the `yield`. What
remained was the constructing half — an unnamed body written at the call
site — which is neither first-class-ness nor Rust plumbing, and which proc
literals supply. The native-boundary point stands on its own and is unchanged
by this.

## 7. Equality, identity, and keys

`PartialEq for Val` (`rt.rs:156-169`) compares `Obj::Str` by content and
every other `Obj` by pointer. So, verified:

```
[1, 2, 3] == [1, 2, 3]     # false
M(1) == M(1)               # false, for any user type M
"x" == "x"                 # true
```

Strings are values; everything else heap-allocated is an identity. That's
Java's model. But Java lets you override `equals`, and mesa — correctly, per
its values — has no operator overloading and no method hooks. So user types
can *never* have value equality. That's not a gap a programmer can close;
it's a property of the language.

This lands awkwardly against "nouns have verbs," because a large fraction of
the nouns you'd model in an application are value-shaped. `tests/fields.ms`
has `Distance(45.7, "m")`. Two distances of 45.7m being unequal is a
defensible rule but an unusual one for a domain-modeling language, and it
propagates: it also means such a value can't be looked up in a `Dict`, since
`Hash for Val` follows the same split.

**Settled: structural equality for user types**, with a protocol system for
opting into built-in behaviour more generally (§13.4). That closes the
question of *whether*, and opens the question of *how*, which §13.4 works
through and §0.3 item 9 tracks.

*Widened by §49 to containers*, which this section's first example is: `[1, 2,
3] == [1, 2, 3]` becomes **true**. `List` and `Dict` stay **unhashable**, which
is how the next paragraph's hazard is kept away from the two values in the
language that are both mutable and have no declaration to opt in at.

The hazard to carry forward: `Hash` has to agree with `Eq`, and once
instances are both structurally hashed and mutable, mutating an instance
that's in use as a dict key silently loses it in its own table. Python
addresses this by convention, Rust by requiring immutability; mesa has
neither lever. §13.4 proposes making structural `Eq` automatic while `Hash`
stays opt-in, so the hazard is opted into visibly rather than fallen into.
Structural equality also needs a cycle answer — `a.x := a` then `a == b`
recurses forever, the same hazard Rust's derived `PartialEq` has on `Rc`
cycles.

*Answered by §24.7: coinductively.* A set of the object pairs under comparison,
assuming equality on revisit, which makes `==` total on cyclic values and makes
sharing invisible. Hashing takes the other route — a fixed node bound, since a
hash cannot be assumed the way an equality can.

## 8. The functional frontier: strings, numbers, collections

Grouping these because they share a cause: `CORE_TYPES` currently registers
`size` fields on `Str`, `List`, and `Dict`, and **zero native methods on
anything**. Every `methods` array in `CORE_TYPES` is empty.

**Lists cannot grow.** `Place::Script` on a `List` requires
`idx < list.items.len()` and errors otherwise, and there is no `push`.
Verified: `l := []` then `l[0] := 1` gives `script index 0 out of bounds`. A
list can only be created at its final length, by literal. This is the single
largest functional gap in the language today, and it's a useful forcing
function: whoever writes `List.push` first will have to answer §2.2's
field-versus-method question and §5's error-channel question, because `push`
needs neither an error channel nor a callback (making it the ideal first
native method), while `pop` on an empty list immediately needs one of them.
Sequencing those two methods deliberately would settle both open questions
with a minimum of speculation.

*Both halves of that plan have since expired* (§20.3). §14 deleted the
field-versus-method question rather than answering it (§2.2), and growth
arrived as the `<<` operator rather than as `List.push`, so the ideal first
native method never gets written. What survived was `pop`, which still forced
the error-channel question and was for a while the only thing that did — until
§18.2's raising accessors and §22.4's `slice` joined it, and §29 answered it.

**Strings are nearly opaque.** No concatenation (`"a" + "b"` is
`arithmetic on non-number a`, verified), no indexing, no iteration, no
interpolation, no case/trim/split. For "a language you'd write a web app in,"
that's the biggest practical hole. It also raises a design question with a
pleasant resolution:

- Making `+` concatenate is a small step toward polymorphic operators, which
  is a small step toward operator dispatch. A separate `..` keeps `+` meaning
  exactly one thing, at some cost to how it reads.
- But **string interpolation may make the question mostly moot**. It's pure
  syntax, no metaprogramming, it's one of the genuine joys of Ruby, and with
  it most concatenation in real code simply disappears. If interpolation
  lands first, the `+`-versus-`..` decision gets made under much less
  pressure, and possibly never needs making. That's the "look for the change
  that removes the need for the feature" move rather than the "pick the least
  bad option" move.

*Settled by §22, and the prediction inverted.* Concatenation is `+`,
hardcoded for `Str × Str` per item 13's non-overloadable rule; `..` is
declined because Lua needed it only for coercion, which mesa doesn't do.
Interpolation did not make the question moot, because it is the piece that got
deferred and `+` is the piece that landed — so the dependency runs the other
way, and interpolation will inherit a desugaring rather than prevent a
decision. Everything else this paragraph lists is answered: indexing is
declined outright (§22.4), iteration goes through §20.8 yielding `Char`s, and
`case`/`trim`/`split` become ordinary native methods once item 32's error
channel exists.

**`Str.size` counts bytes, and shares a name with two element counts.**
`chars: Box<str>`, so `.len()` is the byte length: `"é".size` is 2. Meanwhile
`List.size` and `Dict.size` count elements. So the same name means "number of
elements" on two types and "number of bytes" on a third. Given how much
weight the stated values put on naming, that's worth resolving early — it's
nearly free now and expensive after code depends on it. Rust makes you choose
explicitly; Python and Ruby 1.9+ chose characters; Go chose bytes and made
you ask for runes. Any of those is defensible; the accidental one isn't.

*Settled: characters* (§22.2). Python and Ruby's answer, chosen so that one
name keeps one meaning across `Str`, `List`, and `Dict`. Immutability (§22.1)
makes the count cacheable, so the O(n) scan happens once per string. Bytes are
not lost, only deferred — they return through an opaque `Bytes` type rather
than as a second reading of `size` (§22.6).

**All numbers are `f64`.** No integer type, hence no integer division, and
notably **no modulo operator at all** — `%` doesn't exist in `TokenTag`.
Even/odd, cycling, and bucketing are common enough in the stated domain that
this will be felt. f64-only is a defensible minimalist choice (Lua 5.1 and JS
both shipped it for years, and Lua 5.3 later added integers partly because of
the pain), but it should be a decision on the record. Two consequences worth
noting: indexing truncates silently (`list[1.7]` reads `list[1]`), which is
an implicit coercion sitting inside a language that discourages implicit
behavior; and integer precision quietly ends at 2^53.

*Settled, one answer per part* (§23.1). f64-only stays and is now on the
record, 2^53 included. Modulo waits until something needs it, which is cheap
precisely because nothing is designed around its absence. The truncation ends:
a non-integral or negative index is a `TypeError`, with negatives raising in
order to reserve Python-style negative indexing for later.

**Dict iteration order is nondeterministic across runs.** `Dict` wraps
`HashMap<Val, Val>`, so both `each k in dict` and `$print(dict)` iterate in
`RandomState` order. Verified across three runs of the same script:

```
{"e": 5, "b": 2, "c": 3, "d": 4, "a": 1}
{"b": 2, "d": 4, "e": 5, "a": 1, "c": 3}
{"e": 5, "c": 3, "d": 4, "a": 1, "b": 2}
```

Three separate reasons this matters more than it looks. Ruby hashes have
*always* been insertion-ordered and Python's have been since 3.7, so both of
mesa's closest ancestors would lead a user to expect stability. Second, it's
a local-reasoning problem: the same program produces different output on
different runs, with nothing in the source indicating that. Third and most
immediately, it's a hazard for the golden-file test design — any fixture that
prints a multi-key dict is flaky by construction, and the current fixtures
only avoid it by having one key. Insertion-ordered dicts seem clearly right
here, and the change is contained to the `Dict` representation.

**Dict conflates absent with nil.** A missing key reads as `nil` and there's
no `has`. `main.ms`'s `Flashes` example already depends on this, testing
`pairs[name]` for truthiness to mean "present and set." That's Lua's model,
and it's coherent — but Lua goes one step further and makes *assigning* nil
delete the key, which keeps the two notions genuinely identical. Mesa stores
nil instead, so the two are already distinguishable in principle while being
indistinguishable in practice. If a `has` method is ever added, that
distinction becomes observable and the current conflation becomes a wart.
Worth picking one: Lua's uniformity (nil assignment deletes) or explicit
presence (`has`, and nil is a real storable value).

*Settled: explicit presence* (§18.2), and by a route this paragraph didn't
consider — the miss **raises** rather than returning anything, so the two
notions separate without needing either option. `has` is still wanted, but as
an ordinary presence test rather than as the thing that would have exposed the
conflation. The prediction that adding it would turn the conflation into a
wart is what made raising the obvious move instead.

## 9. Keywords as a namespace

Sixteen keywords: `type def each in do when then else end return self true
false nil and or`. That's Lua-sized and reads well, and word-based block
delimiters are clearly deliberate. But every reserved word is a name taken
away from the programmer, and the cost isn't evenly distributed — it lands
hardest on words that are also natural *field* names in application code.

`parse_member_expr` takes `TokenTag::Ident`, so no keyword can be a member
name. Verified: `x.type` is a syntax error, and `type R(start, end)` fails on
`end`. In a language for web apps and CLI tools, `type`, `end`, `do`, and
`in` are all plausible field names, and `type` is the one you'd reach for
constantly (`event.type`, `account.type`, `token.type`).

There's a clean fix available, and it's the one Ruby uses: **after a `.`,
accept any keyword token as a member name.** There's no ambiguity in that
position — nothing else can follow a `.` — so this costs one line in
`parse_member_expr` and reclaims the entire keyword set for member names.
Ruby lets you write `obj.class` for exactly this reason. Field declarations
in `type T(...)` and parameter lists are a separate question with real
ambiguity, so they'd reasonably stay restricted.

This connects back to §2.3: if `.type` becomes spellable, the most natural
name for the type-of-a-value accessor becomes available, and a type test can
read `x.type == Account` using only mechanisms that already exist
(`Obj::Type` values already compare correctly by identity). Two open
questions, one small parser change.

**Declined** (item 25). Keywords are not accepted as member names after `.`,
so the fix above is not taken and the keyword set is permanently unavailable
for members. Two notes on why the argument didn't carry, since the reasoning
above still reads persuasively:

The connection to §2.3 stopped being load-bearing. `$type` was settled as the
type accessor on its own merits (§13.6), so the second of the "two open
questions" got answered without this change — leaving one benefit rather than
two for the same cost.

And the fix was less complete than this section claims. It reclaims keywords
only for *dotted* access. It explicitly leaves parameter lists restricted, so
the field could never be declared; and §14.4 later made members reachable
*bare*, where a keyword-named member lexes as its keyword regardless of what
`parse_member_expr` accepts. Three things were needed and the one line
delivered one (§20.5). A partial fix for a permanently-spent word is a worse
trade than the full fix this section was pricing.

*The full fix was available and this section mispriced it* (§53). Two of the
three things needed come from one widening rather than three: after `.` and in
a parameter list are both positions where no keyword can begin anything, so the
"separate question with real ambiguity" this section attributes to `type T(...)`
does not exist in `parse_params`. The third is genuinely structural, and only
for words that lead an expression-position construct — which `type` does not.

Second, smaller keyword note: `self` is a keyword in the interner
(`Sym::SELF`) but **not** in `TokenTag` — `lex_ident`'s match doesn't map it,
so `self` lexes as a plain `Ident` and is only special-cased at runtime
(`rt.rs:1048`). The consequence is verifiable: `self := 9` at top level
declares an ordinary local named `self` and prints `9`. Inside a method it
would be shadowed by the real receiver, since the `Sym::SELF` check runs
first. So `self` has three different statuses in three layers: interner
keyword, syntactic identifier, conditional runtime keyword. Nothing breaks
today, but it's the kind of layering mismatch that produces confusing
behavior later, and making `self` a real `TokenTag` (thus a syntax error as
an assignment target) would cost almost nothing.

Third: `return` requires an operand, since `parse_return_expr` always parses
one. So bare `return` is a syntax error and early exit must be written
`return nil`. Given that blocks already yield their last expression
implicitly, `return`'s main job *is* early exit, which is exactly the case
where an operand is least wanted. Making the operand optional would be small
and would fit how the keyword actually gets used.

## 10. Modules and the `$` boundary

The multi-file design is already worked out in detail in memory: per-module
`Scope` reusing the existing `HashMap<Sym, Val>` + `outer` struct,
cross-module visibility as an explicit name-by-name copy rather than
`outer`-chain fallback, and global unqualified `Sym` interning kept so copies
match keys across modules. That's about as explicit as a module system gets,
and the one thing that could have gone implicit — letting the `outer` chain
reach across modules so cross-module access "just works" — was deliberately
rejected to keep `outer` meaning exactly one thing.

*Extended, not replaced* (§28). A module is now also a **value**, so
name-by-name copy is joined by qualified access through the module's members,
and both are needed — `import IO` binds the module, `import IO.File` binds a
member, one dotted-path syntax for both. The rejection above survives intact:
qualified access goes through member lookup, not through `outer`, so that
chain still means exactly one thing. What §28 adds beyond access is that
imports are **static** and the module graph must be **acyclic**.

The question that design doesn't yet answer: **what's a name's default
visibility inside a module, before anything is imported?**

- *Everything public, importer chooses.* Matches Ruby, needs no second
  keyword. Weaker on explicitness: the module's entire internal surface is
  exposed whether or not the author meant it to be, and the author never
  explicitly decided anything.
- *Nothing visible unless exported.* Rust's model. Stronger on explicitness —
  the boundary is declared rather than emergent — and weaker on minimalism,
  since it needs a keyword on the export side to pair with the import side.

Given that the *import* half was already designed to be maximally explicit, I
suspect the export half wants to match. A module system that's ceremonious on
one end and open on the other is a visible seam, and it's the kind of
asymmetry that's easy to notice once both halves are in view. That's an
extrapolation from the existing half-design, not something discussed.

*The extrapolation was wrong* (§27). Mesa takes the first option, and further
than it is stated here — not "everything public" as a default the author could
have overridden, but no visibility mechanism at all. The seam this paragraph
predicted doesn't open, because the asymmetry it feared is between two halves of
one question, and there is now only one half: imports govern what a module
*uses*, and nothing governs what a module *offers*. The objection above — that
"the author never explicitly decided anything" — stands and is accepted, on the
grounds that the decision was never enforceable in the first place (§20.4, §3.4).

**The `$` boundary is a stronger idea than it currently gets credit for.**
`$print` is lexed by a distinct rule and matched by literal string in
`parse_builtin_expr`, so it's syntax rather than a value: it can't be
shadowed, can't be passed around, can't be reached through a variable. That
makes the sigil a visible marker for "this escapes the language's own
semantics," which is a genuinely coherent rule and exactly the sort of
explicitness the stated values ask for.

The question the module design raises is where the boundary should sit as the
library grows. If `$` means "primitive of the interpreter," it should stay
very small — `$print`, plausibly an assert, plausibly a couple of process
primitives — and everything else (string helpers, file IO, collections)
should arrive as ordinary module names subject to ordinary import rules. The
failure mode to avoid is `$` becoming the standard library by default,
because then the sigil stops meaning "escapes the language" and starts
meaning "shipped with it," which is a much less useful distinction and one
that can't be undone once code depends on the spellings.

*The warning held, and the pressure arrived somewhere else* (§20.7). `$` never
grew; the **prelude** did. `Error` and its variants, then the protocol names,
were all heading for a privileged tier for the wrong reason — they ship with
the language — rather than the right one, that mesa's own evaluation rules
refer to them. The answer was this paragraph's, applied to a mechanism it
wasn't written about: keep the privileged tier to primitives, let everything
else arrive as ordinary module names under ordinary import rules. Worth noting
the failure mode is a *shape* rather than a property of `$`, so it will
present again at whatever the next privileged tier turns out to be.

One more interaction: builtins currently bypass the scope chain entirely,
which is precisely why they're immune to shadowing. Module-provided names
won't be. That's the correct difference, but it's worth being deliberate that
`$print` and a future `io.print` would have genuinely different rules, rather
than letting that emerge.

## 11. Concurrency — unstarted, speculative

Nothing here exists, so this is the most speculative section. "System
scripting" as a use case implies something eventually — reading a socket,
waiting on a subprocess, doing two things at once — but async/await is close
to the platonic opposite of the stated values: a call that silently becomes a
suspend point, a coloring problem where `async` propagates up every caller,
and a state machine generated invisibly. It's hard to design something
further from "you can tell what this call does by reading it."

The interpreter's current shape makes this concrete rather than
philosophical. One `Interpreter` owns one `Scope` chain and one
`TypeRegistry`, everything is reached through `&mut self`, and `Val` is built
on `Rc<RefCell<_>>`, which is not `Send`. Shared-memory concurrency would be
a foundational rewrite, not a feature: either `Rc`/`RefCell` become
`Arc`/`Mutex` throughout, or mesa needs a hard boundary that only owned,
unshared data may cross.

That's consistent with "GC by another name" being deferred. Concurrency is
exactly the kind of decision that would change what the frame-arena design
needs to look like, so leaving it unstarted is better than half-designing it
underneath current work.

If a story is eventually wanted, the shape that fits the stated values is
closer to Go's goroutines-and-channels than to async/await: a keyword that
visibly starts a concurrent task, and communication only through an explicit
channel that transfers ownership. The ownership transfer is what makes it
compatible with the current value representation — a value moving through a
channel is by construction not simultaneously reachable from two scopes, so
the `Rc`/`RefCell` problem is sidestepped rather than locked around. It would
still require `Val` to be `Send`, so this is real work rather than a small
addition, but it's at least directionally compatible with "explicit, local,
no hidden state machine," where async/await simply isn't.

A smaller and much nearer question that might arrive first: blocking IO
alone, with no concurrency at all, covers a large fraction of the CLI and
small-web-app domain. It may be worth explicitly deciding that mesa is a
blocking language for now, so the question stops being an open one hanging
over other designs.

## 12. Sequencing

§0.3 lists the open questions. This section keeps only the dependency
structure behind their ordering, which is the part that isn't obvious from
the list itself. (*All of them have since closed*; what follows is the
reasoning that ordered them, not a live queue.)

**One decision determines whether a planned refactor exists.** If a block
cannot contain a declaration (§3.1), the "GC by another name" refactor needs
no upvalue analysis at all, because no closure can ever escape the root
scope. If it can, the full mechanism is required. That's a pure semantics
question, it costs nothing to answer now, and it decides whether a
substantial piece of planned work is on the list or not.

**Two questions get answered by the same small piece of work.** The
field-versus-method boundary and the native-method error channel are both
settled by writing the first real native methods. `List.push` needs neither
an error channel nor a callback; `List.pop` needs one immediately. Doing
those two in that order answers both with evidence rather than speculation,
and incidentally fixes the largest functional gap in the language (§8).

*Superseded* (§20.3). One of the two questions was deleted by §14 rather than
answered (§2.2), and the functional gap is closed by `<<` rather than by
`List.push`. The sequencing insight held for about as long as its premises
did; only `pop` and the error channel remain of it.

**One small parser change resolves two open questions.** Allowing keywords as
member names after `.` (§9) makes `x.type` spellable, which makes the
type-test question answerable using only mechanisms that already exist —
`Obj::Type` values already compare correctly by identity.

*Void* (item 25). The parser change is declined, and the type-test question
was answered by `$type` instead. Worth keeping as a cautionary entry in a
section about sequencing: the leverage here was real but conditional, and it
evaporated the moment one of the two questions found a cheaper answer
elsewhere. A change justified by serving several purposes at once is only as
durable as the weakest of them.

**One decision has already paid off twice.** Closing the instance
method-replacement hole was argued for on local-reasoning grounds, then
turned out to be the only thing blocking fully static name resolution
(§3.4), then fell out of paren-less calls for free rather than needing its
own mechanism. Worth noting as a pattern: the changes that served several
purposes at once were the ones that followed from the language's own stated
distinctions rather than from local convenience.

---

# 13. Second pass: decisions taken, and what they open

Directions taken in response to §§1–12, with the interactions each one
creates. Several of these resolve open questions cleanly. Two of them turn
out to have prerequisites that weren't visible from the first pass, and one
of them (the protocol system) is a new topic large enough that most of this
section is about it.

One structural fact about the grammar shaped several arguments in this
section as originally written, and has since been decided away:

**Mesa had no statement separator, and was newline-insensitive.**
`lex_next` skipped all `is_ascii_whitespace()`, newlines included; block
bodies were parsed by looping `parse_expr` until a terminator, so expressions
were simply juxtaposed and `$print(1) $print(2) $print(3)` ran all three from
one line. §15 reverses this. Arguments below that lean on it are marked.

## 13.2 Chaining conditionals

**Option A — `else when`, no extra `end`.**

```
when x == 1 then
    $print("one")
else when x == 2 then
    $print("two")
else
    $print("other")
end
```

No new keyword, no new AST node, and importantly **no runtime change**: the
parser collects arms in a loop and builds the same right-nested `When` tree
it builds today for hand-nested code. The only change is that the inner
`when`s don't consume an `end`. `When(ExprId, BlockId, Option<BlockId>)`
stays exactly as it is.

**Option B — a dedicated `elsif`/`elif`.** Saves a few characters, costs a
reserved word. Given §9's observation that keyword namespace pressure is
already real (`end` and `type` are unusable as field names), spending another
word to save three characters seems like the wrong trade.

**Option C — a multi-arm form**, arms laid out under a bare `when`:

```
when
    x == 1 then ...
    x == 2 then ...
    else ...
end
```

This is the prettiest option for four or more branches, and it is expressible
now that newlines are significant (§15) — `when` followed by a newline is
distinguishable from `when x == 1 then`.

My preference is still A, on cost rather than on possibility. Option C gives
`when` two forms distinguished only by where the first condition sits, while
`else when` introduces no new form, no new keyword, and no AST change. But
that's a judgement rather than an argument that forecloses C, so this stays
open (§0.3 item 19).

**Settled as A, and §17.5 is what closed it.** The judgement above became an
argument once `when e case … then` existed. With that form in, mesa has three
`when` shapes, and C would have been the only one distinguished by layout
rather than by a keyword — while also being the form whose main use case,
arms under a named scrutinee, C no longer uniquely serves. So the tally that
was "one new form versus none" became "one layout-distinguished form versus
none, for a capability already covered."

Two things worth raising alongside it:

**In expression position, the chain gets awkward.** `when` is an expression
(verified: `x := when c then 1 else 2 end` works, and you noted this is the
intended answer for conditional assignment). A chained version reads
`x := when a then 1 else when b then 2 else 3 end` — fine on one line,
noticeably worse wrapped across several, because the single trailing `end`
sits a long way from the `:=`. Not an argument against A, but worth eyeballing
in real code before committing, since conditional assignment is now a
load-bearing idiom rather than a convenience.

**`when` is spent, and that forecloses `case ... when`.** Mesa uses `when`
for the conditional. Ruby uses `when` for the *arms* of `case`. If mesa ever
wants value dispatch, the natural Ruby spelling isn't available, and the arm
keyword would have to be something else. That's a naming-cohesion
consequence of an early choice that's still cheap to revisit now and won't be
later. It's also a reason to settle whether value dispatch is wanted at all
before more syntax accumulates around `when`.

*Resolved by inverting it* (§17.5). The foreclosure was real but the
conclusion — that the arm keyword "would have to be something else" — assumed
Ruby's ordering. Mesa took `when` for the scrutinee and `case` for the arms:
`when e case Expr.Ident then …`. Both words keep the meaning they already had,
`when` stays the only word that opens a branch, and `case` only ever
introduces a case. Nothing new was spent, and the pressure this paragraph
anticipated never arrived.

## 13.3 Errors: `raise`/`rescue` and postfix `?` are two designs

Taken: some recovery mechanism is wanted; exceptions are appealing; a postfix
`?` is also appealing; explicitness matters.

Worth separating, because these normally solve the same problem twice.
Rust's `?` exists *because* errors are returned values and propagation would
otherwise be manual. Exceptions propagate automatically, which makes `?`
redundant by construction. Picking both usually means picking one and giving
the other a different job.

There is a version where `?` earns a distinct job, and it fits mesa well:
**`?` as rescue-to-nil, not as propagate.** `x := risky()?` evaluates to nil
if the operation failed. That's the majority case of `rescue` — I don't care
why, give me nothing and let me branch — expressed in one character, entirely
within one expression, with no non-local jump. It composes with the existing
`when x then` idiom and needs no block. If exceptions handle the "I need to
know what went wrong" case and `?` handles the "I just need a fallback" case,
they're complementary rather than redundant, and each is doing the thing it's
actually good at.

The cost is that it loads `nil` further, which is becoming a theme (§13.7).

*And that cost is what killed it* (§18.6). Item 1 resolved as absence-only, so
rescue-to-nil would make failure indistinguishable from legitimate absence —
the exact conflation being removed. With its distinct job gone, `?` falls back
to propagate, which the paragraph above already establishes is "redundant by
construction" alongside exceptions. Settled: `raise`/`rescue`, no `?`.

Four questions `raise`/`rescue` forces, roughly in dependency order:

**1. What is a raised value?** A string is cheapest. An instance is the
mesa-shaped answer — nouns have verbs, an error is a noun with a message and
probably some context. Instances means error types are ordinary user types,
which is good for cohesion and means nothing new is needed to define one.

*Settled: an instance of a case type* (§17). That is the instance answer with
the alternation made closed, which is what turns question 2 below from a
design problem into a consequence — and it means question 4's `Signal::Error`
reification is now forced rather than optional.

**2. How does `rescue` select which errors it handles?** This depends on the
type test, and it's why §13.6's `$type` decision matters more than it looks.
`rescue e then when $type(e) == NotFound then ... else raise e end end` is
technically sufficient and genuinely unpleasant — it re-raises manually, it
nests, and it's exactly the pyramid shape §6.1 is trying to eliminate.
Type-filtered rescue (`rescue NotFound as e then`) reads far better but needs
type matching in the grammar, not just a builtin function. So a builtin
`$type` unblocks type *tests* but not ergonomic error *handling*; those are
different requirements and it's worth knowing that before `$type` gets
treated as having settled §2.3.

*Settled by §17.5.* The distinction drawn here was the right one, and the
matching it said was needed arrived from case types rather than from the error
model — so this question cost nothing in the end. The unpleasant construction
above is exactly what `when e case Err.NotFound then …` replaces, with the
scrutinee evaluated once and coverage checked. What is left is only whether
`rescue` embeds the arms or nests a match inside its body; the second
reproduces the pyramid this paragraph objects to, so the first is the obvious
lean.

**3. Are interpreter errors catchable?** This is the fork I'd flag hardest.
`rt::Error` currently has eleven variants (`ArithNonNum`, `UnboundIdent`,
`ScriptIndexOutOfBounds`, …). If user code can rescue them, every variant
needs a mesa-visible type name, and `rt::Error` stops being an implementation
detail and becomes part of the language's public surface — adding a variant
becomes a compatibility question. If user code *can't* rescue them, mesa has
two error systems with different rules, which is the incohesion "one
mechanism" is meant to avoid. Ruby answers yes (everything descends from
`StandardError`); Go answers no (panics and errors are deliberately
different, and the distinction is load-bearing). Both are coherent; the
mixed answer isn't. This is worth deciding before `raise` is designed,
because it determines whether the error hierarchy is a language feature or a
library one.

**Settled: yes.** Mesa answers Ruby's way and has one error system. The
consequences named above are accepted in full — all eleven variants get
mesa-visible names, `rt::Error` becomes public surface, and adding a variant
becomes a compatibility question. The error hierarchy is a language feature.

**Location stays out of band.** A raise site's `Location` is diagnostic
information, not a value: it is never reachable from script, and the error
instance does not carry it as a field. `Signal::Error` carries the location
alongside the value rather than inside it.

That resolves §5's objection — which was that a first-class error value loses
the raise-site location — by keeping the two separate rather than by making
the location script-visible. It also removes what would have been the worst
case for §17.9's no-parent-fields decision: had `loc` been a field, eleven
variants would each have restated it. They don't, so that pressure never
arrives.

The residue is narrower than §5 feared but real: a caught error that is stored
and re-raised elsewhere gets the re-raise site, and the original is gone unless
`raise` is written to preserve it. That is the traceback problem, deferred
deliberately (§0.2).

**A rescue is not a match.** §17.6 makes a match total, so an uncovered variant
demands an `else`. A rescue has to default the opposite way, because an
unhandled error must propagate — that is what exceptions are for. Applying the
coverage rule unchanged would mean a rescue naming two variants silently
swallows every other one. So rescue arms share §17.5's syntax and invert its
totality, which is worth stating outright rather than letting "it's just a
match" carry the design.

**`else` in a rescue is allowed**, and the typo hazard is accepted on Python's
precedent: a bare `except` there catches `NameError` too. Worth noting Python
permits it while discouraging it — PEP 8 pushes toward naming what you catch —
so the mitigation is convention rather than mechanism, which is the same place
Ruby ends up by a different route.

Two things reduce how often `else` is reached for. Errors are grouped (§19), so
a broad catch can name a group rather than falling through to `else`. And
coverage checking still applies *within* whatever the arms do name, so partial
handling is explicit about its partiality.

**Hierarchy is nesting, not subtyping.** Case types are one level deep by
construction, so a two-level error taxonomy is built by a variant *wrapping* an
instance of another case type:

```
type Error
    case TypeError(error) end
end

type TypeError
    case IndexNonNum(val) end
end
```

`Error.TypeError` is a variant holding a `TypeError`. Catching a group is one
arm; catching a specific member is that arm plus a second match on the wrapped
value. That is a two-level pyramid at worst, entered only when specificity is
wanted, and it needs no new mechanism — which is the point. It also keeps the
parent relation exactly as §17.7 describes it, with no subtyping added.

**4. What happens to `Signal`?** It grows from "return or error" into "any
non-linear exit," which is also where `break`/`continue` want to live (§6).
That's the natural shape for it and the variants coexist cleanly. The one
real change is that `Signal::Error` needs to carry a `Val` rather than an
`rt::Error` once errors are script-visible, which is the same reification
question as (3) seen from the implementation side.

One more: **`rescue` makes the traceback (§5) more valuable, not less.** An
error that's caught, partially handled, and re-raised with no record of where
it came from is the standard way exception-based systems become hard to
debug. If recovery is coming, the call-location chain stops being a nice
diagnostic and starts being necessary.

## 13.4 The protocol system

New topic, and the largest thing here. Taken: user types should be able to
opt into built-in behaviour — equality, hashing, subscript, iteration, call —
sitting deliberately between total flexibility and total rigidity.

Mechanically this is small: at each built-in operation site, when the
receiver is an instance, dispatch to a known member instead of erroring. The
design space is entirely in *how a type opts in*, *what's in the set*, and
*what each protocol forces on the rest of the language*.

### Opt-in by declaration, not by name

Two ways to say "this type implements equality":

- **By name.** Defining a method called `==` (Ruby) or `__eq__` (Python) or
  `__index` (Lua) is itself the opt-in. Zero new syntax. But it's implicit in
  exactly the way the stated values push against: nothing in the source says
  "this is a protocol implementation," a misspelling silently doesn't
  implement anything (the same failure mode as §3.2's field typo), and the
  names are permanently reserved in every type's method namespace.
- **By declaration.** Something like `type Point(x, y) impl Eq, Iter ... end`.
  Explicit, checkable at declaration time (the required methods either exist
  or the type declaration is an error), and self-documenting at the place a
  reader is already looking.

The declaration form is the better fit, and it has a payoff beyond
explicitness: **it's the same mechanism §2 predicted mesa would eventually
want for interfaces.** The first pass flagged that mesa has no way to say "I
need something with a `.balance`," and that "no metaprogramming" removes the
usual dynamic-language excuse for not having one. If protocols are declared,
that gap closes with no additional concept — the interface system and the
protocol system are one system viewed from two directions. That's the kind of
consolidation that's much easier to get by designing for it than by
retrofitting.

It also preserves static resolution (§3.4): a type's protocol set is fixed at
declaration exactly like its method set, so nothing becomes runtime-dependent.

### Closed or open set

"Built-in functionality" suggests a closed set — only protocols the language
defines. That's the minimal-machinery starting point and I'd agree with it.
The one thing worth doing now is **picking a spelling that can open later**:
if the syntax is a list of names (`impl Eq, Hash`), allowing user-declared
protocols later is a matter of permitting user names in that list, not a
redesign. Cheap option preservation.

**The option got exercised, and much sooner than "later" implied** (§17.9).
Case types need a way to say that every variant implements `open_chunk`, and
the answer taken was to declare an ordinary protocol carrying that one method
rather than to give case types a header clause of their own. That is a
user-declared protocol, so the set is open. The preservation is what made it
cheap — permitting user names in the list, as predicted, rather than a
redesign. What it did not preserve is a *declaration form* for a protocol,
which a closed set never needed; that is the one genuinely new piece of syntax
this turns up, and it now sits inside §0.3 item 2.

Note that while the set stays closed, §2's interface question stays open —
you can say "this type is comparable" but not "this parameter must be
sortable." That's a fine place to sit, it just shouldn't be mistaken for
having answered it.

### Eq and Hash: the coupling is the interesting part

Structural equality for user types is wanted. The trap §7 identified is that
`Hash` must agree with `Eq`, and once instances are both structurally hashed
and mutable, mutating an instance that's in use as a dict key silently loses
it. Python addresses this by convention, Rust by immutability; mesa has
neither lever.

The protocol system offers a resolution that neither of those languages can
express as cleanly: **make structural `Eq` automatic for all user types, and
make `Hash` an opt-in protocol.**

- `==` on instances just works, uniformly, with no ceremony — which is the
  behaviour wanted, and it makes value-shaped nouns first-class.
- Using an instance as a dict key requires declaring `Hash`, and that
  declaration is a visible promise that this type is used as a value and
  won't be mutated while keyed.
- The hazard becomes something a type author opts into deliberately, at a
  place a reader can see, rather than something every user type falls into.

It is, strictly, an inconsistency — structural equality with non-structural
(or absent) hashing. But it's a *declared* inconsistency rather than a hidden
one, which is the difference that matters, and it lands almost exactly on
"between total flexibility and total rigidity." The alternative worth naming:
hash all instances structurally and accept the universal mutation hazard.
That's more uniform and strictly more dangerous, and §7 already flagged that
this is one of the rare places where the fully-consistent option is the worse
one.

Two smaller consequences of automatic structural `Eq`: it needs a cycle
answer (`a.x := a` then `a == b` recurses forever, the same hazard Rust's
derived `PartialEq` has on `Rc` cycles), and it makes `==` an operation with
unbounded cost, which is worth knowing in a language where `==` currently
never allocates or recurses.

### Subscript: get and set are two protocols

`a[k]` and `a[k] := v` are separate operations, and mesa's native types
already treat them separately — a read past the end of a list returns nil, a
write past the end errors (§12). Splitting them lets a type be readable but
not writable, which is a real and common shape. Merging them saves a name and
forecloses that. I'd split, and name the pair so the relationship is obvious.

### Iteration: push, not pull

This is the hard one, and the two shapes have very different consequences.

**Pull** — the type provides something like `iter()` returning a state object
with `next()`. This needs a way to signal exhaustion. If that signal is nil,
you cannot iterate a collection containing nil, which is a real restriction
in a language where nil is already this common. If it's a distinct sentinel,
that's a new kind of value in a language deliberately short on kinds of
values.

**Push** — the type provides `each(body)` and the interpreter hands it the
loop body. This is Ruby's internal-iterator model, and the key observation is
that **the body never has to be a first-class value**. `Expr::Each` can pass
it as an interpreter-internal callback that script code can't name, store, or
pass on. So mesa gets protocol iteration without adding blocks to the
language surface, which preserves the §6.2 stance rather than quietly
abandoning it.

Push also dodges the sentinel problem entirely, which is the deciding factor:
the pull design's exhaustion signal has no good answer available in mesa's
current value set, and inventing one is a larger change than the iteration
protocol itself.

**Reversed: iteration is pull** (item 10, §20.8), and **re-examined once more
while deciding §34, where it stood.** The re-examination is recorded at the end of
§20.8: push is not an answer to where derived methods live, since a combinator
over `each(body)` still has to be written somewhere — Ruby's `Enumerable` is push
*plus* a mixin of defaults, and the mixin half is what does the work. Two costs
push would now carry are also new since this section: early exit collides with
§29.2's open `break`-across-a-proc-boundary question, and §30's by-value capture
disables the accumulating callback that push combinators lean on. The deciding
factor was
conditional on mesa's value set at the time, and case types changed it — a
sentinel now exists, and §18.5's `Maybe` supplies the specific one, so nothing
had to be invented for iteration's sake. (*§37 declines `Maybe` here*, which
leaves this argument's conclusion intact by a shorter route: the sentinel turned
out to be a `Bool` and a slot, so still nothing had to be invented.) Worth noting the two costs priced
below were the real evidence and pointed the other way all along: pull pays
neither. `break` under pull is simply not calling the iterator again, with no
unwinding through user code, and the `RefCell` hazard evaporates because a
borrow is released before the body runs.

*Superseded, in its argument rather than its conclusion* (§16.5). The
first-class-ness claim above does not survive §14. A user type implementing
`each(body)` *names* the body — it is a parameter — and under §14.6's duality
a callable that is not invocable is exactly the category those rules exist to
eliminate. So the body could not have stayed opaque. With proc literals it
does not need to: the body is an ordinary proc, `each(body)` is an ordinary
method taking an ordinary callback, and the opaque-internal-callback category
never has to exist. Push still wins, but on the sentinel argument alone, which
was always the stronger one. A side benefit: the keyword `each` and a user
type's `each` become the same shape rather than two mechanisms.

Two costs to price:

- `break` from inside a user-defined `each` unwinds through a script-level
  method call. `Signal` handles that correctly, but it means a user's `each`
  implementation must tolerate being abandoned partway through, and any
  cleanup it wanted to do after the loop won't happen. Worth documenting as
  part of the protocol's contract.
- It revives the `RefCell` reentrancy hazard from the known-limitations
  memo, now for user types too: an `each` implementation that holds a borrow
  across the callback while the body mutates the same receiver. §6.2 argued
  this should be a loud runtime error rather than undefined behaviour, and
  that argument gets stronger once user code can hit it.

### Call, Show, Order, and where the line sits

**Call** is a clean fourth arm in `Expr::Call` beside `Proc`, `Type`, and
`Method`. No collision with construction: `Account(...)` calls the type,
`acct(...)` calls the instance. Callable instances plus structural equality
gets a surprising amount of expressiveness without any metaprogramming.

**Show** should confront something already present: mesa has *two* printing
modes today, `rt_print_val` and `rt_debug_val`, where the latter quotes
strings inside collections. That's Rust's `Display`/`Debug` split, already
implemented implicitly. A Show protocol either exposes both (honest, two
methods) or collapses them (simpler, and user types nested inside a list lose
the distinction that native types have). Worth deciding rather than
discovering.

**Order** — one `compare` returning a number is the minimal shape and enables
sorting. Note that mesa has no sort function yet, so this protocol has no
consumer until §8's collection methods exist.

**Arithmetic wasn't named, and that boundary is worth stating out loud.** If
Eq/Hash/Subscript/Iter/Call/Show/Order are in and `+ - * /` are out, the rule
is something like *structure and access, yes; algebra, no*. That's a
defensible and memorable line. Left unstated, it will get relitigated every
time someone wants a `Money` type. It also interacts with §8: if `+` is never
overloadable, then string concatenation has to be a hardcoded native
behaviour or a different operator, which is worth knowing while that decision
is still open.

### The architectural payoff

The reason to design this carefully rather than incrementally: **if protocols
are the dispatch mechanism, the native types' behaviours can be described the
same way.** `List` implements Subscript and Iter; `Dict` implements
Subscript, Iter, and Hash-key; `Str` implements Eq and Hash. That collapses
the currently-hardcoded matches in `Expr::Script` and `Expr::Each` into a
single dispatch path, replacing "which `Obj` variants does this operation
know about" with "which types implement this protocol."

And it gives the **native data feature** something it currently has no answer
for. That memo's third-party native types (a `File`, a `Socket`) can register
methods, but nothing lets them be subscriptable or iterable, because those
behaviours are hardcoded `Obj` matches. Through protocols they'd get both by
the same registration path everything else uses.

So the protocol system, the native fields/methods work now in flight, and the
native data feature are one mechanism at three levels. That's worth
sequencing around: settling the protocol dispatch shape *before* finishing
the native-methods design would let the native side be built as an instance
of it, rather than as a parallel system that later needs reconciling.

### Where the no-metaprogramming line sits

Protocol dispatch isn't metaprogramming as long as three things hold: the
protocol set is fixed at declaration, implementations can't be added at
runtime, and implementations can't be *enumerated or tested* at runtime. The
third is the one to watch, because it's the natural next request — some form
of `x implements Eq` — and it's reflection. Deciding no now is much easier
than deciding no after someone has a use case.

*The third condition was overreach* (see the clarification to the stated
values). "No metaprogramming" means no `method_missing`, no dynamic method
definition, no DSLs — programs modifying themselves. A query is none of those.
The first two conditions do follow from the stated values and still hold; the
third was a preference dressed as a consequence.

Separately, **protocols being values is not opposed**, and follows from the
positive principle that wherever you have a name you can hold the value it
points to — the same principle that already makes procs and types values (§1).
Nothing currently needs it and it isn't being built; §19 deliberately has its
error variants carry protocol *tags* rather than protocol values, because a tag
does that job completely. But the reason is sufficiency, not prohibition, and
this section should not be read as having closed the door.

## 13.5 The loop-scope fix, and what it doesn't fix

Taken: no explicit declaration syntax; block scoping stays; loop scope gets
fixed; conditional expressions are the answer for conditional assignment.

The loop fix (a fresh scope per iteration rather than one shared scope) is
right, matches Ruby, and has a consequence worth noting: it's a prerequisite
for nested `def` being safe. §3.1 flagged that a closure created inside a
loop would currently capture the *shared* scope, reproducing the JavaScript
`var`-in-a-loop bug. Fresh-per-iteration removes that hazard before it can
exist, so this is one less thing the §3.1 conversation has to resolve.

Conditional expressions do cover the case they're meant to (verified:
`x := when c then 1 else 2 end`), and that's a genuinely better answer than
mutable-assignment-from-inside-a-branch — it makes the conditional's result a
value rather than an effect.

The one case they don't cover is the accumulator: `each` returns nil, and
there's no fold or reduce, so `total := 0` before the loop remains mandatory.
That's fine, but it means the declare-before-the-block discipline is now
required in two places (conditionals and loops) with no notation for it and
no error when it's forgotten. Since notation is off the table, the mitigation
available is **diagnostics**: when `UnboundIdent` fires for a name that was
assigned inside an inner block earlier in the same proc, say so.

```
found (assigned at 3,5 inside a block that has since ended)
```

That's computable from the AST without any new semantics, it converts a
confusing error into a teaching one, and it fits the fail-fast-with-precision
posture §5 argues mesa already has. It's the cheapest available answer to the
usability half of §3.2, given that the notation half is settled.

## 13.6 `$type` has a prerequisite

Taken: a `$type` builtin for now, resolve more fully later.

The mechanism is trivial — `Builtin::Type(ExprId)` alongside
`Builtin::Print`, returning the `Obj::Type` value. And type values already
behave correctly for comparison: verified that `A == A` is true, `A == B` is
false, and a type stored in a variable still compares equal to itself, so
`$type(x) == Account` works today as soon as `$type` exists.

But for native values it doesn't work at all, because **native type names are
not bound in any scope** (verified: `Str` is `unbound ident`). So `$type(5)`
returns a `Type` value with nothing in the language to compare it against.
`$type` is inert for every non-user value until the eight `CORE_TYPES` names
are bound somewhere reachable.

Binding them is small and has a side effect worth wanting: it makes `Str()`,
`List()`, and `Dict()` constructible, which activates `NativeType.new` —
currently unreachable from script and therefore effectively dead code. It
also makes §1's "one construction path" claim true of the language rather
than only of the interpreter.

One decision it forces, which connects to §10: names bound in the root scope
are *shadowable*. A program declaring `type Str(...)` would shadow the
built-in. `$print` isn't shadowable because it's syntax rather than a
binding; `Str` would be, because it's a binding. That's a real difference in
kind between two things that both feel like "built-ins," and it's the same
question §10 raises about where the `$` boundary belongs. Worth answering
once, for both.

*It was answered once, for both, and the framing above turns out to be wrong*
(item 4, §20.6). Native type names do not live in the root scope. They live in
a prelude tier above the module, and they are **not** shadowable — not because
they are syntax, but because binding one is a declaration-time error. So the
"real difference in kind" this paragraph identifies isn't shadowability at
all; both tiers are unshadowable. The difference is that `Str` is a **value**
and `$print` is **syntax**, which is a cleaner line and the one §10 should have
been drawn on.

## 13.7 Cross-cutting: nil is accumulating jobs

This surfaced three separate times in this round, which is enough to make it
a policy question rather than a per-feature one.

`nil` currently means: a dict key is absent; a list read is out of bounds; a
`when` matched no branch; an `each` finished. Under discussion, it would also
mean: an operation failed (`?` as rescue-to-nil, §13.3); and, if pull-style
iteration were chosen, a sequence is exhausted (§13.4).

Each addition is individually reasonable. Together they make `nil` mean
"something didn't happen and you can't tell which," which is a local-reasoning
problem of exactly the kind the stated values care about — the value tells
you nothing about its provenance, and the call site can't distinguish a
legitimate nil result from a swallowed failure. It's also why §4 noted mesa
has no defaulting idiom: the idiom is unclear precisely because nil is
ambiguous.

Two coherent policies:

- **Nil is universal absence.** Accept that failure and absence are the same
  thing, keep everything simple, and accept that `x := risky()?` followed by
  `when x then` cannot distinguish "failed" from "succeeded and returned
  nil." Lua-shaped, and internally consistent.
- **Nil is absence only.** Failure gets its own representation, which
  probably means `?` isn't rescue-to-nil after all and the error model leans
  harder on `rescue`. More precise, more machinery.

The thing to avoid is deciding it implicitly, one feature at a time, which is
the current trajectory. This is worth settling before `?` is designed,
because `?`'s meaning depends entirely on the answer.

**Settled: absence only** (§18). The warning above was right about the
sequencing — `?`'s meaning did depend entirely on the answer, and the answer
removed the feature rather than shaping it. Two things this section could not
have known made the expensive branch affordable: case types (§17) supply the
"more machinery" it priced but had no way to buy, and errors carrying real
types absorb the failure jobs. What remains is that nil is still *common* —
an `else`-less `when` is a nil source and always will be — but common in one
meaning rather than six, which was the actual complaint.

# 14. Paren-less calls: a design pass

Taking this one on its own. Constraints set: zero-arity only; it must work in
bare position (`render` inside a method calls that type's `render`); a
reference operator in the spirit of Ruby's `&` is acceptable; and there must
be no caveat distinguishing a method from a field that holds a proc.

That last constraint is the one that does the most work, and following it
produces a better rule than the first sketch's.

## 14.1 Make the rule about the value, not about the member kind

The first sketch proposed "auto-call iff the name resolves to a method," which is
exactly where the caveat came from: the rule mentions *kind*, so kind becomes
observable, so a proc in a field behaves differently from a method. Drop the
mention of kind and the caveat goes with it:

> Resolving a member — however it's written: `x.foo`, `self.foo`, or bare
> `foo` inside a method — invokes it if it is an invocable.

One sentence, no mention of method versus field, and no mention of the dotted
form specifically. A method with no parameters invokes. A field holding a
zero-arity proc invokes. A field holding a number yields the number.

The bare case matters as much as the dotted one. Given `type Box(func)`,
bare `func` inside a method of `Box` **invokes** when the field holds a
zero-arity callable, and does not merely yield it. A member is a member
regardless of how it's spelled, so `func`, `self.func`, and `b.func` from
outside all behave identically. Anything else reintroduces a caveat on a
different axis: you'd be able to observe *how you referred to* a member,
which is no better than being able to observe its kind.

(§14.4 widens this from members to any name, and §14.6 settles what happens
when the arity isn't zero. §14.8 states the finished rules.)

One thing to exclude deliberately: **`Obj::Type` should not count as
callable here**, even though calling a type is how construction works. A
zero-field type (`type Marker end`) is technically a zero-arity callable, and
without the exclusion a field holding `Marker` would silently construct a
`Marker` on every read. Restricting auto-call to procs and bound methods
avoids that. Worth writing into the rule rather than discovering later.

There is still a *semantic* difference between the two cases, and it should
be stated plainly rather than hidden: a method invoked this way has `self`
bound to the receiver; a proc stored in a field does not, because it closed
over its own definition context and was never anyone's method. That's
correct behaviour — a stored proc is a value, not a verb of that noun — and
it's the same rule Ruby applies to a `Proc` held in an instance variable. But
it means the uniform syntax covers two things that behave differently when
they touch `self`. That difference is inherent, not an artifact of the
design, so the honest move is to name it rather than to try to erase it.

## 14.2 Callee position has to suppress the invocation

Immediate consequence that has to be handled or all existing code breaks: if
`x.foo` invokes, then `x.foo()` means "invoke the method, then call its
result." Every `acct.name()` in the codebase would start failing.

The fix is one syntactic exception: **a member in callee position is not
auto-invoked.** `Expr::Call(Member(...), args)` evaluates the member, then
calls it with the given arguments, exactly as today. Everywhere else, a
member auto-invokes if the rule in §14.1 applies.

**This exception has to cover the bare form too**, which the dot-centric
phrasing above missed. Bare `func()` parses as
`Expr::Call(Expr::Ident(func), args)`, not `Expr::Call(Expr::Member(...))`.
If `Expr::Ident` auto-invokes, then `func()` invokes the proc and then calls
its result — the same double-call bug, reached by the other path. So the
rule is: **callee position suppresses auto-invocation, whatever the callee
expression is.** With that, `render` and `render()` mean the same thing, as
do `func` and `func()`, and `f(a)` still calls a one-arity proc held in a
local rather than erroring on the bare reference to it.

This is statically determined by the shape of the AST, needs no new node
type, and gives the property you want: for a zero-arity member, `x.foo` and
`x.foo()` mean the same thing. It's also what Ruby does, where `obj.foo` and
`obj.foo()` are indistinguishable.

It composes correctly with a method that returns a proc: in `x.make()()` the
first `()` is callee position on the member (so the method is called once,
not twice) and the second calls the returned proc.

**One asymmetry this creates, worth knowing about.** Assignment targets are
places, not values, so the auto-invoke rule doesn't apply to the left side:
`func := p` stores `p` in the field, as it should. But that means for a
callable field, reading and writing the same bare name are no longer
inverses. `func := func` invokes the stored proc and replaces it with its
return value, rather than being a no-op. The identity round-trip is
`func := &func`.

This is narrow, but it's the one place the design lets a plausible-looking
expression do something quite different from what it reads like. It doesn't
arise in Ruby because Ruby has no bare field reads to begin with (`@func` is
required), so this is genuinely mesa's to answer. Options are to accept it as
a known sharp edge, or to make assignment from a bare callable member without
`&` an error, which is more explicit but adds a rule to a design whose main
virtue is having one.

## 14.3 The reference operator costs almost nothing

`&x.foo` should mean "the member, uninvoked." That is precisely what
`Expr::Member` evaluation does *today* — `Member::get` already builds
`Obj::Method(Method::User(inst, proc))` for a method, and bound methods
already work as values (verified: `f := p.show; f()` runs and prints
`def show()`).

So the framing is: **you're not adding a reference operator; you're keeping
the current meaning of `.` and giving plain `.` a new one.** `&` needs no new
runtime machinery at all, only a parse form that suppresses the auto-invoke.

That also retires a cost the first sketch raised: that optional parens would
remove bound-methods-as-values with no workaround, and that this was a reason
to pull the nested-`def` question forward. With `&` in the design the
capability is preserved, so that argument goes away.

Two things to pin down:

**Precedence.** `&` must bind looser than `.` so that `&x.foo` parses as
`&(x.foo)` rather than `(&x).foo`. `.` is at `Precedence::MEMBER`, now 9
rather than the 7 recorded here — the comparison and connective levels were
inserted below it since — so `&` sits below that as a prefix form applied to
the completed postfix chain. `Precedence::NEG` is 8 and already satisfies
that constraint, which is a concrete data point for where `&` can sit.

**Prefix parsing now exists.** When this was written mesa had *no* prefix
operators and `&` would have been the first; the argument was that negative
literals and `not` needed the same infrastructure, so it was worth building
once rather than three times incrementally. That is what happened — `not` and
unary minus landed together, sharing `parse_unary_expr` (take the tag, parse
at the operator's level, wrap in `Expr::Unary`). So `&` inherits the
infrastructure rather than paying for it, and the sequencing argument is
spent.

One qualification the shared-infrastructure framing hid, visible now that the
code exists: `&` is not a third instance of the same thing. `not` and `-`
build a node that evaluates its operand and applies an operation. `&` has to
*suppress* the auto-invoke behaviour of its operand rather than add an
operation, so it wants its own node — or a `UnaryOp` variant special-cased in
`eval_expr` — rather than a third arm sitting naturally beside `Not` and
`Neg`. The parse shape is shared; the evaluation shape isn't.

**Landed as its own node.** `Expr::Mention(Mention(ExprId))`, evaluated by
resolving the operand through a new `eval_expr_raw` (the callee-position
suppression `Expr::Call` already had, lifted out so both share it) and
checking the result against the same invocable test `§4.8` uses. The name
picks the use/mention distinction over "reference" — `f` uses the proc, `&f`
mentions it — partly because `Ref` collides with `rt.rs`'s bare-imported
`RefCell` and its `rf` convention for `Rc<RefCell<Obj>>`. `&` stays the
spelling; only the node's name changed.

## 14.4 Bare position: one rule, no kinds

One obvious-looking line to draw here is *member versus variable*: members
auto-invoke, variables don't. It doesn't survive. Given a module-level
`def invoke() … end`, `v := invoke` would bind `v` to the proc; move the
identical `def` into a type body and `v := invoke` inside a method would bind
`v` to its *return value*. Same declaration, same spelling, different meaning
depending on where it sits — the same class of inconsistency §14.1 removes,
relocated rather than eliminated.

Three positions are coherent. Two are ruled out by decisions already taken,
which leaves exactly one.

**(A) Value-based, everywhere.** Any name or member holding an invocable
invokes when referenced; `&` suppresses. No distinction between members and
variables, or between declaration forms.

**(B) Declaration-based.** Names introduced by `def` invoke; names introduced
by `:=` yield. This is roughly Ruby's methods-versus-local-variables split.
But it means a proc *stored into a field* yields while a method invokes —
which is precisely the method-versus-field caveat already rejected in §14.1.
Ruled out by that decision.

**(C) Member versus variable.** Ruled out by the inconsistency above.

So (A) is the only position consistent with both objections, and the two
together turn out to determine the design rather than merely constrain it.
The full rule:

> A name or member holding an **invocable** — a proc or bound method, not a
> type (§14.7) — is invoked when referenced. Zero arity invokes; non-zero
> arity is an error, so supply arguments or write `&`.
>
> `&expr` yields the invocable without invoking it, and is an error on a
> non-invocable.
>
> Callee position suppresses invocation, so `f(a)` and `x.foo(a)` evaluate
> the callee without invoking it first.
>
> Assignment targets are places, so the left of `:=` never invokes.

Four clauses, no kinds, no positional exceptions. Bare `render` inside a
method invokes; bare `invoke` at module level invokes; bare `func` on
`type Box(func)` invokes when the field holds a zero-arity proc. All three
for the same reason.

### What it costs

**A proc held in a variable needs `&` to be passed onward.** This is the
thing (C) was invented to avoid, and it's no longer worth avoiding, because
`&` already exists for the member case. `f := &id`, then `f(5)` to call it
and `&f` to pass it on again.

This breaks one existing test. `tests/procs.ms` is `$print(id(id))`, where
the inner `id` is a bare reference to a one-arity invocable — an error under
the new rule. It becomes `$print(id(&id))`. A small change, and arguably a
clarifying one: `&` marks *behaviour being passed as data*, which is exactly
the kind of thing that benefits from being visible at the use site rather
than inferred from context.

**A bare name can now run code**, and you can't tell from the name whether
`v := lookup` reads a variable or executes a query. That's the standard
critique of Ruby's model and it's a real local-reasoning cost, worth naming
plainly rather than waving past. Three things bound it: the rule is uniform,
so there's one thing to know rather than a set of cases; `&` marks the
exception visibly; and mesa has no `method_missing` and a statically-knowable
member set, so *what* a bare name can invoke is always determinable from the
source. It's also squarely in the Smalltalk tradition, where a bare unary
message is the normal way to say anything at all.

**It does not affect §14.5.** Resolution order still needs fixing — a
module-level `def v(x)` shadowing an instance field `v` still changes what a
bare `v` means inside every method, and uniform invocation doesn't repair
that. If anything it sharpens it, since both readings are now live
expressions rather than one being a plain value.

The supporting argument worth making explicitly: **bare field access already
works.** Verified — `v` inside a method of `type A(v)` reads the field. So
today fields are reachable bare and methods are not, which is an existing
inconsistency between a noun's data and its verbs. Bare method invocation
*removes* an inconsistency rather than adding one. Given that mesa already
accepts implicit `self` for fields, extending it to methods is the more
coherent position, not the looser one.

## 14.5 The blocker: current resolution order defeats this

This is the part that changes the plan, and it's a pre-existing problem that
paren-less calls would sharpen considerably.

`Expr::Ident` checks locals via `Scope::local`, which walks the whole `outer`
chain — and the root scope, where every top-level `def`, `type`, and
assignment lives, is *in* that chain. Only if the walk finds nothing does it
fall back to `self`'s members. So module-level names shadow instance members.
Verified, and it applies to fields, not just methods:

```
type A(v)
    def show()
        v            # expected: the field
    end
end

def v(distinguishing_param)
    "t"
end

$print(A(99).show())    # prints: def v(distinguishing_param)
```

A top-level `def v` silently shadows the field `v` on an unrelated type.
That's already a local-reasoning problem today: a name introduced anywhere at
module level changes the meaning of bare identifiers inside every method of
every type in that module.

Paren-less bare calls make it materially worse, because the shadowed and
shadowing readings stop being merely different values and become *different
kinds of expression*. Adding a top-level `def render` anywhere in a module
would silently change bare `render` inside every type that has a `render`
method from "invoke this object's render" to "reference an unrelated proc."
Action at a distance, across a whole module, with no diagnostic.

The fix is a resolution order of **local variables → self's members →
module-level names**. That's Ruby's order: local variables shadow methods,
but other top-level definitions don't win over the receiver's own methods.
Mesa can express it because the multi-file design already recorded in memory
gives each module its own `Scope` node — so "is this binding below the module
scope" becomes a structural question about where the walk terminates, not a
new concept.

So paren-less bare calls have a prerequisite: **splitting locals from
module-level names in the scope chain.** The module work already implies that
split. This is a reason to sequence the two together, or at minimum to know
that bare-position auto-invoke shouldn't land while a single flat chain still
conflates them.

Worth noting the order also has to keep working for assignment
(`Place::Ident`), which mirrors the read path. Under the new order, bare
`x := v` inside a method would write to `self`'s field before it would write
to a module-level name, which is the behaviour you'd want and is consistent
with how bare field writes already work.

### Does this need the module system specified first?

Less than it looks like. Worth separating what the fix mechanically requires
from what it leaves undecided.

**What it requires is one distinguished node**, and that node already exists.
The chain today is call frame → (block scopes) → root scope, where the root
scope *is* the module. So "stop the local walk before the module scope" is
implementable now: mark the root, have `Scope::local` stop there, then
consult `self`'s members, then consult the module scope. No imports, exports,
visibility rules, or multi-file support are needed to draw that line.

**It generalises without change.** If nested `def` lands (§3.1), a proc's
chain grows intermediate frames, and "everything below the module scope"
still names exactly the right set. If multi-file lands, the recorded design
gives each module its own `Scope` node and copies imported names *into* it,
so the boundary is still one identifiable node and imported names naturally
sit at module tier — below `self`'s members, which is the behaviour you'd
want when a type has a method sharing a name with something imported.

**One tier genuinely is an open module question.** §13.6 needs somewhere to
bind the eight native type names, and the choice is whether they live in the
module scope (shadowable by user code, `type Str(…)` overrides the built-in)
or in a prelude tier above it. So the full order is

> locals → self's members → module names → (prelude?)

and only the last element is unsettled. That's a smaller question than "spec
the module system," and it's the same question as §10's `$`-boundary: what
counts as built-in, and can user code shadow it.

**Settled: a prelude tier, and user code cannot shadow it** (item 4). The
order is

> locals → self's members → module names → prelude

with no parenthetical. Note that "unshadowable" does not follow from the
ordering — it contradicts it. The prelude sitting *above* the module means an
ordinary walk finds a module-level `type Str` first and shadows the built-in
exactly as this paragraph feared. So the prohibition is a separate,
declaration-time check: binding a prelude name is an error rather than a
shadow. That is a third semantic job for the resolver, alongside §16.2's
escape check and §17.6's coverage check.

The guess that this was the same question as §10's `$`-boundary was right, and
§20.6 works through the answer.

**It isn't extra work.** The sym table refactor already needs this exact
boundary — the recorded design says its unit of compilation is one module's
`Scope`, which requires knowing where locals end and module names begin. So
fixing the resolution order is step one of that refactor pulled forward, not
a new task competing with it.

**And it's arguably a conflation rather than a design.** `CLAUDE.md`
documents the intent as *locals* shadowing fields, with an `each` loop
variable as the motivating case, and `tests/shadow.ms` pins that behaviour
down deliberately (a loop variable `x` shadows field `x`, and the field
survives the loop unchanged). Module-level names acquired the same shadowing
power incidentally, by sharing one chain with locals. Which also means the
fix can't be a wholesale reorder — checking members before the chain would
break the documented and tested locals-shadow-fields behaviour. The boundary
is load-bearing.

## 14.6 What this settles, and what it leaves open

**Settles §2.1.** `x.foo := v` has no coherent reading once `x.foo` invokes,
so `Member::set` must reject names that resolve to methods. Instance method
replacement becomes impossible, as predicted.

**Static resolution (§3.4) survives.** Name-to-member resolution stays
static: the member set of a type is still fixed at declaration. The only
thing that becomes dynamic is whether a *field* access invokes, which depends
on the stored value. The sym table refactor resolves names to slots, not
expressions to call-ness, so it's unaffected.

**Bound methods survive**, via `&`. An earlier sketch had them lost with no
workaround, which was the one real cost of paren-less calls; the reference
operator recovers the capability entirely.

**Decided:** `&` as the spelling for now, revisitable. The `func := func`
asymmetry is accepted, on the grounds that it usefully emphasises the
distinction between places and expressions. And `&` applied to anything
non-invocable is an error rather than an identity.

### The last question answers itself

That third decision resolves the arity case, and not in the direction that
first looked right. If `&` errors on a non-invocable, the symmetric treatment
is to make **plain access to a non-zero-arity invocable an error too**. The
two rules become exact duals:

> A member that is an **invocable** is invoked on plain access; `&` yields it
> without invoking. A member that is not an invocable is a value on plain
> access; `&` on it is an error.

Under the yielding version, `&` would have been a no-op on any invocable with
parameters — `x.handler` and `&x.handler` would mean the same thing for a
two-arity proc field and different things for a zero-arity one. That's the
"arity is quietly load-bearing" wart §14.1 flagged, and it disappears here:
`&` always means something, and plain access always means invoke.

Nothing is lost by the stricter reading. For a two-arity proc field you write
`x.handler(a, b)` to call it (callee position, suppressed) or `&x.handler` to
reference it. The only construction that stops working is bare `x.handler`
with no intent to call — which is precisely the case where `&` is the thing
you meant to write.

**One definition this forces.** "Invocable" has to name the same set in both
rules, or the duality breaks. It is *proc or bound method*; §14.7 settles
that `Obj::Type` does not join it.

**Also decided:** `&` binds looser than `.`, so `&x.foo` parses as
`&(x.foo)`. And `&` applies to any expression rather than only to members —
restricting it to members would mean you'd have to know whether a bare name
resolved to a member or to a variable before knowing whether `&` was legal on
it, which is the same observability problem §14.1 and §14.4 exist to avoid.
On a variable holding a proc it's a no-op, which is harmless.

## 14.7 Types are nouns

The last question was whether `Obj::Type` counts as invocable. It looks like
the hardest one, because construction has a fixed requirement — `Marker()`
constructs and `Marker` must not — and §14.4's rule says a bare name holding
an invocable *does* invoke. If types were invocable, `Marker` would construct
on sight.

So types have to be excluded. The question is whether that's a carve-out
needing defence, and the answer follows from the language's own central
distinction rather than from convenience:

> Auto-invocation is for verbs. A type is a noun.

`Marker` names a kind of thing; it doesn't do anything. A method is a verb of
its noun, and invoking it on access is exactly the ergonomic win this whole
feature is for. Constructing an object is not the same act as invoking a
verb, and a member that names a kind should read as naming a kind.

That reframing matters because it changes "invocable" from an extensional
list with an exclusion (*proc or bound method, but not type*) into an
intensional definition (*the things that are verbs*). Same set, but the first
reads as a carve-out that needs defending and the second reads as a
consequence of nouns-and-verbs, which is what mesa is organised around in the
first place. Under that definition there is no carve-out to justify: `render`
invokes because it names a verb, and `Marker` doesn't because it names a
kind.

### Does this push toward `Marker.new`?

Worth taking seriously, because it has real appeal: all three of mesa's named
object-oriented influences spell construction as a message rather than as a
call on the type. Ruby has `Account.new`, Smalltalk has `Account new`, Java
has `new Account()`. Bare-call construction is more the Python/Lua shape.

Under `.new`, types would never be callable, "invocable" and "callable" would
name the same set, and the question above wouldn't arise. `Marker.new` on a
zero-field type would even auto-invoke under the dual rules and read nicely.

Three costs, though, and the first is the one I'd weigh most:

- **`new` becomes a reserved member name on every user type, forever.** Given
  how much weight the stated values put on naming, permanently spending a
  short, ordinary English word across the entire type namespace is a real
  price. It's the same class of cost §9 identified for keywords, applied to
  members instead.
- **Its arity varies per receiver.** `NativeMethod` is
  `{ arity: usize, call: fn(&Val, Vec<Val>) -> Val }` — a fixed arity per
  method. Construction arity depends on which user type the receiver *is*,
  not on the receiver's type, so `new` doesn't fit the existing native-method
  shape and would need special-casing in `Expr::Call` anyway. That undercuts
  the main argument for it, which was uniformity.
- **It costs §1's construction unification**, where calling a type constructs
  for both native and user types through one path. That's one of the few
  pieces of conceptual economy mesa has already earned.

So I don't think consistency forces `.new`. It's a legitimate design with
good heritage, and if it's chosen it should be chosen because
`Account.new(...)` reads better than `Account(...)`, not because the
alternative is incoherent. The alternative is coherent, on a principle mesa
already holds.

**Recommendation:** exclude types from "invocable," on the nouns-and-verbs
reading. A type-valued member yields the type on plain access, and `&` on it
is an error — consistent with the dual rules, and never inconvenient, since
plain access already gives you the thing `&` would have been asked for.

## 14.8 Settled

The rules as agreed, stated once in full.

**Invocable** means a `Proc` or a bound `Method`. Types are excluded (§14.7).

1. **Referencing an invocable invokes it.** This applies to a bare name and
   to a member expression alike, and regardless of how the name resolves —
   local, module-level, field, or method. Zero arity invokes; any other arity
   is an arity error, so pass arguments or write `&`. *Amended by §21.3:*
   once parameters may have defaults, arity is a range rather than a number,
   so the rule is **zero *required* arity invokes** — `def f(a: 1)` invokes on
   reference, with `a` taking its default.
2. **`&expr` yields an invocable without invoking it.** On anything that
   isn't an invocable, including a type, it is an error. It applies to any
   expression, not only members, and binds looser than `.`, so `&x.foo` is
   `&(x.foo)`.
3. **Callee position suppresses invocation.** The callee of a call
   expression is evaluated but not auto-invoked, so `f`, `f()`, `x.foo`, and
   `x.foo()` all do the right thing for a zero-arity invocable, and
   `f(a, b)` still works for one that takes arguments.
4. **Places never invoke.** The left side of `:=`, whether a bare name or a
   member expression, always designates the slot. Consequence: for a
   callable member, read and write are not inverses — `func := func` stores
   the *result* of invoking, and `func := &func` is the identity (§14.2).
5. **Types are callable but not invocable.** `Marker()` constructs;
   `Marker` yields the type wherever it appears; `&Marker` is an error.
   *One exception since* (§17.9): a case-type parent is not callable, because
   an instance is always one of its variants. It still yields the type on
   plain access and `&` on it is still an error, so only the callable half
   changes.
6. **A `def` may omit an empty parameter list.** `def render end` and
   `def render() end` declare the same thing; `params` is empty either way,
   so this is pure surface syntax. It brings `def` in line with
   `parse_type_decl`, which already treats the paren list as optional
   (`type Marker end` parses today, `def render end` does not — verified
   both ways).
7. **`&` is the spelling**, no longer provisional.
8. **A proc literal does not auto-invoke** (§16). Clause 1 is about
   *referencing* a name or member; a literal is a construction, so it yields
   the proc and needs no `&`. This is what makes an inline callback —
   `list.each(def (x) … end)` — read plainly, and it narrows `&` to its real
   job of marking an existing *named* verb passed as data.

Implementing clauses 1–5 correctly requires the resolution order in §14.5,
now complete: `locals → self's members → module names → prelude`, the last
tier unshadowable (item 4, §20.6).

Clause 6 is a better fit than it first appears: with zero-arity methods
called without parens, declaring them without parens makes the two ends
symmetric. Before it, you would declare `def render()` and call `render`,
which puts the parens on the side that has less to say.

### What clause 6 depends on

**Mesa has no grouping parentheses.** Verified: `x := (1 + 2)` is a syntax
error, and so is `$print((1 + 2) * 3)`. `parse_expr_unit` has no `LParen`
arm at all — `(` exists only as the postfix call operator and in declaration
parameter lists. That's why clause 6 is unambiguous in today's grammar: after
`def render` a `(` *cannot* begin an expression, so it can only be a
parameter list.

That's worth recording as a gap in its own right, separate from the present
topic. `(a + b) * c` is currently unwritable, and the only workaround is a
temporary binding. Precedence climbing handles the common shapes, so it
hasn't bitten yet, but grouping is basic enough to belong on §12's list of
unambiguous items. It was missed in the first pass.

**Once grouping parens exist, the two readings need a tiebreak**, since
`def render (a + b) * c end` could be an empty parameter list followed by a
grouped expression, or a parameter list. With significant newlines (§15) the
tiebreak is Ruby's and it's natural:

> A `(` on the same line as the declared name is a parameter list. A newline
> after the name means there is no parameter list.

No caveat, and it applies equally to `type`, which already permits omission
and would otherwise have acquired the same ambiguity. Without significant
newlines the fallback would have been maximal munch — `(` immediately after
the name is always a parameter list — leaving the narrow restriction that a
body starting with a grouped expression couldn't omit the list.

### One consequence for output

`rt_print_proc` always renders `def name(params)`, parens included, so a
proc declared as `def render end` would print as `def render()`. If the
paren-less form is the canonical one for zero arity, printing should follow
it. Small change, and it moves golden-file expectations —
`tests/procs.ms` currently pins `def id(val)`, which is unaffected, but any
zero-arity proc that reaches output would change.

---

# 15. Significant newlines

Taken: newlines become significant. The current insensitivity was incidental
— it made the first parser easier — rather than a design position.

That's worth flagging loudly, because I used newline-insensitivity as a
*premise* in three separate arguments earlier in this document, and treated
it as a structural fact about mesa rather than as an accident. All three need
revisiting. Two of them turn out not to change the outcome; one gets cleaner.

## 15.1 What this reopens

**§13.2's option C is back.** The multi-arm `when` form was ruled out on the
grounds that `when` followed by `x == 1 then` and `when x == 1 then` are the
same token stream. With significant newlines they're distinguishable, so:

```
when
    x == 1 then ...
    x == 2 then ...
    else ...
end
```

is expressible again. I'd still lean `else when`, because option C means
`when` has two forms distinguished only by where the first condition sits,
and `else when` needs no new form at all. But the argument is now a
preference rather than a proof, and since conditional chaining was flagged
for more discussion, it should be reopened honestly rather than left closed
on a premise that no longer holds. The broader claim in §13.2 — that layout
can never carry meaning in mesa — is now simply false and should be read as
superseded.

**Paren-less calls with arguments become expressible.** They were ruled out
earlier because juxtaposition was the only statement separator, so `f g` was
already two expressions. With newline termination, `f g` on one line can mean
`f(g)`. This does *not* change the decision — zero-arity-only was accepted on
its own merits — but it changes its status from forced to chosen. Worth
recording as a decision now, since the constraint that made it automatic is
gone. My read is that it should stay zero-arity-only: Ruby's paren-less
argument calls are a recurring source of its own ambiguity warnings, and the
readability win is much smaller than for the zero-arity case, where the point
is that a verb reads like a property.

**§14.8's maximal-munch caveat disappears.** The `def render (a + b) * c end`
ambiguity was going to require "a `(` immediately after the name is always a
parameter list," leaving the caveat that you can't omit an empty parameter
list when the body starts with a grouped expression. With significant
newlines the tiebreak is Ruby's and it's natural: a `(` on the same line as
the declared name is a parameter list; a newline after the name means there
isn't one. No caveat, and it applies equally to `type`.

## 15.2 What it fixes

Two real hazards in the current grammar, both verified.

**A leading-bracket line is silently absorbed as a subscript.** This one
produces a wrong answer with no error:

```
y := [10, 20, 30]
x := y
[1]
$print(x)        # prints 20
```

`[` is the subscript operator at `Precedence::SCRIPT`, so the postfix loop
continues across the line break and `x` becomes `y[1]`. This is JavaScript's
classic ASI hazard, present in mesa today and *fixed* by making newlines
significant rather than caused by it. The multi-element version
(`[1, 2]`) at least errors, but on the comma, which points at the wrong
place.

**A leading-operator line joins to the previous one.** `x := a` followed by a
line beginning `+ b` yields `3`. Visible rather than silent, but surprising.

More generally, the absence of any statement separator means a syntax error
can cascade well past its cause, because the parser will happily juxtapose
expressions until something is outright unparseable. Termination gives errors
a place to be reported.

## 15.3 The minimal shape

Two pieces, both small.

**One bool on `Token`.** `lex_next` already has a loop that skips whitespace
and comments before producing a token; it can record whether any skipped byte
was a newline and stamp `nl_before: bool` on the token it returns. No new
token variant, no change to the token stream's shape, and no line-number
computation — worth avoiding, since `Source::loc` scans from the start of the
file and calling it per token would be quadratic.

The flag has to survive comment skipping, since `# …` runs to the newline and
the loop then skips the newline itself. That falls out naturally if the flag
is set wherever a `\n` byte is consumed in that loop.

**One rule, shaped like `Precedence::of`.** A newline terminates an
expression only if the preceding token *can end* one:

> `Ident`, `Str`, `Char`, `Num`, `Bool`, `Nil`, `RParen`, `RBrack`, `RBrace`,
> `End` — terminate. Everything else — operators, `:=`, `,`, `:`, `then`,
> `do`, `else`, `(`, `[`, `{` — continue.

This is Go's rule, and it's a `TokenTag` → property match, structurally
identical to `Precedence::of`, which is already the codebase's idiom for
exactly this kind of table.

It only needs to be consulted in `parse_expr_prec`'s postfix/infix loop —
"don't extend the chain across a terminating newline." Declaration parsing is
unaffected, since `parse_def_decl` and friends already start each body
expression fresh.

**One consequence to accept deliberately:** the previous-token rule means you
must break *after* an operator, not before. `total := a +` newline `b` works;
`total := a` newline `+ b` becomes two statements and the second is a syntax
error. Go enforces exactly this; Ruby permits both by also looking ahead.
Looking ahead is more forgiving and meaningfully more complex, and "break
after the operator" is a defensible house style rather than a limitation.
*Amended by §40:* looking ahead turns out to be cheap once it is restricted to
tokens that cannot also start an expression, which most operators are. The
"break after, never before" house style is retired for that subset; `-`,
`[`, and `(` keep it, for reasons §40 gives.

## 15.4 One interaction with an already-accepted item

§12 accepted making `return`'s operand optional. Combined with significant
newlines, that recreates JavaScript's `return` hazard directly:

```
return
compute_something()
```

Under the previous-token rule, `Return` is not in the can-end set, so the
newline does *not* terminate, and the following line is silently swallowed as
the return value. JavaScript has the opposite failure (it terminates and
silently returns `undefined`), but either way the code doesn't mean what it
looks like.

The fix is the one Go uses: **`return` terminates at a newline**, so a bare
`return` is an early exit and an operand must begin on the same line. That's
a special case in the rule, which is mildly unfortunate in a table that's
otherwise uniform, but the alternative is a silent wrong answer in the exact
construct the optional-operand change exists to enable.

Worth noting these two items were accepted independently and neither is
problematic alone. It's the combination that needs the extra clause.

## 15.5 Migration

Checked the existing fixtures and `main.ms` for anything relying on
cross-line expression continuation. Nothing does. The one line that begins
with a bracket (`tests/prec.ms:16`, `[{"id": 123}]`) is the first expression
of a method body, which is parsed fresh rather than as a continuation, so it
is unaffected. No fixture ends a line with a trailing binary operator or
comma.

So the change appears to be behaviour-preserving for everything currently
written, which is a good sign about how much the insensitivity was actually
being used, and consistent with it having been incidental rather than
intended.

*Amended by §40:* re-checked against the same fixtures and `main.ms` after
widening the rule to look ahead as well as back, with the same result — no
existing program's meaning changes, because the tokens admitted to the
forward direction cannot begin an expression today.

---

# 16. Proc literals

Taken: no dedicated block construct; procs gain a literal form; a literal is
always anonymous, since only a declaration binds a name; a literal captures
`self` lexically; `return` inside a literal returns from the literal; a literal
may not outlive the frame it captured from. Spelling deferred.

*The last of those was reversed* (§30): a literal captures **by value** and may
outlive its frame. §16.2 carries the full revision; the rest of this section
stands.

## 16.1 Why this is the whole feature

§6.2 argued the block question was narrower than "mesa needs blocks," because
procs are already first-class (§1) and `each x in list do … end` covers the
common case. §14 then removed a second piece without anyone aiming at it:
under clause 1, a parameter holding an invocable invokes when referenced, so

```
def twice(f)
    f
    f
end
```

makes `f` its own `yield`. No dedicated slot, no `yield` keyword, no
block-versus-proc distinction, and the thing being called is a named parameter
rather than an invisible extra channel.

That left exactly one half of Ruby's block unaccounted for: constructing an
unnamed body inline at the call site. A proc literal is the minimal form of
it, and with it the block question closes without a block construct — the
outcome §6.2 predicted, by a route §14 opened.

## 16.2 The question was escape, not declaration

§0.3 item 3 was phrased as *"can a block contain a declaration?"*, which made
it look like a grammar question. It isn't. What it decides is whether a
closure can escape the frame it was created in, since that is the whole job of
the "GC by another name" upvalue analysis (§3.1).

A literal is an `Expr`, not a `Decl` — `parse_expr_unit` grows an arm,
`parse_decl` is untouched, and `Decl::Def` still cannot appear in a block. But
the letter of item 3 was never the point:

```
def make_counter(n)
    def (x) n + x end
end
```

escapes, by the other path.

**Decided: it may not.** A literal may capture, but a proc may not outlive the
frame it captured from. That covers the callback case in full — a callback
runs *during* the call it was passed to, so the enclosing frame is still live
and no heap cell is required — and rejects the returned-closure case.

Two consequences:

- **The GC refactor keeps its happy path.** Every frame stays a plain stack
  frame, popped unconditionally, with no upvalue conversion. §3.1's
  observation that the analysis is "currently vacuous" stays true rather than
  turning into a debt.
- **It substitutes a smaller job for a larger one.** Rejecting escapes is not
  free — it needs escape analysis — but it is meaningfully cheaper than
  supporting them, and §3.4 already argues the resolver sees everything
  required. Call it the second checking job for the sym table pass.

The line also happens to be one the stated values already draw. *Simple
abstractions: types and values, no higher-order machinery* — combinators are
built out of closures that outlive their frames, so a proc that cannot is a
callback mechanism and not a functional substrate. Worth noting the
restriction is the value expressed as a mechanism, rather than an
implementation convenience that happens to align with it.

**`Proc.scope` becomes load-bearing.** §3.1 noted the field "exists but
currently captures nothing distinguishing," since every proc closes over the
root scope. Building a literal with the *current* scope instead is most of the
mechanical change.

---

*Revised by §30, and this is the largest reversal in the document.* A literal
**captures by value** and **may outlive its frame**; the prohibition decided
here is removed.

What this section got right is that the callback case it examined needs no heap
cell. What it missed is that "callback" names two populations — the immediate
one it priced, and the *stored* one (test registration, routing tables, event
handlers, factories) that it blocked without noticing. The named-proc path hid
the gap, since a named proc escapes freely and `&` passes it uninvoked; the
restriction only bit when a stored callback needed to capture, which is exactly
where naming doesn't substitute.

Both consequences above survive in altered form. The GC refactor still keeps its
happy path, but because escaping literals carry copies rather than because
nothing escapes (§30.3). And the resolver still does the smaller job, but it now
*computes* capture sets rather than *rejecting* escapes — the same analysis put
to a different use.

The one argument that doesn't survive is the paragraph below about combinators.
Under §30 a proc can outlive its frame, so the mechanism no longer expresses
"no higher-order machinery." Nothing else in the document depended on it, and
the stated value was never that closures are forbidden — §1 records procs as
values approvingly. Read that paragraph as a rationalisation that fit while the
restriction stood.

## 16.3 `self` is captured lexically

The naive reading — a literal is a plain proc, so `eval_proc_call` hands it
`inst: None` (§1) — makes literals nearly useless in the place they are most
wanted:

```
type Account(balance)
    def report(items)
        items.each(def (x) balance := balance + x end)
    end
end
```

Under `inst: None`, `balance` is unbound; or worse, it silently writes a
module-level name under §3.2's declare-on-assign rule. That is the same
failure §2.1 verified for a proc stored in a field, which produced
`unbound ident 'n'`.

**Decided: a literal captures `self` lexically.** §14.1 supplies the reasoning
without having been aimed at this. It justified a field-held proc *not* seeing
`self` on the grounds that it "closed over its own definition context and was
never anyone's method." A literal written inside a method body did close over
a context that has a `self`, so capturing it is the consistent reading of that
rule rather than an exception to it. The two cases stay distinguishable by
where the proc was written, which is exactly what "closed over its own
definition context" means — a literal later stored into a field keeps the
`self` it captured, because capture happens at creation.

Worth stating plainly what this is and isn't. It is Ruby's block-`self`. It is
not `instance_eval`: nothing rebinds `self` from outside, the binding is fixed
by where the literal appears in the source, and §1's "`self` doesn't leak
dynamically" survives intact. The half of Ruby's behaviour the stated values
object to is the dynamic half, and this takes only the lexical half.

## 16.4 `return` returns from the literal

`eval_proc_call` catches `Signal::Return`, so a `return` inside a literal
unwinds to the literal's own boundary and no further.

**Decided, and worth recording as chosen rather than inherited.** Ruby
distinguishes procs, whose `return` returns from the enclosing method, from
lambdas, whose `return` returns from themselves; that split is one of its
genuine warts. Mesa having a single answer is a small piece of conceptual
economy, and it falls out of the existing call machinery rather than needing
anything new.

`break` from inside a user-defined `each` is a separate matter and is
unaffected — it still unwinds through a script-level method call, so an `each`
implementation must tolerate being abandoned partway through (§13.4).

## 16.5 What it cleans up

**Most of §14.4's `&` cost.** The recorded cost of rule (A) was that a proc
held in a variable needs `&` to be passed onward, with `tests/procs.ms`
becoming `$print(id(&id))`. A literal is a construction rather than a
reference, so it does not auto-invoke and needs no `&`. The common case — an
inline callback — is written plainly, and `&` narrows to what it is actually
good at: marking an existing *named* verb being passed as data. This needs a
clause in §14.8, since clause 1 is phrased about names and members and a
literal is neither.

**§13.4's iteration special case.** That section justified push iteration
partly on the claim that "the body never has to be a first-class value… an
interpreter-internal callback that script code can't name, store, or pass on."
That was written before §14 and does not survive it: a user type implementing
`each(body)` names the body, and under §14.6's duality a callable that is not
invocable is precisely the category the rules exist to eliminate.

Literals dissolve the tension in the simpler direction. The body is an
ordinary proc, `each(body)` is an ordinary method taking an ordinary callback,
and the opaque-internal-callback category never has to exist. Push iteration
survives on its remaining argument, which was always the stronger one — pull
needs an exhaustion sentinel and nil cannot be it (§13.7) — and the keyword
`each` and a user type's `each` become the same shape rather than two
mechanisms.

*And that argument fell too* (§20.8). §17.8 supplied the sentinel from case
types, leaving push with nothing standing, and item 10 re-decided as **pull**.
This paragraph's cleanup still holds on its own terms — proc literals do
dissolve the opaque-callback category — it just turned out to be tidying a
mechanism that wasn't kept.

## 16.6 Shape, ahead of spelling

Spelling is deferred, but the structure is decided. There are four forms, from
two positions crossed with the parameter list being optional:

| | with parameters | without |
| --- | --- | --- |
| **declaration** | `def name(a, b) … end` | `def name … end` |
| **literal** | `def (a, b) … end` | `def … end` |

**Position decides which row applies, and it decides completely.** In
declaration position a name is *required* — the parser takes an `Ident` rather
than looking ahead for one — so a name has nothing to be ambiguous with there.
In expression position a name is not *permitted*: `Decl::Def` is what binds
names, declarations are top-level only (§3.1), and the `f` in `x := def f … end`
would have no scope to live in. An `Ident` after `def` in a literal therefore
begins the body, and `x := def ident end` is an anonymous proc returning
`ident`.

*Correcting how this section originally put it.* It described the name and the
parameter list as two independently optional pieces of one construct, and gave
the disambiguation as "after `def`, an `Ident` means named." That rule would be
needed only if a named literal existed, and it would spend the entire class of
literals whose body begins with a name — `def ident end` — to buy a form with
nothing to bind. The four forms are two constructs each with an optional
parameter list, not one construct with two optional pieces.

**The parameter list is the one place an ambiguity survives**, and it is a real
one, because position does not settle it: both readings are legitimate for a
literal. Once grouping parentheses land (§0.2), `def (a + b) * c end` is
ambiguous exactly as §14.8's clause 6 was, and §15.1's newline tiebreak has no
name to hang off. So it becomes "a `(` on the same line as `def` is a parameter
list" — the same rule relocated from the name to the keyword — and a literal
whose body opens with a parenthesised expression needs a newline after `def`.

**A literal has no name to call itself by**, which makes recursion a `def`
declaration. §30.4 reaches the same limit from capture-by-value — a proc-local
name is snapshotted before the literal is bound to it — but it is structural as
well, since there is no name in the form at all. Admitting a named literal later
to close the gap would resurrect the ambiguity above, and the rule that resolved
it would take `def ident end` with it.

Reusing `def` costs nothing against §9's keyword-namespace pressure, which a
new word would.

*True of the current regime only* (§53.5). Under contextual keywords the
literal is what makes `def` an expression head, which moves it out of the class
of words recoverable as bare member names and rules out the one-token lookahead
that class depends on — this section's own "an `Ident` after `def` begins the
body" is the conflicting rule. The cost is nil in practice, since no one wants
a member named `def`; what it demonstrates is that the recoverable class is a
whole-grammar property rather than a property of declaration keywords, and it
gives `B7` a column.

---

# 17. Case types

Taken: a type may declare a closed set of variants. Variants come first in the
parent body and each `case` clause is closed by `end`. The parent body may
hold ordinary methods, which is how behaviour common to all variants is
expressed. Variant names are always written qualified. Consuming a case type
is primarily ordinary method dispatch; `when … case … then` handles the
operations that don't belong on the type.

## 17.1 The declaration

```
type Source
    case File(path)
        def open_chunk()
            …
        end
    end
    case Text(body)
        def open_chunk()
            …
        end
    end

    def read_all()
        # implemented once, in terms of open_chunk
    end
end
```

The parent declares no parameters of its own; every field lives on a variant
(§17.9). The parent body holds methods only.

Three ordering and delimiting rules, and they are not independent — they are
the resolution of a trilemma. *Variants first*, *`end` optional on a bodiless
variant*, and *parent members allowed* are pairwise compatible and jointly
impossible. The obstruction is a bodiless variant followed by `def`, which is
either that variant's body or the parent's first method:

```
type Expr
    case Lit(lit)
    def loc() 1 end
end
```

That is not ambiguous — counting `end`s to the end of input resolves it
uniquely, since with one trailing `end` only the bodiless reading balances and
with two only the body reading does. It is worse than ambiguous for a
hand-written parser: it needs unbounded lookahead, so it does not fit
recursive descent, and it destroys error locality. A missing `end` anywhere in
the file would be reported at EOF with everything between parsed under the
wrong reading, which is precisely the cascade §15.2 cites as a reason for
significant newlines.

**Resolved by requiring `end` on every `case` clause**, which keeps the other
two. Worth recording why that was the right one to give up: the argument for
optional `end` leaned on §6.1's objection to a stack of `end`s, and that was a
bad analogy. §6.1 is about `end`s that *nest*, accumulating at the bottom of a
four-way branch. Variant `end`s are sequential, one per clause, all closing at
the same depth. Nothing accumulates, so the saving was cosmetic, while losing
either parent methods or variants-first would have been structural.

## 17.2 Parent bodies are safe because they reach variants through verbs

The concern that made parent bodies look expensive was field access: in a
method on `Expr`, bare `name` exists on `Ident` and not on `Lit`, and mesa's
implicit field access (§3.2) assumes one known field set per type — that is
what makes `balance := balance - 10` work, and it is load-bearing.

That concern does not apply to parent methods, and the reason generalises.
A parent method reaches its variants through **verbs, not fields**:

```
def read_all()
    # loops calling open_chunk
end
```

`open_chunk` bare-invokes through `self`'s members under §14.8 clause 1, and
every variant has it. The parent never touches variant-specific state, so the
union-field problem never arises for it. Fields stay per-variant; behaviour
generalises over the verb surface. This is the same property that makes
method dispatch workable at all (§17.4), applied to the parent.

Three alternatives were considered and are recorded because each fails for a
different reason:

- **Union bare access** — every field of every variant nameable on the parent.
  Rejected. It splits what `fields` means, which §2.2 identifies as currently
  denoting exactly one thing: constructor-provided state, zipped against args
  at construction and walked by `rt_print_obj`. It makes field layout
  tag-dependent, so §3.4's "field of self" stops being a single answer and the
  sym table refactor either dispatches on tag before indexing or pays a union
  layout on every instance. It needs a representation for "this variant hasn't
  got that field," which lands on §13.7's nil policy as a sixth job for nil and
  the worst-behaved one — a type error wearing a value's costume. And the
  write path either errors (a second read/write asymmetry, cf. §0.3 item 30)
  or partially reopens the hole §1 celebrates.
- **Destructuring in match arms** — rejected as a second layer, correctly: it
  puts pattern syntax in a position that otherwise holds expressions, which is
  a new grammatical category rather than a derivation. §17.5 shows it isn't
  needed.
- **Composition instead of a parent body** — a separate wrapper type holding a
  variant. This needs *nothing* new and is what a language with no supertype
  would naturally say. It stays the right answer when the shared behaviour is
  genuinely a different noun; it is the wrong one here, because it splits one
  noun across two names and puts a wrapping step at every use.

## 17.3 Names are always qualified, and types gain members

A variant is written `Expr.Ident` everywhere it appears, including in match
arms. No bare form, no context-sensitive shorthand: one spelling means one
thing, and there is never a question of whether `Ident` denotes this variant
here and a different one there.

The alternative — binding variant names at module scope — was rejected on a
concrete collision rather than on taste. Module-scope variants land in the
module tier of §14.5's resolution order, which is exactly where §13.6 wants to
bind the eight native type names. So

```
type Token
    case Num(v) end
    case Str(v) end
end
```

would silently shadow the built-in `Num` and `Str` module-wide, and those are
precisely the words wanted for token and AST variants.

**Qualified access needs one new capability: types having members.** That is
genuinely new — `Val::member` currently handles instance fields and type
methods, not members of a `Type` value. But it needs no new *rule*, because
§14.8 already covers what happens: clause 1 invokes a member if it is an
invocable, and clause 5 excludes types from that set. So `Expr.Ident` yields
the type and `Expr.Ident("x")` constructs, both falling out of the settled
rules. §14.7's "types are nouns" earns its keep a second time here — had types
been invocable, a qualified variant name would have constructed on sight.

`.` rather than `::`: `::` is a new token and a new concept (namespace access
as distinct from member access), where `.` reuses what exists and already
means "member of," which for a variant is accurate rather than a stretch.

The cost is a repeated parent name at every construction and every arm. That
is the price of the property, and it is paid knowingly.

## 17.4 Dispatch is primary; matching is for what doesn't belong on the type

The intended way to consume a case type is an ordinary method call. `e.show()`
dispatches, bare `name` inside `Ident`'s own method is safe with no new rule,
and nothing is taken apart at the use site. This is the Smalltalk-shaped answer
and it is what "nouns have verbs" says to do.

The trade being taken is the OO side of the expression problem: adding an
operation means touching every variant, where matching lets you add one in a
single place. Adding a *variant* is correspondingly cheap. Mesa's stated values
lean this way, so this is the consistent choice rather than a convenient one,
but it is a real trade and worth naming as such.

What the closed set contributes on top of ordinary types is therefore not
dispatch — it is exhaustiveness, meaningful type identity, and a declaration
that documents the alternation in one place.

## 17.5 Matching

```
when e
    case Expr.Ident then e.name
    case Expr.Lit   then e.lit
    else nil
end
```

`when` stays the only word that opens a branch; `case` only ever introduces a
case, in a type body when declaring one and in an arm when selecting one. Two
words, each meaning one thing.

**It is LL(1), with no dependence on significant newlines.**
`parse_when_expr` already parses `when`, an expression, then an unconditional
`take(Then)`. It becomes: parse `when`, an expression, then peek — `then` is
the conditional exactly as today, `case` is a match. One token of lookahead
after the expression, one new branch where there is currently none, and the
form works on a single line.

This is a better distinguisher than the alternative considered, a bare
`case e` opener followed by arms, which relied on layout. It also weakens
§13.2's standing objection to a multi-arm form — that "`when` has two forms
distinguished only by where the first condition sits." Here they are
distinguished by an explicit keyword in a fixed position.

**No binding form is needed, and this is the design's best economy.** Inside
an arm the variant is known, so field access is ordinary field access:
`e.name` works because `e` is an `Ident` there. This requires nothing new.
§1 records that `Val::member` returns `None` unless the name is in
`inst.fields ∪ typ.methods`, and `Place::Member` turns that into
`NoSuchMember` — so `e.name` on a `Lit` errors through machinery that already
exists, and in an `Ident` arm the check simply never fires. All three
candidates for reaching variant state collapse: no union access, no
destructuring, no flow-sensitive narrowing. The price is `e.name` rather than
a bound `n`, which is also more explicit about where the value came from.

The scrutinee evaluates once, which is the concrete improvement over the
`$type` chain §13.3 calls "genuinely unpleasant": `when f() case … end` calls
`f` a single time.

## 17.6 Exhaustiveness comes from the arms, not the scrutinee

The natural assumption is that the checker looks at the scrutinee's type. It
cannot. §3.4's static resolution covers *names*, not *values* — mesa is
dynamically typed, so a resolver looking at `when e` has no idea `e` is an
`Expr`.

It can read the arms instead. If every arm names a variant of the same case
type, that type is the intended scrutinee and coverage is decidable on the
spot, purely syntactically:

- All arms are variants of one case type → coverage is checked.
- Arms name unrelated types → no coverage check; it is a type-test chain.

That degrades gracefully, and it lets the same construct do plain type
dispatch (`case Num then`) once native type names are bound (§13.6), without
needing a second form.

**`else` is kept, with a rule that preserves the signal it would otherwise
cost.** The Rust `_` failure mode is that adding a variant silently falls into
the catch-all. Inverted:

- All variants covered *and* `else` present → error, the `else` is unreachable.
- Some variant uncovered → `else` required.
- Non-exhaustive arm set → `else` optional; no match yields nil, consistent
  with §13.7's existing "an unmatched `when` returns nil."

The dead-`else` error does the work. Add a variant later and a
previously-unreachable `else` becomes live rather than silently absorbing it.
It costs nothing beyond the coverage check already being run, and it fits §5's
fail-fast-with-a-precise-location posture.

This is the first genuinely *semantic* job for the resolver that §3.4
describes — until now its work was name-to-slot assignment. It joins §16.2's
escape check.

## 17.7 What it costs structurally

**A parent relation, which mesa has not had.** §2 records "no supertype" as a
current property, and this is the first thing to qualify it.

I estimated earlier that this makes `TypeId` non-flat. That was pessimistic.
`TypeId` can stay exactly as it is if each variant is an ordinary `UserType`
carrying a parent pointer, with `UserType` gaining `parent` and `variants`.
`Val::type_id` returns the variant's own id; method lookup checks the
variant's methods and then the parent's. That single fallback step is the
whole new dispatch rule, and a variant method shadowing a parent method is
ordinary override.

§1's "an instance's shape is bounded by its type" survives, with the bound
widened from `inst.fields ∪ typ.methods` to include the parent's methods —
still fixed at declaration, still enumerable from source, still nothing that
can be bolted on at runtime.

**Structural equality (§13.4) must compare the tag**, so `Expr.Ident("x")` and
`Expr.Lit("x")` are unequal despite matching field values. Straightforward,
but it has to be stated or the field-wise implementation is wrong.

## 17.8 What it opens

Case types touch four items on the working list, which is more than anything
else considered so far.

- **Item 1, the nil policy.** §13.7 frames the choice as nil-as-universal-
  absence (cheap) against nil-as-absence-only ("more precise, more
  machinery"). Case types are that machinery, arriving for independent
  reasons, which makes the more precise branch affordable for the first time.
- **Item 7, rescue selection.** That item says a `$type` builtin "unblocks
  type *tests* but not ergonomic error *handling*; type-filtered rescue needs
  matching in the grammar." §17.5 is that matching. If a raised value is an
  instance of a case type — item 6 — then item 7 is answered by a construct
  that exists for other reasons. The mechanism is settled; the decision to use
  it still depends on item 6.
- **Item 10, push versus pull iteration.** §13.4 chose push because pull needs
  an exhaustion sentinel and "inventing one is a larger change than the
  iteration protocol itself." A case type is a proper sentinel. That
  justification is now available, so the choice should be re-made rather than
  inherited. *It was, and it flipped* — item 10 closes as pull (§20.8), with
  §18.5's `Maybe` as the sentinel, so this prediction cashed out directly.
  (*Half-refunded by §37*: the flip stands, but the sentinel is not a case type
  after all, so case types were what made the choice re-askable rather than what
  answered it.)
- **Item 19, conditional chaining.** Three `when` shapes are now conceivable:
  `when c then`, `when e case … then`, and §13.2's option C. The first two are
  distinguished by a keyword; option C would be distinguished by layout alone.
  That argues for settling item 19 as `else when` and dropping C, since the
  case where arms under a scrutinee are most wanted is now served by a
  better-signposted form.

It also bears on §2's interface question without closing it. A sealed
alternation answers "this is one of these known things"; a protocol answers
"this has a `.balance`." §13.4's claim that protocols close the interface gap
stands independently.

## 17.9 The three remaining questions, settled

**The parent takes no parameters.** All fields are declared on the variants.
The prefix form was rejected on how it reads at the construction site, and the
concatenation reading that would have rescued it is declined too — so shared
state repeats:

```
case Ident(loc, name) … end
case Lit(loc, lit) … end
```

That is real duplication across a wide alternation, and it buys the thing
§2.2 asks for without qualification: a variant's field list *is* its
constructor signature, one concept and not two, printable by `rt_print_obj`
and zipped against args at construction exactly as every other type is.

*One consequence follows that wasn't stated when this was decided, and it is
the first of its kind in mesa.* A parent with variants cannot be constructed —
`Source()` has no coherent meaning, since an instance is always one of the
cases. So a case-type parent is a type that is **not callable**, which is an
exception to §14.8 clause 5 ("types are callable but not invocable") and the
first crack in §1's "one construction path." The crack is narrow — one
construction path still serves every constructible type, and the parent is
simply not one — but it means `Obj::Type` splits into constructible and
abstract, and `Expr::Call` needs an arm that rejects the latter with a real
error rather than producing a tagless instance.

**`$type(e)` returns the variant**, `Expr.Ident`, not `Expr`. So
`$type(e) == Expr` is false and there is no way to ask "is this some kind of
`Expr`."

That is deliberate rather than a gap, and it lands on a line mesa has already
drawn. §13.4's no-metaprogramming boundary holds "as long as … implementations
can't be *enumerated or tested* at runtime," and flags that the natural next
request is some form of `x implements Eq`, which is reflection. Asking whether
a value belongs to a parent alternation is the same request wearing different
clothes. Refusing both is one decision, not two. Nothing currently designed
needs it: dispatch goes through verbs, and §17.6's coverage check reads the
arms rather than the scrutinee.

*This argument has since collapsed at both ends, and the conclusion is
reopened.* It rests on §13.4's third condition — that implementations can't be
enumerated or tested — and §13.4 has itself retracted that condition as
"a preference dressed as a consequence," since a query modifies nothing and so
isn't metaprogramming. §31.3 then made conformance **testable**, which is
precisely the `x implements Eq` this paragraph refuses. So the shared
justification is gone, and "refusing both is one decision" now cuts the other
way: having accepted one, parent membership no longer has an argument against
it, only an absence of demand. §31.3 records it as genuinely open. What survives
here is the narrower observation that nothing *designed* needs it — dispatch
goes through verbs, and coverage reads the arms.

**Required verbs are not a case-type feature.** A parent's `read_all` calling
`open_chunk` does imply every variant must have `open_chunk`, but that
requirement gets no header clause of its own. The case is rare enough to be
served by declaring a protocol carrying just `open_chunk` and having the
variants implement it — the general mechanism, not a special one.

That is the right shape, and it has a consequence worth being explicit about:
**it opens the protocol set.** §13.4 chose a closed set — "only protocols the
language defines" — as the minimal-machinery starting point, while
deliberately preserving the option: "if the syntax is a list of names
(`impl Eq, Hash`), allowing user-declared protocols later is a matter of
permitting user names in that list, not a redesign." This is that later,
arriving sooner than expected, and the preserved option is what makes it
cheap. It does still need a *declaration* form for a protocol, which §13.4
never designed because it didn't need one. That is now part of §0.3 item 2
rather than a separate question.

---

# 18. The nil policy

Taken: nil means absence and nothing else. Out-of-bounds list reads and missing
dict keys raise rather than returning nil. Truthiness collapses to a single
question. Optionals, where wanted, are case types.

## 18.1 One job

§13.7 recorded nil accumulating jobs — absent dict key, out-of-bounds read,
unmatched `when`, finished `each`, plus proposed "operation failed" and
"iteration exhausted" — and warned that together they make nil mean "something
didn't happen and you can't tell which."

This is the "absence only" branch that section named. What makes it affordable
now and not then is that the alternative representations exist. §13.7 framed
the choice as universal-absence (cheap) against absence-only ("more precise,
more machinery"); case types (§17) are that machinery, and errors carrying
real types (§13.3) are the rest of it.

Worth recording why the jobs accumulated, since it wasn't carelessness:
returning nil avoided inventing error types at a time when nothing could catch
them. That was a sound holding position. It stops being one once errors are
meant to be recoverable and a closed alternation can carry a type.

## 18.2 What stops being nil

An out-of-bounds list read raises. A missing dict key raises.

The dict change closes §0.3 item 18 in the direction §8 predicted. That section
observed mesa "stores nil instead [of deleting], so the two are already
distinguishable in principle while being indistinguishable in practice," and
that adding `has` would make the distinction observable and the conflation a
wart. Now `d["k"] := nil` stores nil, reading it back gives nil, and a genuine
miss raises — three distinct cases with no further rules, and Lua's
assign-nil-deletes uniformity is not needed.

The list change closes §0.3 item 30, which recorded a read/write asymmetry:
reads return nil, writes error. Both raise now. The asymmetry was an artifact
of having nowhere to put a read failure.

**Two native methods become obligatory rather than optional.** Every language
that raises on a dict miss ships `get(key, default)` and `has(key)`, because
optional-config reads are otherwise unusable and the workaround is two
lookups. §0.3 item 16 treats `push`/`pop` as the forcing function for the
native-method error channel — its other half, the field-versus-method boundary,
was deleted by §14 (§2.2, §20.3). Dict `get` and `has` join that set, and `has`
is the one §8 predicted would be needed.

## 18.3 What stays nil

Nil remains the value of a proc that returns nothing, a search that finds
nothing, an unmatched `when`, and `each`.

The third can't be legislated away, and it's worth knowing why. `when` is an
expression and mesa has no statement/expression distinction, so
`x := when c then 1 end` and a `when` sitting last in a proc body are the same
construct in the same position. You cannot require an `else` "only when the
value is used," because the value is always potentially used. So an
`else`-less `when` is a nil source, and after item 19's `else when` resolution
that is most of them. **Nil stays common. It stays common in one meaning
rather than six**, which is the whole of what this buys.

One conflation is accepted knowingly: mesa has no `Void`, so "returned
nothing" and "found nothing" are the same value. Swift distinguishes them
(`Void` against `Optional.none`). Named as accepted rather than unnoticed.

## 18.4 Truthiness becomes the same question

**`Bool` answers for itself; every other type is truthy except `nil`.**

Ruby's and Lua's rule, both stated influences. `0`, `""`, and `[]` are truthy.
`Val::Obj(_)` was already unconditionally truthy, so only `Num` changes.

The point is not primarily the bug it fixes. Today truthiness asks a different
question depending on what it is handed — *is this nonzero* for `Num`, *is this
true* for `Bool`, *does this exist* for `Obj` and `Nil`. Three questions behind
one syntax. Under the new rule it asks one: **is this present.** And since nil
now means exactly *absent*, truthiness and nil's meaning stop being two
concepts that interact and become one concept with two spellings. `when x then`
reads "when x is present," and that is literally what it computes.

Structurally this is §14.1's move again: the rule stopped mentioning kind, and
the caveats went with it.

**What it fixes.** A search returning `0`, or a zero `count`, no longer reads
as absent. That was the larger half of the hazard, and it is the case §4 was
right about even though its specific claim was wrong (see the correction
there).

**What it leaves.** `false`. A search that legitimately finds `false` is still
indistinguishable from one that found nothing. Ruby has exactly this and has
lived with it for thirty years, which is fair evidence it's tolerable — but
the residue exists and shouldn't be described as fixed.

**Migration is one line.** `tests/when.ms:21` pins `when 0 then` taking the
else branch. (*Not the only one* — `tests/bool.ms:21` prints `not 0` expecting
`true`, and pins the same rule. Found later, while checking this claim rather
than by it.)
`main.ms:30`'s `when pairs[name] then` tests a dict read for presence and needs
revisiting regardless, since that read will now raise.

## 18.5 Optionals, and where enforcement actually lives

Swift's guarantee is not that nil has one meaning. It is that an optional
cannot be *used* without unwrapping. Mesa cannot have that from a type system
it doesn't have, but it can have much of it from something it does:

```
type Maybe
    case Some(val) end
    case None end
end
```

A match on that is coverage-checked at resolve time (§17.6), so the `None` arm
cannot be forgotten — and under the settled `else` rule, a match already
covering both variants cannot carry a defensive `else` either. That is static
enforcement at the use site, with no type system involved.

The limit, stated plainly: nothing requires a given search to return a `Maybe`
rather than nil. Swift's force comes from the declared return type. So mesa
gets **opt-in optionals, enforced at every use of an API that opts in**, and
which APIs opt in is convention. That is the strongest position available to a
dynamically typed language and it should not be mistaken for Swift's.

Truthiness cohesion and un-ignorability are therefore separate wins from
separate mechanisms, and neither substitutes for the other.

*Demoted by §37.3.* Everything above still describes what a library **may** do,
and `Maybe` needs no language support to do it — it is an ordinary case type.
What is withdrawn is the standing this section gives it. §18 spent itself making
nil mean exactly one thing and truthiness ask exactly one question; offering a
second spelling of *absent*, opted into by convention, is §13.7's problem
inverted — one job acquiring two values rather than one value acquiring six
jobs. §18.4's rule is the answer to absence, and is the only one. The
un-ignorability win named above is real and is not being claimed as free: what
§37.3 concludes is that it is not worth two spellings of the same question, not
that it was imaginary.

## 18.6 What it closes and what it forces

**Closes item 5 — `raise`/`rescue`, no postfix `?`.** §13.3's entire case for
keeping both rested on `?` earning a distinct job as rescue-to-nil: "the
majority case of `rescue` — I don't care why, give me nothing and let me
branch." Under absence-only that is exactly the conflation being removed, since
it makes failure indistinguishable from legitimate absence. §13.3's own
analysis then applies unchanged: exceptions propagate automatically, which
"makes `?` redundant by construction." If `?` is ever wanted it needs a third
job — rescue-to-a-case-type, say — and that is a different feature.

**Reframes item 23, the defaulting idiom.** §4 gave two arguments for
strict-boolean `and`/`or`. The first was that "a value-returning `or` would
compound one surprise with another: `count or 10` would yield `10` whenever
`count` is `0`." §18.4 evaporates that hazard, since `0` is truthy. The second
— "a condition's type is always `Bool` no matter what flows into it" —
survives, so strict boolean stays defensible but is no longer over-determined.
A dedicated nil-defaulting operator is still the better answer, because it
tests `== nil` exactly and stays correct under any truthiness rule; the case
for it is now ergonomic rather than hazard-driven, which is weaker footing
than §4 assumed.

**Settled item 8, in the direction it was pushing.** The reason for raising
rather than returning nil is that these failures are meant to be recoverable,
which presupposes they are catchable. Answering "no" would have put the two
most common recoverable failures in the uncatchable system. Item 8 is now
closed as yes (§13.3 question 3): one error system, `rt::Error` as a case type
with mesa-visible variants, and the dict-miss and out-of-bounds errors added to
it as ordinary members rather than as a second class of failure.

## 18.7 The sequencing cost, taken knowingly

These errors arrive before recovery does. Until `rescue` exists §5's framing
holds — errors are fatal — so an out-of-bounds read that used to yield nil and
limp onward now ends the program.

That is the reverse of the holding position which produced the conflation, and
it is the right direction, but the window between the two is real. It raises
the value of the two items §5 already flags: a non-zero exit status, and
call-location traceback, which §5 calls "arguably the highest
value-per-unit-of-work item anywhere in this document." Both are on §0.2 and
neither depends on anything open.

---

# 19. The built-in error taxonomy

Taken: interpreter errors are rescuable (§13.3), so the error set becomes
public surface and needs to be a set someone would choose rather than one that
accumulated. Naming follows Python's conventions. Grouping is by nesting, since
case types are one level deep (§17.7). Raise-site `Location` is diagnostic only
and never a field.

## 19.1 The current set grew rather than being designed

Eleven variants, and the names describe the *check that failed* rather than the
kind of failure: `ScriptNonScriptable`, `ScriptNonIndex`, `ArithNonNum`,
`CompareNonNum`, `IterNonIterable`, `CallNonCallable`. Six of the eleven are
"you gave the wrong kind of thing to X," differing only in what X was. That is
a reasonable way to write error variants when nothing can catch them and the
only consumer is a `Display` impl. It is a poor public surface.

## 19.2 The collapse

Python's scheme groups by *what kind of mistake it is*, not by which operation
caught it. Applied to mesa's eleven, plus the dict-miss error §18.2 adds:

| Current variant | Group | Why |
| --- | --- | --- |
| `CallNonCallable` | `ProtocolError` | receiver doesn't implement Call |
| `IterNonIterable` | `ProtocolError` | receiver doesn't implement Iter |
| `ScriptNonScriptable` | `ProtocolError` | receiver doesn't implement Script |
| `CompareNonNum` | `ProtocolError` | receiver doesn't implement Order — once Order is a protocol |
| `WrongArgCount` | `ArgumentError` | the call's shape doesn't match the callee's signature |
| `ScriptNonIndex` | `TypeError` | the list *is* scriptable; the index isn't a number |
| `ArithNonNum` | `TypeError` | not a number, where numbers are required by fiat rather than protocol |
| `NoSuchMember` | `MemberError` | |
| `AssignReadOnlyMember` | `MemberError` | |
| `ScriptIndexOutOfBounds` | `IndexError` | `OutOfRange` once §23.1 splits the group |
| `UnboundIdent` | `NameError` | |
| *(new, §18.2)* dict miss | `KeyError` | |

Seven groups, and the third column is the organising principle: **errors are
grouped by why the operation failed, not by which operation caught it.**

An earlier pass at this had five groups with seven of the eleven landing in
`TypeError`, and treated that concentration as the finding — faithful to
Python, where `TypeError` is deliberately broad. It was an artifact. A real
group was hiding inside it, and once protocols are visible in the source the
distinction is visible too: *you used an operation on something that doesn't
implement it* is a different mistake from *you gave that operation the wrong
argument*.

**A second group was hiding in there too**, found the same way — by adding to
`TypeError` and watching the addition refuse to fit (§19.3). Once keyword
arguments and defaults turn one arity check into four,
`ArgumentError` separates out and `TypeError` is left with two variants rather
than seven.

Worth recording the pattern, since it has now happened twice: **`TypeError`
was the residual category, not a real one.** Both times, something added to it
turned out to belong to a group that didn't exist yet, and both times the
symptom was a naming asymmetry rather than an obvious modelling error. After
this split it finally isn't residual — what remains is two variants that say
the same thing (`*NonNum`) about the same kind of mistake.

**`ProtocolError` is a group in mesa that isn't one in Python**, and that is
the justification rather than a departure. Python has no protocol
declarations, so "not callable" there really is just a type mismatch. Mesa
will have conformance declared in the type (§13.4), so an error naming an
unimplemented protocol points at something a reader can find in the source.
Same reasoning that made `MemberError` beat `AttributeError`: import the
grouping principle, not vocabulary for concepts mesa doesn't share — and add
groups for concepts mesa has that Python doesn't.

**`MemberError`, not Python's `AttributeError`.** Python's name fits a language
whose objects have attributes. Mesa's types have *members*, which are fields or
methods (§2.2), and that word is already settled in the implementation —
`Val::member`, `Member::get`, `Expr::Member`, and commit `ab781f9` renamed
"Access" to "Member" specifically for consistency. Importing Python's grouping
is worth doing; importing a vocabulary mesa has already rejected is not.

**`NameError` is deferred rather than adopted.** The current variant is
`UnboundIdent`, and "ident" is mesa's *syntactic* term — `TokenTag::Ident`,
`Expr::Ident`, `parse_ident_expr`. What fails here is not the ident, which
parsed fine, but the resolution of the *name* it denotes; and "name" is already
the design vocabulary for that — §14.5's "module names," §3.4's "name
resolution," the sym table refactor. On that reading `NameError` is the
correction rather than a Python import, and `UnboundIdent` is the one using a
syntactic word for a semantic failure. Left open regardless, to be settled
with the module spec (§10), where "names in a module" and "names in a scope"
acquire a precise distinction that this error should match.

## 19.3 Nesting only where there is something to nest

```
type Error
    case ProtocolError(error) end
    case ArgumentError(error) end
    case TypeError(error) end
    case MemberError(error) end
    case IndexError(error) end
    case NameError(name) end
    case KeyError(key) end
end

type ProtocolError
    case NotIterable(val) end
    case NotAccessible(val) end
end

type ArgumentError
    case Missing(name) end
    case TooMany(max, got) end
    case Unknown(name) end
    case Duplicate(name) end
end

type TypeError
    case IndexNonNum(val) end
    case ArithNonNum(val) end
    case NotCallable(val) end
    case NotInvokable(val) end
end

type MemberError
    case Missing(val, name) end
    case ReadOnly(val, name) end
end

type IndexError
    case OutOfRange(index) end
    case NonIntegral(index) end
end
```

*`IndexError` gained its variants in §23.1*, after this section was written;
the paragraph below on one-member groups is amended there.

The `Script` spelling follows the code (`Expr::Script`, `Precedence::SCRIPT`),
not the `Subscript` this document used in §13.4 before the protocol framing
existed. The protocol's name is still open — item 11 settled its *shape*, and
the name is in §0.4 with the other eight (§32.3); the error's name is not a
separate decision and should move with it.

`MemberError`'s variants drop the word the parent already carries. Written out,
`MemberError.NoSuchMember` stutters where `MemberError.Missing` doesn't, and
the qualified form is the only form (§17.3) so the parent is always in view.

**`ArgumentError` is its own group, and `WrongArgCount` is retired into it.**
Keyword arguments and defaults (§21.3) turn one arity check into four: a
required parameter left unfilled, more positionals than parameters, a name no
parameter has, and one parameter supplied twice — positionally and then by name
(`f(1, a: 2)` where `a` is first), or by name twice.

`WrongArgCount(want, got)` cannot survive that. `(want, got)` assumes a single
correct count, and with defaults arity is a range, so there is no `want` to
report. A call can also supply exactly the right *number* of arguments and
still be wrong: `f(a: 1)` against `def f(b)` has count 1 for a one-parameter
proc, and is two errors at once.

**Three of the four aren't type errors in any sense**, which is what makes this
a group rather than a subdivision. `Unknown` involves no type — a name doesn't
exist. Neither does `Duplicate`, nor `Missing`. Filing them under `TypeError`
would be misnaming and not merely misgrouping, and Python does it only because
it has no better bucket. Ruby, which does, calls it `ArgumentError` — so §19's
usual rule applies from the other direction this time: take the group where
mesa's own distinction supports it, whichever language happens to supply the
name.

The variants drop the word the parent carries, as `MemberError`'s do. Note
`ArgumentError.Missing` and `MemberError.Missing` now both exist; since the
qualified form is the only form (§17.3), that reads as one word meaning one
thing in two places rather than as a collision.

`IndexError`, `NameError`, and `KeyError` have one member each, so they carry
their payload directly rather than wrapping a sub-case-type. That is
non-uniform — `Error.IndexError` yields its data in one step, `Error.TypeError`
in two — and the alternative is a one-variant case type for each, which is
uniform and pure ceremony. Minimal wins here; Python does the same thing, with
`IndexError` having no subtypes.

*Amended: `IndexError` is now a group* (§23.1), leaving `NameError` and
`KeyError` as the flat two. The rule this paragraph states is unchanged — it
just stopped applying to `IndexError`, which acquired a second *why* when
index validation gave it `NonIntegral` alongside `OutOfRange`. Following
Python was the weaker half of the argument anyway; Python has no reason to
subdivide because it raises `TypeError` for a float subscript, which is
precisely the split §23.1 declines.

**The naming asymmetry resolves itself.** An earlier draft had `TypeError`
carrying two families — `Not*` for a wrong receiver and `*NonNum` for a wrong
operand — and flagged the mixed spelling as something to unify or justify. It
needed neither: the two families were two *groups*, and separating them puts
each naming shape in its own type. `ProtocolError` is uniformly `Not*`;
`TypeError` is what remains. A spelling inconsistency turned out to be a
grouping error wearing a naming costume.

*And then it came back* (§32.2). `NotCallable` moves here and `NotInvokable`
joins it, because §31.4 made callability non-extensible, so `TypeError` carries
`Not*` variants again. The regrouping above was still right — those two are
grouped by *why*, which is what the section argues for. What was wrong is the
extra claim that the naming had become uniform: that was a coincidence of the
membership at the time. Renaming these two to preserve the pattern would be
exactly the costume this paragraph warns about, worn the other way round.

**Protocol variants carry tags, not protocol values.** Protocols being values
is not opposed — it follows from *wherever you have a name, you can hold the
value it points to*, the same principle that already makes procs and types
values. It just isn't needed here: a variant tag is matched on and printed,
which is the error's entire job. If protocols do become values later, these
variants need not change.

*They do* (§31.3): a conformance test needs a protocol to be either an
operator's right operand or an element of a list. The prediction holds — the
variants carry tags and are unaffected.

**What joins later.** `NotOrderable` once Order is a protocol, which also
gives §0.3 item 31 (`"a" < "b"` doesn't work) somewhere principled to land,
and `NotHashable` once §13.4's opt-in `Hash` makes a non-`Hash` key possible.
And if arithmetic ever became overloadable (§0.3 item 13, currently leaning
*structure and access, yes; algebra, no*), `ArithNonNum` would move from
`TypeError` to `ProtocolError` and nothing else would change. Grouping by
*why* rather than by *what caught it* is what makes that a relocation instead
of a redesign.

Catching a group is one arm. Catching a specific member is that arm plus a
match on the wrapped value — a two-level pyramid at worst, entered only when
specificity is wanted.

## 19.4 What this does and doesn't privilege

`Error` is an ordinary case type. Item 6 settles that *any* case type instance
may be raised, so a user error is not a variant of `Error` and does not need to
be:

```
raise ReadError.NotFound(path)
```

Rescue arms name whatever they catch, built-in or not, by the same rule. What
`Error` gets is not privilege but a name that user code can reach for when it
wants to catch the interpreter's own failures as a group.

**And `Error` must be imported to be named** (§20.7). It is not a prelude name
— the prelude holds only the eight core types — so it lives in a built-in
module under ordinary import rules. Three things follow. An uncaught error
still reports correctly without any import, since diagnostics don't need the
type name bound. Catching one requires reaching for the import deliberately,
which is about the right ceremony for handling the interpreter's own failures.
And a module that wants `Error` for itself simply doesn't import the built-in,
which is why this section can say `Error` has "no privilege" and mean it
literally.

**There is no "catch every error" arm short of `else`**, because there is no
common parent over `Error` and a user's own error types — and this is chosen,
not merely tolerated. Catching absolutely everything ought to be a distinct
thing you reach for deliberately, spelled differently from catching a group,
rather than the natural extension of naming one more arm. Adding a universal
parent would also force two levels of nesting on user errors, which is worse
than the thing it fixes.

## 19.5 Two smaller consequences

**Structural equality applies, and is wanted.** Errors are case type instances,
so §13.4's automatic structural `Eq` makes two `Error.KeyError("id")` equal
regardless of where they were raised — the location is not part of the value
(§13.3). Confirmed as the intended behaviour rather than merely a consequence:
an error compares by what went wrong, not by where.

**Reification can be lazy.** `Signal::Error` can keep propagating a Rust-side
representation plus a `Location`, converting to a mesa instance only when a
rescue arm actually matches. Errors that are never caught — the whole fatal
path, which is every error until `rescue` exists — then cost no allocation.
Once caught and bound, the value is an ordinary instance like any other.

---

# 20. Six working-list resolutions, and what they cost

Items 9, 11, 13, 16, 20/21 and 24 settled together. Each is recorded at its
number in §0.3; this section keeps only the parts that reach past the item
they came from.

## 20.1 Protocols mean two things, and the keyword should stop claiming otherwise

Item 9 makes structural `Eq` automatic for every user type, `Hash` opt-in, and
`Eq` overridable. Declaring a protocol therefore no longer means one thing:

- `Hash` **grants a capability** the type didn't have. Before the declaration,
  the type cannot be a dict key; after, it can.
- `Eq` **replaces a default** the type already had. Before the declaration,
  `==` works structurally; after, it works however the type says.

**The keyword becomes `impl`, not `is`.** This is the fix, and it works because
the two spellings make different claims. `is Eq` asserts a *property*, so its
absence reads as "this type is not Eq" — false for the override case, since a
type without the declaration still has equality. `impl Eq` asserts the presence
of an *implementation*, and its absence correctly reads as "no explicit
implementation here," which means *cannot* for a granting protocol and *the
default applies* for an overriding one. One word, and both readings come out
right.

That doesn't erase the grant/override split; it makes the split safe to have.
The distinction is still worth stating, because it predicts what declaring any
future protocol will mean, and it plausibly sorts the list cleanly: `Hash`,
`Iter`, `Call`, `Script` grant; `Eq`, `Show`, `Order` override defaults every
value already has in some form.

§13.4's reasons for declaration-based opt-in are untouched, and so is the
property it protected deliberately — `impl Eq, Hash` is still a list of names,
so opening the set to user-declared protocols (§17.9) remains a matter of
permitting user names in the list.

`impl` is an abbreviation, which mesa avoids everywhere except `def`; the same
question hangs over item 24's visibility keyword, and the two are worth
deciding together since they will sit within a few lines of each other in most
declarations. *That pairing is gone* — §27 removes the visibility keyword along
with the feature. *And the worry is answered* — §31.6's rule is that mesa
abbreviates **frequent** things, so the exception was never `def` specifically;
`impl` and `proto` both qualify.

Two threads item 9 leaves owed. The **cycle answer** — `a.x := a` then `a == b`
recurses forever, §7's hazard, unaddressed since. And **an overridden `Eq`
invalidates the derived `Hash`**, so a type overriding one while declaring the
other has to supply both. That is checkable at declaration, which is where
§13.4 puts every other protocol check.

*Both closed in §24.* The second as stated, in §24.6. The first in §24.7,
coinductively — and it decides the hashing rule as a side effect, since a
bounded hash may not read the sharing that equality cannot see.

## 20.2 `ProtocolError.NotImplemented`

Item 11 collapses subscript to one protocol, with a read-only type raising a
predefined error from the half it declines. **Settled: it is
`ProtocolError.NotImplemented(val, name)`** — the receiver and the member it
won't provide.

It sits in `ProtocolError` rather than getting its own group, which is right on
reflection even though every other group there describes a *caller's* mistake
and this one is the receiver declining. What unifies the group is not whose
fault it is but that something about protocol conformance went wrong:
`NotIterable` is "no implementation at all," `NotImplemented` is "an
implementation that declines this member." Neighbouring cases of one thing.
(*This paragraph originally used `NotCallable` as the example*; §32.2 moved that
variant to `TypeError`, since no user declaration can make a value callable. The
point is unchanged with an example that stayed.)

The general form is deliberately more useful than a subscript-specific error.
It means "this operation exists and is deliberately unimplemented," it is
raisable by any user code, and the declined half of a subscript is only its
first customer — a case-type parent whose verb some variant genuinely cannot
provide is the next.

It also reads well against §20.1's keyword: `impl` declares an implementation,
`NotImplemented` is what you raise when part of that declaration is a promise
you don't intend to keep. The error name and the keyword sharing a root is
worth keeping if the keyword changes.

## 20.3 `<<` spends half a forcing function, and the other half was already gone

Item 16's plan was that `push` and `pop`, written in that order as native
methods, would settle two open questions with evidence rather than
speculation: the field-versus-method boundary, and the native-method error
channel (`fn(&Val, Vec<Val>) -> Val` can only panic).

*Correction.* **The field-versus-method boundary no longer exists**, and hasn't
since §14. §2.2 records it: with zero-arity methods dropping their parens,
"'field' and 'zero-arity method' become one syntactic idea," and `NativeField`
becomes an implementation detail of how native zero-arity members happen to be
stored. The question was deleted rather than answered. Item 16's phrasing dates
from §8, in the first pass, and I repeated it here without checking it against
what §14 had already done — the same staleness class this document exists to
catch.

So only one question was ever still live, and `<<` does sidestep it: an
operator dispatches through `Expr::Binary`, never crossing the native-method
boundary. **`pop` is now the only forcing function, and it forces the one thing
that still needs forcing.**

The error channel has also become more concrete since item 16 was written, and
easier to answer. It is no longer "some way for a native method to fail." With
errors settled as rescuable case-type instances (§13.3, §19), it is specifically:
a native method must be able to construct and raise an `Error` variant, which
means its signature grows a failure path — `Result<Val, Signal>` or equivalent —
rather than being able only to panic. (*Answered in §29 as
`Result<Val, Error>`*, not `Signal`: a native member has no frame to return from
and no loop to break. And `pop` is no longer the only forcing function — §18.2
and §22.4 have since added several.) §5 anticipated the shape ("it needs a way
for a native method to produce the same fatal, well-located error
`Error::ArithNonNum` already produces"); §19 supplies the variants it would
raise.

## 20.4 Visibility is bigger than item 24 and is deferred

*Superseded by §27, which removes visibility rather than sizing it.* This
section is kept because its central observation is what eventually killed the
feature: a private member's existence is readable from the source regardless, so
privacy could never have hidden anything. Read the rest as the case that
produced that conclusion, not as a live design.

Item 24's private-by-default is the answer §10 predicted, and the keyword
spelling is open. But the question has three requirements that together make it
larger than "what's the default at module level," and it is **deferred** rather
than settled:

1. **A constant must be exportable.** This rules out the tidy answer I
   proposed — that only `type` and `def` may be exposed, so a module offers
   nouns and verbs but never state. Tidy, and wrong: a module that cannot
   publish a constant is missing something obvious.
2. **A type must be able to say which of its methods are exposed.**
3. **Possibly which of its fields, too.**

(1) collides with §3.2. There is no declaration syntax, so a module-level value
is `x := 5` — an assignment that happens to be first — with nothing for a
modifier to attach to. Either `pub x := 5` becomes a declaration form after all,
reintroducing for one position exactly the notation §3.2 declined, or visibility
is declared somewhere other than at the binding. An **export list** is the
option that fits mesa's existing shape best: §10's module design is already
"an explicit name-by-name copy" on the import side, so a name-by-name export
list is the symmetric answer and needs no declaration syntax at all. Its cost is
that a declaration no longer shows its own visibility, which is what
modifier-at-the-declaration buys and why Python's `__all__` is generally
considered the worse design.

(2) and (3) are a different question wearing the same word, and the more
consequential one. Member-level visibility means `Val::member` gains a check
relative to the *calling context*, which mesa has no notion of today — `self`
identifies a receiver, not a privilege. The natural rule maps onto machinery
that already exists: code lexically inside `type X`'s body sees X's full member
surface, everything else sees only the exposed part. That stays statically
decidable, so §3.4 survives.

**Settled ahead of the rest: accessing a private member fails with its own
error**, distinct from `MemberError.Missing`, name to be chosen. So the member's
existence is admitted rather than hidden — Ruby's choice, not C++'s.

That is the right way round for mesa specifically, and not merely a debugging
preference. §1 records that "the full method surface of every type is determined
by the source text," with no monkey-patching and no `eval`; §3.4 turns the same
fact into full static resolvability. A private member's existence is therefore
readable from the source by anyone who has it. An error that pretended otherwise
would be concealing something already in plain view, which buys nothing and
costs the diagnostic. It belongs in `MemberError` alongside `ReadOnly`, which is
already a permission-shaped failure rather than an existence-shaped one.

One thing still to work out when the rest is picked up: a private *field* is
still constructor-provided (§2.2), so `Account(balance)` with a private
`balance` means callers supply a value they then cannot read — coherent, but odd
enough to want a deliberate answer.

*Closed by removal* (§27). Not by picking a spelling for any of the above:
mesa has no visibility mechanism, so none of these questions has a subject.
Everything in this section is retained because the reasoning is what produced
the answer. Its own strongest observation — that a private member's existence is
readable from the source anyway, so privacy could hide nothing — is the argument
that eventually removed the feature rather than merely shaping its error
message. The odd case in the paragraph above never has to be answered, and the
private-access error is never named.

## 20.5 The keyword budget is no longer Lua-sized

§9 opened by observing that mesa's sixteen keywords are "Lua-sized and reads
well," and built its argument about namespace pressure on that. Counting what
is now settled or accepted, the list is heading for roughly twenty-five: `not`
(built), `case`, `loop`, `break`, `next`, `raise`, `rescue`, the visibility
keyword, and probably `is`.

*Three amendments since.* `is` became `impl` (§20.1); `next` came back off the
list when §24.5 dropped the continue concept; and the visibility keyword comes
off permanently now that §27 removes the feature it would have marked. So the
count is two lower and the word that was costing most is unspent. None of it
changes the argument below, which is about the aggregate rather than any member
of it — though it is worth noting both refunds came from *deleting a feature*
rather than from finding a cheaper spelling, which is the only move that has
worked.

None of those is wrong individually — each was chosen against a specific
alternative and won. But the aggregate invalidates the premise §9 reasoned
from, and **item 25 is now closed as declined**, so nothing reclaims any of
it. Every keyword is a member name spent permanently.

Why the reclamation wasn't worth taking, since §9 argues for it persuasively:
the one-line fix delivers one of the three things a keyword-named member
actually needs.

| | |
| --- | --- |
| `x.type` after a `.` | would have been fixed |
| `type Event(type)` — declaring the field | not fixed; §9 excludes parameter lists explicitly |
| bare `type` in a method body | not fixed |

The third is the one §9 could not have seen, because a later decision creates
it. Mesa has implicit `self` field access (§3.2), extended to methods by
§14.4, so a member is reachable bare — and a bare keyword-named member lexes
as its keyword whatever `parse_member_expr` accepts. Ruby never hits this:
`@ivar` is required, so there is no bare form to collide with. The gap is
mesa's own. Working around it with `self.type` would also cost §14.8 clause
1's promise that `foo`, `self.foo`, and `x.foo` behave identically — a
keyword-named member would be the one case where the spelling decides whether
the code compiles.

So the cost is real and now permanent. The concrete recurring casualties are
`type` (`event.type`, `token.type`), `end` (`range.end`, and `type R(start,
end)` cannot even be declared), and — *at the time this was written* — `next`
(`node.next`, on any linked structure), which §24.5 has since refunded by
dropping the keyword. The practical consequence for future design is that **adding a
keyword now has an unrecoverable cost**, which is an argument for holding the
list where it is rather than a reason to revisit item 25.

*The casualty table is amended by §53, and unevenly.* Rows 1 and 2 are both
recoverable for the entire keyword set by a parser-side widening at the
positions where no keyword can begin anything — so `range.end` and `type
R(start, end)` are not permanent losses, and §9's reason for excluding
parameter lists ("real ambiguity") does not hold against `parse_params`. Row 3
is structural as this section says, but only for words leading an
expression-position construct; the declaration keywords escape it via §3.1,
`type` among them. What survives intact is the *conclusion about new* keywords,
with its direction reversed: under a contextual regime softening later is
compatible and hardening later is the break, so a new declaration keyword is
recoverable rather than unrecoverable. §53.6 is the argument on the other side,
and it is about forgiving parsing rather than about the budget.

## 20.6 The prelude, and what it finally settles about `$`

Item 4 closes: built-in names live in a prelude tier above the module, and
user code cannot shadow them. The resolution order is complete —
`locals → self's members → module names → prelude`.

**Unshadowable is a prohibition, not a consequence.** Worth being precise,
because §13.6 and §14.5 both reasoned as though picking the tier would settle
it. It doesn't: the prelude sits *above* the module, so an ordinary walk finds
a module-level `type Str` first and the built-in is shadowed exactly as those
sections feared. What prevents it is a separate declaration-time check —
binding a prelude name is an error. That is the resolver's third semantic job,
after §16.2's escape analysis and §17.6's match coverage, and it is the
cheapest of the three.

**It draws the `$` line properly.** §10 and §13.6 both flagged the boundary as
one question and guessed it would come down to shadowability: `$print` can't be
shadowed because it's syntax, `Str` could be because it's a binding. That
distinction is gone — both are unshadowable now. What's left is the better
line, and it is the one the stated values already imply:

| | bindable | holdable as a value |
| --- | --- | --- |
| keywords (`when`, `def`) | no | no |
| `$` builtins (`$print`) | no | no |
| prelude names (`Str`, `Num`) | no | **yes** |
| ordinary names | yes | yes |

`$` marks the names that are not values. The prelude holds names that are.
That is exactly *wherever you have a name, you can hold the value it points
to* (the clarification to the stated values) — with `$` as the visible marker
for the places where the principle deliberately doesn't apply, which is what
§10 was reaching for when it called the sigil "a visible marker for 'this
escapes the language's own semantics.'"

It also delivers what §13.6 wanted from binding the type names: `$type(x) ==
Str` works and cannot be broken by an unrelated module-level declaration, and
`Str()`/`List()`/`Dict()` become constructible, which activates
`NativeType.new` — currently dead code from a program's point of view — and
makes §1's "one construction path" true of the language rather than only of
the interpreter.

## 20.7 The prelude stays small; the rest is importable

The tier raised a second namespace budget. "Built-in type names" is eight if it
means `CORE_TYPES`, but §19 defines seven more that are types (`Error`,
`ProtocolError`, `TypeError`, `MemberError`, `IndexError`, `NameError`,
`KeyError`), and if protocols are values then `Eq`, `Hash`, `Iter`, `Call`,
`Script`, `Show`, `Order` are names too. Twenty-two unshadowable names rather
than eight, with `Error`, `TypeError`, and `KeyError` among the most plausible
things an application would want to name for itself.

**Settled: the prelude holds the core types only. Everything else is
imported.** `Str` and `Num` need no import; `Error` and most protocols do.

The resulting structure is two mechanisms rather than one tier with two
visibility classes:

- **The prelude** — the eight `CORE_TYPES`, always visible, unshadowable,
  never imported. (*Nine since §22.3's `Char`*, which spent the first name
  against this budget and argued the case for doing so; `Module` is a possible
  tenth, left open in §28.1.)
- **Built-in modules** — `Error` and its variants, most protocols, and
  whatever else ships with the language. Ordinary modules, subject to §10's
  ordinary name-by-name import rules, shadowable and aliasable like anything
  else because a name you never imported is just a free name.

That collapses the budget problem rather than managing it. Eight names are
permanently spent; nothing else is. A module wanting its own `Error` simply
doesn't import the built-in one.

**This is §10's own discipline, applied to the mechanism that was about to
break it.** That section warned the failure mode for `$` was "becoming the
standard library by default, because then the sigil stops meaning 'escapes the
language' and starts meaning 'shipped with it.'" The prelude was heading
somewhere identical from a different direction, and the same answer applies:
keep the privileged tier to primitives, let everything else arrive as ordinary
module names.

**Where the line falls, and where it's a judgement.** The clean criterion is
that the prelude holds *the types mesa's own evaluation rules refer to* — every
`CORE_TYPES` entry is what some literal or some evaluator step produces, so
`"x"` has type `Str` whether or not anything was imported. `Error` is
awkward for that criterion, since the interpreter raises errors too. The
tiebreak is namespace cost rather than principle: `Error`, `TypeError`, and
`KeyError` are ordinary English words an application will want, and `Str` and
`Num` are not.

Three consequences worth having written down:

- **An uncaught error still reports fine without the import.** Diagnostics
  don't need the type name bound; only `rescue case Error.TypeError then`
  does. So catching an interpreter error is explicit about the fact that it's
  reaching for a built-in, which is arguably the right amount of ceremony.
- **`impl Eq` requires importing `Eq`.** Omitting the import means `Eq` is an
  unbound name and the type declaration fails — a clear error with an obvious
  fix. Low ceremony, since a protocol is named once per type at most.
- **A user protocol may legitimately be called `Eq`.** With the set open
  (§17.9) and the built-in not imported, `impl Eq` refers to whatever `Eq` the
  module has. Same spelling, different meaning, resolved by the import list at
  the top of the file — which is the ordinary cost of any importable name, and
  the reason §10 made imports explicit name-by-name in the first place.

**One temporary exception, and §34.8 widens it.** The built-in protocol names get
a prelude binding until §28's modules exist, because `impl Eq` cannot require an
import from a module system that isn't built. That was already decided as
scaffolding — ROADMAP §3.2a took it and noted the reasoning was missing on this
side, which §34.8 supplies. §34 widens it from names to **definitions**: with the
derived library written in mesa, the protocols and their provided bodies are
evaluated into the prelude tier rather than merely named there. The principle
above is unaffected and so is the expiry — both the names and the definitions move
into built-in modules when §28 lands, and the permanently-spent count stays at
nine. What the widening costs is listed in §34.8: a source identity for stdlib
code, a fixed startup cost, and §20.6's unshadowable check applying to everything
the stdlib declares.

## 20.8 Iteration is pull

Item 10 closes against §13.4's recommendation. Both of that section's arguments
for push had already expired — §16.5 took the first, §17.8 the second — so the
question was open on its merits rather than settled by inheritance.

**The shape.** Two protocols, not one. A collection yields an iterator; an
iterator yields a step. That is Rust's `IntoIterator`/`Iterator` and Python's
`__iter__`/`__next__`, and the split is load-bearing rather than ceremonial: the
collection and the position-within-it are different objects with different
lifetimes, and conflating them means a collection can only be iterated once.
`each n in xs do … end` survives as surface syntax and desugars onto it.

**It costs what it looks like it costs.** More mechanism than push's single
`each(body)`: two protocols instead of one, a sentinel type, and a state object
allocated per loop where push allocated nothing. The last is the one to watch,
since it lands on the most common construct in the language. Native collections
can be special-cased cheaply — `Expr::Each` already matches `Obj::List`
directly — but doing so re-creates the two-path split §13.4 hoped protocols
would collapse ("that collapses the currently-hardcoded matches in
`Expr::Script` and `Expr::Each` into a single dispatch path"). Uniform
dispatch or cheap native loops; probably not both.

*Narrowed by §34.9.* With the derived library written in mesa, the seam falls
between the **base verbs** and the **library**: native types implement the step
and collection verbs natively under §29, and `map`, `filter`, and `fold` are mesa
source calling them through the protocol. So the two-path split applies to a
handful of verbs rather than to the whole collection surface. What stays open is
only whether `each` over an `Obj::List` pays an interpreted step per element or
takes a native fast path — one construct, not the shape of the library.

**Both of push's costs simply don't arise, which is the retroactive
justification.** §13.4 priced two, and pull pays neither:

- **`break`.** Under push, `break` unwinds through a script-level method call,
  so a user's `each` "must tolerate being abandoned partway through." Under
  pull, breaking means not calling the iterator again. No unwinding through
  user code at all. (What survives is narrower: an iterator holding a resource
  still gets no cleanup hook, which is a general no-destructors gap rather than
  an iteration one.)
- **The `RefCell` reentrancy hazard.** Under push, an `each` implementation
  holding a borrow across the callback while the body mutates the receiver is
  the canonical double-borrow. Under pull the borrow is taken and released
  inside the step call, and the body runs with nothing held. The hazard §6.2
  wanted to turn into "a loud runtime error" mostly stops existing.

**The sentinel is probably not new machinery.** §13.4's objection to pull was
that exhaustion needs a signal, nil can't be it, and "a distinct sentinel …
is a new kind of value in a language deliberately short on kinds of values."
Case types answered that, but §18.5 has already sketched the specific type for
an unrelated reason:

```
type Maybe
    case Some(val) end
    case None end
end
```

A step is a `Maybe`. `Maybe.Some(nil)` is distinct from `Maybe.None`, so a
collection containing nil iterates correctly — which was the precise failure
§13.4 raised against nil-as-sentinel. One case type serving both optionals and
iteration exhaustion is real economy, and it means the "more mechanics" this
decision costs is one protocol and one state object, not three new things.

*Reversed by §37.* The conclusion above survives; its argument does not. The
economy claim was load-bearing — it is what made a case type in the hot path
look free — and it depends on §18.5's optionals, which §37.3 declines. Without
that join the sentinel has to pay for itself, and a case type costs an
allocation per element per stage where a `Bool` plus a slot on the state object
this section already allocates costs none. So a step is a `Bool`, the element
lives in the slot, and the "not three new things" claim is stronger than stated
here rather than weaker: the sentinel is not a new thing at all.

**The iterator's method cannot be called `next`.** Item 21 made `next` the
loop-continue keyword; item 25 bars keywords as member names, bare or dotted.
So the conventional spelling is unavailable, and something like `advance` is
needed instead.

Worth dwelling on for a moment, because it is the first time §20.5's warning
has produced a concrete casualty rather than a hypothetical one — and the
casualty is not a user's field name but *a method the language itself wants*.
§20.5 listed `next` as a loss for `node.next` on linked structures; that it
also takes the iterator protocol's method name was not foreseen. The two
decisions were made three exchanges apart and neither mentioned the other.
Nothing here is broken, but it is evidence that the keyword budget's cost is
harder to see in advance than §20.5's framing suggested.

*Reversed by §24.5.* Item 21 dropped `next` — the continue concept waits for a
better word — so the keyword is unspent and both losses above are refunded:
`node.next` works, and this method may take the conventional name. The
observation survives its example, with a second half attached. The cost was
hard to see in advance *and* it was recoverable, because nothing had been
written against the keyword yet. That argues for holding uncertain keywords
open rather than for trusting that they can be reclaimed later; item 25 makes
the general case irreversible, and this one escaped only on timing.

**Re-examined while deciding §34, and pull stands — with a third argument this
section could not have made.** The question came back because the absence of
default bodies looked like it might force push: if derived methods have nowhere
to live, an `each(body)` that a combinator can be written over starts looking
necessary. It isn't. `map` over a push-iterable still has to be written
somewhere, so push is not an answer to the defaults question — Ruby's
`Enumerable` is push iteration *plus* a mixin of defaults over `each`, and it is
the mixin half doing the work. The two questions are close to orthogonal, and all
four combinations exist: pull with defaults is Rust, push with defaults is Ruby,
pull with a wrapper is roughly Python, push with a wrapper is unusual but
coherent. §34 answers the defaults question and leaves this one where it was.

Two costs push would now carry that were not visible when §13.4 priced it, both
of them arriving from decisions made after it:

- **Early exit collides with the open `break`-across-a-proc-boundary question.**
  This section's cleanest argument for pull is that breaking means not calling the
  iterator again. Under push, `take(n)` and a `find` must stop the source's `each`
  from inside a callback, which is exactly the mechanism §29.2 records as intended
  not to work and §0.4 carries as undecided. Push would not merely reopen that
  thread; it would make answering it a prerequisite for writing `find`. The
  alternative is a callback return-value convention, which is an implicit rule of
  its own.
- **§30's by-value capture partly disables accumulation.** Push combinators lean
  on a callback writing to enclosing state, and §30.4 breaks rebinding an
  enclosing local through a closure. It is survivable — §30.2's "bindings
  snapshot; objects don't" means `result := []` followed by `<<` inside the
  callback works, since the `Rc` is shared — but a rebound counter does not, and
  needs a body field or a one-element list. Pull combinators use an ordinary loop
  and rebind freely.

Neither was available to §13.4, and the second was not available to this section
either, since §30 postdates it. The conclusion is unchanged and now rests on four
arguments rather than two.

---

# 21. Three additions

Taken together because they land in the same place: a type's member namespace.
Static methods, local types, and keyword arguments with defaults.

## 21.1 Static methods, spelled `def self.name`

```
type Amount(cents)
    def self.of_dollars(dollars)
        Amount(dollars * 100)
    end
end
```

**The access machinery already exists**, because §17.3 gave types members so
that qualified variant names (`Expr.Ident`) would work. `Amount.of_dollars` is
that same member access. §14.7's "types are nouns" makes it coherent rather
than a carve-out — a noun having verbs is what mesa is organised around.

**Why `self` and not a keyword.** The alternatives are `static def name`, a
`class << self`-style section, or `def Amount.name`. The first two spend a
keyword permanently, and §20.5 established that keywords are now an
unrecoverable cost since item 25 declined ever reclaiming them as member names.
`def Amount.name` spends nothing either, but couples every static to the type's
spelling, so renaming the type edits all of them. Eigenclasses were considered
and rejected on the obvious grounds. So this is decided on the budget rather
than on taste.

**It resolves item 28 rather than aggravating it.** That item records `self`
holding three inconsistent statuses — interner keyword, syntactic identifier,
conditional runtime keyword — and §9 judged that making it a real `TokenTag`
"would cost almost nothing." Static methods make it near-necessary, since the
parser must recognise `self` immediately after `def`. The side effect §9 wanted
arrives with it: `self := 9` becomes a syntax error rather than declaring an
ordinary local.

**What it closes.** Mesa has exactly one constructor per type, since fields
*are* the constructor signature (§2.2), so alternative construction has meant a
module-level proc — which "pollutes the module namespace with a name that's
only meaningful in one place," §3.1's phrasing about nested `def`, applying
here for the same reason. It adds no second construction path (§1); a static
constructor calls `Amount(…)` like anything else.

**Constants come along free.** Under §14.8 clause 1 a zero-required-arity
invocable invokes on reference, so `Amount.zero` reads exactly as a constant
with no separate mechanism. That covers type-associated constants, which is
where most constants live, and reduces — without removing — §20.4's
requirement that a module be able to export one.

**Two smaller rules.** A static is not among an instance's members, so bare
`of_dollars(…)` inside an instance method does not resolve under §14.5's order;
qualification is required, as in Ruby, and the order stays at four tiers. And
`self` inside a static body is the type.

## 21.2 Local types

```
type LinkedList
    type Iter
    end
end

iter := LinkedList.Iter()
```

**This fills a hole rather than opening one.** The parser already accepts
`type` inside `type`; the runtime panics at `rt.rs:569`'s `todo!()` — verified.
§3.1 flagged that `todo!()` as one of "the two places where the parser accepts
something the runtime can't express," and proposed making it a syntax error
*if the answer is "never."* This is the other answer.

**Pull iteration created the demand, three exchanges after being settled.**
§20.8 requires "a separate struct for iterability," so every iterable type
needs an iterator meaningful only to its collection. Without nesting each one
spends a module-level name — `LinkedListIter`, `TreeIter`, `RangeIter`. This is
the same instinct as §20.7 keeping the prelude to eight names, applied one
level down.

**The cost: `A.B` stops meaning one thing.** `Expr.Ident` says Ident *is a case
of* Expr; `LinkedList.Iter` says Iter is merely *scoped inside* LinkedList.
Same spelling, two relations, and a reader cannot tell which without finding
the declaration. Java carries the same ambiguity (`Outer.Inner` against
`Enum.CONSTANT`). The compiler always knows the difference — §17.6's coverage
checking applies to variants and not to nested types — so this is reader-facing
only, and accepted.

Ordering needs no new rule: variants come first per §17.1, and since every
`case` clause now takes `end`, both `def` and `type` are unambiguous member
starts afterward.

*One rule arrives later.* §26 adds a third member start, `Ident :=`, which is
equally unambiguous but unlike `def` and `type` is order-sensitive, since a body
field may reference those to its left. Body fields therefore sit between the
variants and the methods, and that is the only ordering constraint a type body
has.

## 21.3 Keyword arguments, with defaults

Not a separate parameter form — any parameter may be passed by its own name,
as in Python. Defaults are in scope, since keyword arguments without them are
only a call-site readability feature.

**Mesa dodges Ruby's disaster for free.** Ruby's implicit hash from trailing
key-value pairs made "keyword arguments" and "a hash literal" ambiguous for a
decade, resolved only by Ruby 3 separating them by force. Mesa cannot hit it,
because dict literals require braces: `f(a: 1)` is unambiguously a keyword
argument and `f({a: 1})` is a dict. The parse is one token of lookahead — a
bare `Ident` followed by `:` in argument position.

**Defaults evaluate per call, not once at declaration.** This is the single
most important rule here, and it is where Python should *not* be followed.
`def f(x: [])` in Python evaluates the default once at definition, so every
call shares one list; it is among the best-known traps in the language.
Evaluating per call removes it entirely. It also means a default can reference
an earlier parameter (`def f(a, b: a + 1)`), which Python cannot express —
worth deciding whether that is a feature or an order-dependence to forbid.

*Settled as a feature* (§24.4). Call-site arguments evaluate left to right in
written order; defaults then fill in parameter declaration order, so a default
sees every parameter to its left however that one was supplied. A reference
rightward is a declaration-time error.

**It amends §14.8 clause 1.** With defaults, arity is a range rather than a
number, so "zero arity invokes" becomes **zero *required* arity invokes**.
`def f(a: 1)` invokes on reference with `a` defaulted. `&f` still yields it.

**Fields should take defaults too**, since fields *are* the constructor
signature (§2.2) — `type Account(balance: 0)` — and the symmetry is hard to
argue against once `def` parameters have them.

**Two costs, one of which is a leak worth catching now.** Parameter names
become public API, so renaming one breaks callers; for types this reaches
further than usual, because renaming a *field* is then a caller-visible change.
And more sharply: item 24's deferred work includes private fields, but a
private field's name would still be exposed at the constructor as a keyword
argument — `Account(balance: 5)` where `balance` cannot be read. The two
features are being designed in the wrong order to notice that later.

*The second cost was voided rather than paid* (§27): there are no private
fields, so nothing leaks through the constructor. The first stands and is
unaffected — parameter and field names are public API, and with no visibility
anywhere that is now the general rule rather than a local consequence.

**Where it helps.** §17.9 has case-type variants repeating shared fields
(`case Ident(loc, name)`, `case Lit(loc, lit)`), and positional construction
gets opaque as those lists grow; the keyword form makes the repetition
readable. And §18.2's obligatory dict `get(key, default)` becomes one method
with a defaulted parameter rather than two overloads — `get(key, default: nil)`
gives back the old miss-returns-nil behaviour as an opt-in.

**Two things to settle.** Defaults on *native* methods need `NativeMethod`'s
fixed `arity: usize` to become a range, which touches item 32's territory.
(*It touches it more directly than expected* — §29.1 folds that struct into
`NativeMember`, so the arity change and the error channel are edits to the same
declaration and want one pass over `CORE_TYPES` rather than two.) And
§19 gains a whole group. The four checks a call now needs — `Missing`,
`TooMany`, `Unknown`, `Duplicate` — separate out as `ArgumentError`, retiring
`WrongArgCount`, whose `(want, got)` payload cannot survive arity becoming a
range.

# 22. Strings

§8 called this "the biggest practical hole" and §0.3 called it the largest
functional gap, both describing item 14 as though it were one question. It is
five, and they settle in an order where each narrows the next: mutability
decides what may be cached, caching decides what `size` costs, `size` decides
what a character is, and what a character is decides how access is spelled.
Four settle here. Interpolation is deferred deliberately, and §22.6 argues
that deferring it *now* costs less than deferring it would have before the
other four landed.

## 22.1 Strings are immutable, and the hash decides it

`Obj::Str` already compares by content (`rt.rs:162`) and hashes by content
(`rt.rs:179`), so a string is already a value-like key — and the most common
kind of key any program has. Item 9 made `Hash` opt-in for user types
precisely so the mutable-key hazard would be visible at the declaration. `Str`
cannot opt out, because its hash is built in. A mutable `Str` would therefore
put that hazard in the one place the language has no way to mark it.

Nothing is given up, because nothing mutates a string today: `Str` carries one
field and `CORE_TYPES` registers no method on it.

Two things are bought, both cashed below rather than here: a scalar count that
can be cached with no invalidation path (§22.2), and views that cannot be
invalidated if they are ever wanted (§22.6). Every "can this be cached"
question in this section is answered by this one.

## 22.2 `size` counts characters, closing item 15

Three candidates, each defensible in isolation: bytes (Go), Unicode scalar
values (Python, Ruby 1.9+), grapheme clusters (Swift). Graphemes need a
Unicode table and a dependency, and give version-dependent answers to
`"x".size` — out of scale for mesa. Bytes are what mesa does today, by
accident of `Box<str>` rather than by choice.

Scalars, then. §8's complaint was that `size` "means 'number of elements' on
two types and 'number of bytes' on a third," and this closes it by making
`Str` agree with `List` and `Dict` rather than by renaming anything. One name,
one meaning, three types.

The cost, stated rather than discovered later: `size` still isn't "what you
see" for an emoji with a modifier or a letter with a combining accent, where
one grapheme is several scalars. Python and Ruby both live with this. Swift is
the language that doesn't, and pays the table for it.

Immutability makes the count cacheable — computed on first request and stored
on the `Str`, which is already behind a `RefCell`. So `size` is O(n) once and
O(1) thereafter, with no invalidation to get wrong.

*One argument retired.* An earlier pass leaned on an ASCII fast path: when the
scalar count equals the byte length, the string is pure ASCII and scalar
offsets *are* byte offsets, which would have made indexing O(1) for most
strings. That was carrying §22.4's indexing question, and §22.4 removed the
question rather than answering it. The fast path is still true and no longer
load-bearing.

## 22.3 `Char`, an immediate

A character is `Val::Char`, holding a validated Unicode scalar — the same
shape as `Num` and `Bool`, not an `Obj`. Surrogate values are unrepresentable
by construction rather than by check.

**The argument is representational, not conceptual.** Python and Ruby both
declined a character type and use one-element strings; mesa could have. What
makes that expensive *here* is mesa's own value representation: a one-scalar
`Str` is an `Rc<RefCell<Obj>>` around a `Box<str>`, two allocations and a
refcount to hold one character, so `"hello".chars` would be about twenty
allocations for five characters. `Val::Char` allocates nothing, and `chars`
becomes a single `Vec<Val>` with no nested allocation. The type doesn't only
add a noun; it removes a pathology.

**It is the ninth prelude name**, and the first thing to test §20.7's budget,
which spent exactly eight and argued the number was permanent. The ninth is
taken knowingly, because the alternative isn't eight names — it's eight names
and a representation that punishes the most common string operation.

**No implicit interoperation with `Str`.** `c == "a"` is `false`, `"x" + c` is
a `TypeError`, and conversion is explicit. Rust takes this line; Java's `char`
promoting into arithmetic is the counterexample. It also keeps §22.5's `+`
from needing a third case.

**Literals are single-quoted:** `'a'`. Ruby, Python, and JavaScript all accept
`'hello'` as a string, so a widespread habit will produce an error in mesa where
those three produce a value — and that is the point rather than the price. Two
spellings for one string type is a wart mesa declines to inherit, so the
character is better spent on something the language actually has. `?a` and
having no literal at all were both considered; with `Str` unindexable (§22.4),
the only other way to name a character is `"a".chars[0]`, bad enough that a
literal stops being optional.
The mitigation belongs in the error message: a multi-scalar `'…'` should say
that mesa spells strings with double quotes, not merely that the literal is
malformed. Lexing mirrors `lex_str`, with `\'` joining the escape set.

`Char` is truthy always, which needs no new rule — §18.4's is that everything
but `nil` and `false` is truthy. Ordering arrives with item 31's `Order`
protocol rather than as a special case, so `c1 < c2` and `"a" < "b"` land
together.

## 22.4 `Str` is not indexable

`s[i]` is not written. Characters come from `s.chars`, which returns a `List`
of `Char`, and that list is indexed like any other.

**Where this sits among the languages.** Ruby and Elixir index characters
directly over byte storage and pay O(n) — Ruby with a 7-bit-ASCII fast path.
Python affords O(1) by not storing UTF-8 at all: since PEP 393 it picks
Latin-1, UCS-2, or UCS-4 per string, which is three internal representations
plus a scan at construction. JavaScript, Java, and C# get O(1) by indexing
UTF-16 code units, which hands back half a surrogate pair for an astral
character. Go, Rust, and Swift each refuse in their own way: `s[i]` is a byte
in Go, doesn't compile in Rust, and needs a `String.Index` in Swift, whose
stated reason is that an API shouldn't make an O(n) operation look O(1).

Julia takes a fourth position worth recording separately, because it is the one
mesa would reconsider if §22.6's `Range` indexing ever wants integers too:
indices are **byte offsets**, so lookup is O(1), but valid indices are sparse
and a non-boundary offset raises rather than returning a fragment. It buys
constant time without buying UTF-16's silent wrongness, at the cost of an index
space with holes in it.

Mesa joins the last group and gets there more cheaply than Swift does, because
`List` already does the job `String.Index` was invented for. No view type, no
index type, no new class of thing.

**`chars` is O(n) per call and is not cached.** Three reasons, the first
decisive on its own. A `List` is mutable and has identity, so a cached list
would be shared:

```
cs := s.chars
cs << 'x'
s.chars        # would now have the extra element
```

That is a correctness bug, not a surprise. Avoiding it means copying the cache
per call — which saves the decode and not the allocation — or introducing
frozen lists, a second kind of list in a language that has one. Second, a
per-string decoded cache is roughly sixteen bytes per character held for the
string's whole lifetime, filled by a single past call and never released: a
permanent cost arriving just as the frame-arena work has to reason about
lifetimes. Third, and most to the point, the only code a cache helps is code
that calls `s.chars` repeatedly, which is exactly the pattern this design
exists to make visible.

**Iteration never materializes anything.** §20.8's pull protocol on `Str`
yields `Char`s one at a time, so `each c in s` builds no list and costs O(1)
per step regardless of encoding. That is the common case. `chars` is for
random access, which genuinely needs the list, and where hoisting it into a
local is the entire point.

**Substrings need a method**, since nothing else can produce one now:
`s.slice(from, to)`. Recorded as a stopgap by intent — §22.6's `Range` is the
better answer, and this is what stands in until it exists.

**Two costs, both accepted.** `list[0]` works and `s[0]` doesn't, an asymmetry
a reader learns rather than derives. And item 11's one-protocol subscript
loses the first built-in client an earlier draft credited it with, since a
non-subscriptable `Str` implements neither half. Both are temporary in the
same way: range indexing returns `Str` to the subscript protocol, read-only,
which is precisely item 11's read-only-half-raises shape.

## 22.5 Concatenation is `+`

Item 13 closed arithmetic as non-overloadable and warned that string
concatenation would therefore have to be "hardcoded native behaviour or a
different operator." It is the first.

`+` on two `Str`s concatenates. Nothing else does: `Str + Num`, `Str + Char`,
and `Num + Str` are all errors, since §22.3 bars implicit interoperation and
mesa has never coerced.

**Why not `..`.** Lua separates the operators because it *does* coerce between
numbers and strings, so `1 .. 2` and `1 + 2` must differ. Mesa doesn't, so the
ambiguity `..` exists to prevent cannot arise, and the operator would spend a
token to distinguish cases the type check already separates.

**Why not `<<`.** It would mutate in place on `List` and allocate a new value
on `Str` — one spelling, two relations. §21.2 accepted that shape once, for
`A.B`, and reluctantly. There's no reason to accept it twice when a
well-understood alternative exists.

**`+` now names two operations**, which is the honest cost. It is bounded by
the operand types being disjoint — no value is both a `Num` and a `Str` — so
the reading is fixed by the operands rather than by context, and every
language in mesa's ancestry except Lua spells it this way.

**One §19 consequence.** `TypeError.ArithNonNum(val)` currently covers every
bad `+`. With `Str + Str` legal, `+` dispatches on the left operand and the
failures split: a non-`Num` right operand under a `Num` left stays
`ArithNonNum`, and a non-`Str` right operand under a `Str` left wants
`TypeError.ConcatNonStr(val)`. §19's rule applies as written — group by *why*
it failed, not by which operator caught it.

## 22.6 Three deferrals, and why they are cheap now

**Interpolation stays open**, so item 14 narrows to it rather than closing.
What makes it safe to wait is §22.5: with concatenation defined, interpolation
has a desugaring to land on, and nothing above changes when it arrives. Item 12
was expected to travel with it — one printing mode or two — because
interpolating a value needs the display form and not the debug form. *That went
the other way:* §25 settled the split on its own, so interpolation now has both
of its dependencies defined ahead of it and is purely a question of spelling.

**Byte access waits for an opaque `Bytes` type.** The alternative was
`str.bytes` as a `List` of `Num`, at sixteen-plus bytes per input byte.
Deferring it also closes the views question raised earlier in this thread:
`bytes` was the only view that earned its keep on merit, since UTF-8 bytes are
randomly accessible where characters are not, and an opaque type covers that
ground when it is needed. What made views *safe* in the first place is §22.1,
and that doesn't expire.

**`Range` brings indexing back.** `s[0..3]` with an explicit range type is the
intended long-term spelling for substrings, replacing §22.4's `slice`, and it
is where §23.1's reserved negative indices land as well. Deferring it costs
nothing today, and §23.1's reservation is what keeps it a pure extension
later.

# 23. Three closures from the working list

## 23.1 Numbers stay `f64`, and indices are validated (item 17)

**No `Int`, no `Float`, no numeric tower.** §8 recorded f64-only as
"defensible minimalist" and asked that it be a decision on the record rather
than an inheritance; it is now on the record. Lua 5.1 and JavaScript both
shipped this for years. The consequences §8 named are accepted: integer
precision ends at 2^53, and there is no integer division.

**Modulo stays unwritten until something needs it.** This is a decision not to
decide, and it is cheap for a specific reason: `%` costs a `TokenTag`, a lexer
arm, and a `Precedence::of` mapping, and nothing else in the language waits on
it. Nothing is designed around its absence, so adding it later changes no
other decision, and `fmod` semantics on f64 answer the numeric question in
advance.

**Non-integral and negative indices raise.** §8's other complaint — that
`list[1.7]` silently reads `list[1]`, "an implicit coercion sitting inside a
language that discourages implicit behavior" — ends here. A list index must be
a whole non-negative number; anything else raises rather than truncating.

The negative half is load-bearing, and for forward compatibility rather than
hygiene. Python-style negative indexing is wanted eventually. If `-1`
truncated and failed today, programs would come to exist that depend on
`list[-1]` raising, and giving it a meaning later would change what working
code does. Raising now makes that a pure extension. Ranges are reserved on the
same reasoning, and §22.6 is where both arrive.

**`IndexError` becomes a group**, which is what these three failures ask for
once they are laid side by side. `list["a"]` is a wrong *type*; `list[1.7]`
and `list[-1]` are right-typed values that aren't positions. So the line falls
between the groups rather than inside one:

- `TypeError.IndexNonNum(val)` keeps its name and its job — the subscript
  isn't a `Num` at all.
- `IndexError.NonIntegral(index)` — a `Num` that isn't whole.
- `IndexError.OutOfRange(index)` — a whole `Num` that isn't a position.

That is §19's rule reaching a different answer than §19.3 did, for the reason
that section gave: grouping follows *why*, and `IndexError` had exactly one
why when it was written. It has two now, so it nests like the others, leaving
`NameError` and `KeyError` as the flat cases.

**Negatives are `OutOfRange`, not a variant of their own.** The valid domain
is `[0, size)`, so `-1` is literally outside it, and the reservation above
survives the change without ceremony: when negative indexing lands, the domain
becomes `[-size, size)` and `OutOfRange` still means what it says — `list[-5]`
on a three-element list is out of range then too. A dedicated `Negative`
variant would have to be retired at that point, which is a worse shape for
something the design intends to grow into.

**Dict keys are exempt**, because they are keys rather than indexes. `1.7` is
a perfectly good key and so is `-1`; nothing about a hash lookup wants a
position. The validation belongs to the sequence subscript, not to
subscripting in general — which is also why `KeyError` stays flat while
`IndexError` splits. The same validation does apply to §22.4's
`s.slice(from, to)` and to §22.6's eventual `Range`, since those arguments are
positions under another name.

## 23.2 `and` and `or` return their operands (item 23)

`a and b` evaluates to `a` when `a` is falsy and to `b` otherwise; `a or b`
evaluates to `a` when `a` is truthy and to `b` otherwise. Short-circuiting is
unchanged, since it was never in question, and `not` still returns `Bool`.
`rt.rs:945-962` returns `Val::Bool(...)` in every branch today; each branch
returns the selected operand instead.

**This makes `or` the defaulting idiom**, which mesa has not had. §4 argued
for strict boolean on two grounds and §18.6 recorded that the first had
expired — `count or 10` yielding `10` when `count` is `0` depends on `0` being
falsy, which §18.4 ended. The second ground, that a condition's type is always
`Bool`, is what this spends. That is a real loss for local reasoning, taken in
exchange for the idiom.

**It retires the dedicated nil-defaulting operator.** §4 proposed one, and
§18.6 had already downgraded the case for it from hazard-avoidance to
ergonomics. A value-returning `or` covers the same ground with no new token,
failing only when the left operand is genuinely `false` — under §18.4 the only
non-`nil` falsy value in the language. That residue is the honest cost: `x or
default` returns `default` for `x = false`, which is the JS and Python hazard
with the `0`, `""`, and `[]` cases already removed by §18.4. Narrow enough to
accept.

Nothing else changes. Conditions were never required to be `Bool` at runtime —
`when` calls `is_truthy`, which §18.4 makes total — so a condition whose type
is now dynamic needs no new rule.

## 23.3 Zero arity prints without parens (item 29)

`rt_print_proc` always prints parens, so `def render end` would print as
`def render()`. Under §14.8 those are the same declaration, since a `def` may
omit an empty parameter list, so the printer shouldn't reproduce what was
written — it should have one form per proc. Zero arity prints as `def render`;
any other arity prints its list. The alternative, recording whether the
parentheses were typed, would carry a purely cosmetic distinction into the
runtime.

# 24. Seven loose threads, closed

Each of these sat inside an item already marked closed, where §0.3's
strikethroughs hide them. §0.4 now indexes what remains after this section.

## 24.1 Mesa is a blocking language (item 26)

Nothing about concurrency is designed, and this decides to say so rather than
leave the question shadowing other work — which was item 26's actual request,
not a request for a concurrency design.

Everything below the language surface already assumes single-threaded
execution: `Rc<RefCell<…>>` throughout, one `self.scope` chain on the
interpreter, §20.8's per-loop iterator state. None of that is accidental, and
none of it is being *preserved* for a future design either. The point of
deciding is that subsequent work can stop asking whether it needs to.

The cost is bounded and worth naming rather than discovering. Threads later
would mean `Rc` becomes `Arc` and every `RefCell` becomes a lock, which is a
representation change across the whole runtime — expensive, but mechanical,
and it changes no semantics decided in this document. The models that would
*not* be mechanical are the ones that change what a value is, and mesa's stated
values point away from those anyway.

## 24.2 Ordering covers `Num`, `Str`, and `Char` (item 31)

`<`, `<=`, `>`, and `>=` are defined on numbers, strings, and characters.

Strings compare by scalar sequence — code-point lexicographic, not locale
collation. That is the only ordering definable without a Unicode table, and
§22.2 already declined that table when it declined graphemes, so this is the
same decision reaching the same place a second time. `Char` compares by scalar
for the same reason. Mixed operands don't compare: `'a' < "b"` is an error,
per §22.3's no-implicit-interoperation rule.

What closes here is the built-in set. Whether *user* types can join is item 2's
`Order` protocol, and §19.3's `NotOrderable` arrives with it.

*Amended by §43.* That handoff left a clause unstated — whether an implementor's
`compare` owes agreement with `==` — and the answer is **no**: `Order` is total
on the ordering key, which may be coarser than equality. §43.2 finds no
mechanism depending on the stronger contract, and finds that requiring it would
push key-ordered types into overriding `Equal` to misreport their own fields.
The three built-ins here satisfy it regardless, deriving both from one
representation, which is what made the question invisible from this section.

## 24.3 `<<` evaluates to its receiver (item 16)

`l << 1 << 2` appends both, which requires left associativity in
`Precedence::of`. The alternative was nil, under which chaining doesn't fail
usefully — it becomes `nil << 2`, an error one step removed from the mistake.

One thing to state because a reader may guess otherwise: `x := l << 1` binds
the *list*, not the appended element. Returning the receiver is what makes
chaining work and it makes the element unavailable in the same breath.

The protocol should require this of user implementations rather than leaving it
to convention. A protocol that constrains the operator's effect but not its
result gives chaining a per-type answer, which is exactly the kind of thing
§13.4 built protocols to prevent.

## 24.4 Defaults may reference earlier parameters (§21.3)

`def f(a, b: a + 1)` is legal, which Python cannot express. The rule has two
halves:

Call-site arguments evaluate **left to right in written order**. Then defaults
fill in **parameter declaration order**, so a default sees every parameter to
its left — whether that one was supplied positionally, supplied by name, or
itself defaulted. A default referring to a parameter to its *right* is an
error, caught at declaration rather than at the call, since §3.4's resolution
is static.

This promotes an implementation detail to a promise. Mesa already evaluates
arguments in order because that is how the loop in `Expr::Call` is written;
now it is guaranteed, and a later change to that loop would be a language
change rather than a refactor.

## 24.5 `break` takes a value, and `next` is dropped (item 21)

`break x` makes the loop an expression yielding `x`. Bare `break` yields nil,
matching everything else that returns nothing (§18.3). An infinite `loop do …
end` can only end by `break`, so it always yields whatever its `break` gave;
`each` keeps yielding nil unless broken. Ruby allows exactly this shape, and
item 21 predicted the question without answering it.

**`next` is dropped rather than renamed.** The continue concept goes unbuilt
until a word shorter and less common than `continue` presents itself. That is a
deferral of the feature, not of the spelling — nothing is reserved.

**The consequence is that the keyword comes back, and it was the one costing
most.** §20.5 named three recurring casualties — `type`, `end`, `next` — and
§20.8 then found a fourth cost nobody had predicted: the iterator protocol's
step method couldn't be called `next` either, so it needed `advance` or
similar. Both reverse here. `node.next` works on any linked structure, and
§20.8's iterator method can take the conventional name after all.

§20.8's broader remark — that this was "the first time §20.5's warning produced
a concrete casualty" and that keyword costs are "harder to see in advance than
§20.5's framing suggested" — stands, and gains a second half. The cost was hard
to see *and* it was reversible, because the keyword had not yet been spent in
code. That is an argument for holding uncertain keywords open rather than for
relaxing §20.5's caution.

## 24.6 An overridden `Eq` requires a provided `Hash` (item 9)

Structural `Eq` is automatic and `Hash` is opt-in, coupled by the invariant
that equal values hash equally. Overriding `Eq` therefore invalidates the
derived `Hash`, so a type that overrides one and declares the other must supply
both. Checked at declaration, with no runtime cost.

The reverse isn't required: overriding `Eq` without declaring `Hash` is fine.
The type simply isn't usable as a key, which is the state every user type
starts in.

## 24.7 Structural equality is coinductive (item 9's last thread)

`a.x := a` then `a == b` recurses forever under a naive structural comparison.
This closes item 9's last thread, and the shape of the answer decides the
hashing rule with it.

**The algorithm.** Identity first: `Rc::ptr_eq` short-circuits to true, which
covers `a == a` without any further machinery. Otherwise the comparison carries
a set of **pairs** of object identities currently being compared. On entering a
pair, add it; on revisiting a pair already present, **assume equal** and return
true; on unwinding, remove it. Only `Obj` pairs ever enter — `Num`, `Bool`,
`Char`, and `Nil` are immediates and cannot participate in a cycle — so the set
is a short `Vec` of pointer pairs with a linear scan, allocated lazily because
almost every real comparison is shallow. Termination is structural: a heap has
finitely many objects, so at most |objects|² pairs, and the set only grows. The
traversal takes shared borrows only; a `borrow_mut` anywhere inside it would
panic on a value that reaches itself.

**Pairs, not individual objects.** Marking single objects gives false
positives, because marking `a` suppresses every later comparison involving `a`,
including against something entirely different:

```
type Node(x)
a := Node(nil)   a.x := a          # self-loop
b := Node(nil)   b.x := b          # a different self-loop
p := [a, a]
q := [b, 5]
```

`p == q` must be false on the second element. Object-marking gets the first
element right, then compares `a` against `5`, finds `a` marked, and assumes
true. Pair-marking never entered `(a, 5)`, so it compares normally and fails.

**What "assume equal" means.** The inductive reading of structural equality —
equal because built from equal parts — has no base case on a cyclic value and
so defines nothing there. Taking the **greatest** fixed point instead defines
equality as the largest relation where related values have the same type and
pairwise-related fields: *equal unless some finite path of field accesses
distinguishes them*. The seen-set is a candidate relation and the traversal
checks it is closed under field access, which is Park and Milner's
bisimulation, and the same algorithm as checking two deterministic automata for
equivalence by exploring their product. Prolog II's rational-tree unification
does this too, and Ruby reaches the same place operationally through
`rb_exec_recursive_paired` behind `Array#==`.

Two consequences follow rather than being chosen. **Sharing is invisible**:
`a.x := a` equals the two-node cycle `b.x := c, c.x := b`, because both unroll
to the same infinite tree. And **inequality has a finite witness while equality
doesn't** — a difference is always provable by a path, a match is only ever the
absence of a refutation.

**Two alternatives declined.** *Bottoming out on pointer identity* — returning
`ptr_eq` at a revisit rather than true — gives a finer relation that separates
the self-loop from the two-cycle. It was rejected because it leaks identity
into `==`: two separately-constructed but structurally identical cyclic values
would compare unequal, silently, which contradicts item 9's promise exactly
where a user is least likely to expect it. It would also be the only place in
mesa where object identity is observable at all, and it would break equality
across any future deep copy or deserialization. Its one real advantage is that
it relaxes the hashing constraint below. Transitivity under it looks plausible
but is unproven, where coinductive equality is an equivalence relation by
construction — and `Dict` correctness depends on that contract.

*Raising on cycle detection* was also considered, making `==` explicit and
partial. Declined because `==` is automatic on every user type: a partial
operation that is opted into is a corner someone chose, and a partial operation
that is universal is a hazard invisible at the call site.

**What other languages do**, since the choice looks arbitrary without it. Three
families. *Sidestep:* make the cyclable things unhashable or hash them by
identity — Python's `list` and `dict` are unhashable outright and its default
`__hash__` is identity-based, JavaScript and Lua have no structural hashing at
all, and this is mesa today, where `rt.rs:172-184` hashes `Str` structurally and
everything else by pointer. *Overflow and document it:* Java specifies
`List.hashCode` structurally and `ArrayList`'s javadoc warns that a
self-referential list may throw `StackOverflowError` from `hashCode`, `equals`,
and `toString`; Rust's derived `Hash` does the same on an `Rc<RefCell<…>>`
cycle. *Bound or detect:* Ruby detects reentry and substitutes a constant at the
outermost frame (§49.7); OCaml bounds the traversal.

The equality side has a matching datum, verified rather than recalled: CPython
has no general cycle detector for `==`, so two *distinct* self-referential lists
raise `RecursionError`. Python's containers avoid the hashing question entirely
by being unhashable, and pay for it on the comparison side instead.

**Hashing is bounded, not detected.** No seen-set can save hashing — you cannot
*assume* a hash the way you can assume an equality — so the hash traverses a
fixed number of nodes and stops, in the shape of OCaml's `Hashtbl.hash`, which
is `hash_param 10 100` and bounds by node count rather than depth. That bound
was chosen there for cost rather than for cycles, and termination falls out;
the same is true here, and it keeps a hash of a large collection from being
O(n). The collisions are deliberate.

**The bound must depend only on the unrolling, never on identity or reentry.**
This is not an extra rule but a consequence of the paragraph above: equal values
must hash equally, and coinductive equality makes the self-loop equal to the
two-cycle, so a traversal that mixes in "I detected reentry here" reads the one
thing equality cannot see and can hand equal values different hashes. The
practical upshot is convenient — the hash needs a counter, not a seen-set. The
suspicion once recorded here — that Ruby's `Array#hash` may hand different
answers to two arrays `Array#==` calls equal — is false, and §49.7 records what
reading the source found in its place: Ruby obeys this rule, and the reason to
prefer OCaml's bound over its detection is collision quality, not correctness.

**None of this bites today.** `rt.rs:156-184` makes only `Str` structural;
every other `Obj` compares and hashes by pointer, so no cycle is reachable. The
whole mechanism arrives with item 9's structural `Eq`, which is where it should
be built.

*Two amendments from §49, which had to decide the container half item 9 never
scoped.* The **equality** mechanism arrives earlier than this paragraph expects:
structural `List` equality makes `a := []; a << a` a cycle with no user type in
it, and that change touches no protocol and no declaration-time check, so the
pair-set is buildable and testable well before item 9. And the **hashing** rule
above is an instance of a wider one (§49.1): the bound may read only what
equality can see. Identity is one thing outside that; insertion order under an
order-insensitive dict equality is another, and the paragraph as written does
not cover it. What keeps the wider rule from costing anything is that containers
are unhashable, so no unordered collection ever enters a hash traversal — which
is also why the O(n) claim above survives, being true of ordered structure and
false of unordered structure (§49.2).

# 25. Display and Debug (item 12)

Two printing modes, affirmatively, rather than the split existing implicitly
because `rt_print_val` and `rt_debug_val` happen to differ.

## 25.1 The split, and no round-trip

**Two protocols.** `Display` is the form written for someone reading output;
`Debug` is the form written for someone reading *values*. Initial naming
candidates: the protocols as `Display` and `Debug`, the verbs as `show` and
`inspect`. Both are placeholders held for a later bikeshed — recorded here so
the design can be discussed with names attached, not because the spelling is
settled. §0.4 carries the thread.

*Settled by §39, and one of the two moved.* The protocols are **`Display`** and
**`Inspect`**, the verbs **`display`** and **`inspect`**. `Debug` fails §39.1's
rule — a value is never the thing being debugged — and `Inspect` is the verb it
was already paired with. `Display` keeps both its name and, now, its verb: the
candidate `show` was dropped so the protocol names its own member rather than
repeating Rust's `Display`/`fmt`, and `Show` was declined for the protocol
because Haskell's `Show` is the round-trippable form this section has just
finished refusing, which would have made the name read backwards.

**No round-trip guarantee, explicitly.** Haskell specifies that *derived* `Show`
and `Read` are mutually inverse, and Python states the aspiration for `repr`
with an escape hatch for when it can't hold. Mesa declines both, because the
promise cannot be kept across the value space: procs, types, and bound methods
have no literal form, and neither does a cyclic value. A guarantee with that
many holes is worse than none, since its value is entirely in being relied on.

Worth naming what this gives up, and what it doesn't. `rt_debug_val` currently
emits `"x"`, `[1, 2]`, and `{"a": 1}`, all of which are valid mesa literals, and
`Obj::Instance` emits `Account(5)`, a valid constructor call. That is allowed to
stay true — declining the guarantee doesn't mean going out of the way to break
it. What it means is that no code and no future feature may depend on it, and
that a debug form is free to change when a better one appears.

This also settles what `Debug` is *for*, which is the question the round-trip
was standing in for. It is for a programmer looking at a value, not for a
machine reconstructing one.

## 25.2 Containers use the debug form for their elements

Already true in the tree: `rt_print_obj`'s `List` and `Dict` arms call
`rt_debug_val` on their contents, so `$print(["a"])` gives `["a"]` while
`$print("a")` gives `a`. Ruby and Python both do exactly this, and the reason
is that without it a list of strings prints ambiguously — `[a, b]` could be two
strings or two identifiers, and `["a, b"]` and `["a", "b"]` become
indistinguishable.

It is also the concrete argument against a single mode, and a better one than
taste: one mode cannot serve both positions, because the same value needs
quoting inside a container and not outside one.

**`Obj::Instance` is inconsistent with this** — `rt.rs:1233` calls
`rt_print_val` for field values, so `Account("hi")` prints as `Account(hi)`.
That is a bug rather than a decision, and it gets fixed when printing is
implemented rather than being preserved as precedent.

*One addition since* (§26): an instance's body fields print after its
constructor fields and by name — `Stack(3, items: [1, 2], count: 0)` — which
keeps the positional part resembling the constructor call while showing state
the caller never supplied. They are printed rather than hidden because `==` sees
them, and a `Debug` that didn't would let two unequal values print identically.

## 25.3 Cycles print as `...`

Printing hits §24.7's problem a third time, and takes a third answer.

Equality assumes on revisit; hashing may not detect at all, because equal values
must hash equally and detection reads the sharing that equality cannot see.
Printing **detects and substitutes**, emitting `...` where a value reaches
itself — Ruby and Python both print `[...]`. Detection is safe here precisely
because printing carries no contract to violate: nothing requires equal values
to print identically, so reading sharing costs nothing.

So the three operations differ, and it is worth stating why rather than letting
it look like inconsistency. Equality can assume because a hypothesis can be
refuted. Hashing cannot assume and cannot detect, so it bounds. Printing can
detect because it answers to no invariant.

## 25.4 `Debug` is automatic; `Display` is opt-in

Proposed rather than settled, on the shape item 9 already established for
`Eq` and `Hash`.

*`Debug` is named `Inspect` and `Eq` is named `Equal` as of §39; the prose below
keeps the placeholders it was written with.*

Every user type gets a structural `Debug` for free — `Account(5)` from the type
name and the fields, exactly as `rt_print_obj` builds it today. `Display` is
declared with `impl` when a type wants one, and `$print` falls back to `Debug`
when it isn't. Python does this (`str` falls back to `repr`); Ruby's default
`to_s` gives `#<Account>`, which is strictly less useful.

The symmetry with item 9 is the argument: the structural form is derivable and
therefore free, the form requiring judgement is opted into. It also means
`$print` always has something to print, so no type is unprintable.

**No second builtin is needed.** `$print` uses `Display`; §22.6's interpolation
uses `Display`; the debug form is reached through the verb, as `$print(x.inspect)`
— which prints a `Str` through `Display` and so prints raw, correctly. Ruby needs
`p` alongside `puts` because it has no such member; mesa's paren-less member
access (§14.8) makes the protocol's own verb the access path, and the `$` tier
stays at one printing name (§20.6).

# 26. Fields declared in the type body

```
type Stack(limit)
    items := []
    count := 0

    def push(x)
        …
    end
end
```

A bare name, `:=`, an expression, at the top of a type body. The field is
per-instance, evaluated at construction, and not part of the constructor
signature.

## 26.1 It introduces no new form

**§3.2 already owns this spelling.** Mesa has no declaration syntax — `x := v`
declares on first assignment — and this is that spelling doing the analogous job
one level in. The alternatives all cost more: `static def`, an `init` block, or
a `field` keyword each spend a word permanently under §20.5's budget, to express
something the language can already say.

**It resolves `rt.rs:579`.** §3.1 named two places where the parser accepts what
the runtime can't express — nested `type` at `rt.rs:569` and a bare expression in
a type body at `rt.rs:579` — and proposed making both syntax errors "if the
answer is *never*." §21.2 gave the first a meaning; this gives the second one.
Restricting the left side to a bare name leaves `type T(a) 1 + 1 end` with no
reading, so that becomes the syntax error §3.1 wanted rather than a panic. The
pair is now fully resolved, by opposite routes.

**Position is the constructor-signature line.** §2.2 already says fields *are*
the constructor signature, so: inside the parens means the caller supplies it,
inside the body means the caller can't. That is the whole rule, and it needs no
modifier keyword. It is *only* about supply — body fields are ordinary public
members once constructed, readable, assignable, and part of the type's
interface. Visibility remains item 24's question, untouched.

**Order.** Constructor fields bind first, then body fields evaluate in
declaration order, so a body field may reference constructor fields and body
fields to its left. A rightward reference is a declaration-time error, which is
§24.4's rule for parameter defaults applied unchanged. Body fields come before
methods and nested types in the body; for a case type, after the variants, which
§17.1 already requires to come first. A case-type parent cannot have them at
all, since §17.9 makes it non-constructible and there is no instance to evaluate
them against.

## 26.2 Three cases, and only two of them justify it

**Internal state the caller doesn't supply** — `items := []`, `count := 0`.
Mesa currently cannot express this at all: every field is constructor-provided,
so a type that wants a working list has to make the caller pass one in. This is
the case that most justifies the feature, and it is the one the framing
"computed fields" obscures, since nothing here is computed from anything.

**Captured at construction** — a generated id, `created_at := $now()`, a
normalized copy of an input. Not recomputable later by definition, so a
zero-arity method cannot express it.

**Derived from other fields** — `area := 3.14159 * radius * radius`. This one
is a cache, and it is the weakest case. Fields are assignable, so `c.radius := 2`
leaves `c.area` stale with nothing at the call site to indicate it, which is the
implicit behaviour mesa usually declines. The alternative already exists and
reads identically under §14: a zero-arity `def area radius * radius * 3.14 end`
is spelled `c.area` at the use site and cannot desynchronize. The form is
available for this and shouldn't be the reason to reach for it.

Note the constructor-parameter version remains available and means something
different: `type Circle(radius, area: 3.14159 * radius * radius)` under §21.3
and §24.4 computes the same value while still letting a caller override it.

## 26.3 What it deliberately isn't

**No `self.zero := Amount(0)`.** A type-level constant is not spelled this way,
and the reason is that `:=` has no free slot. `def self.name` could take on a
new meaning because `def name` and `def self.name` were never synonymous —
§21.1 introduced the second into empty space. But `x := 5` and `self.x := 5`
are *already* synonymous inside a method body under §3.2's assignment order, so
making them differ inside a type body would give one region of the grammar a
distinction that exists nowhere else and can't be recovered by reasoning.

It would also buy nothing observable. §21.1 already gives constants through
zero-required-arity statics, and the only difference a stored constant offers is
one shared instance rather than a fresh one per reference — which §24.7 makes
undetectable, since those compare equal and mesa exposes no identity operator.
It would additionally require evaluating something at type-declaration time,
a phase that doesn't currently exist and that would bring its own ordering
questions. The static form defers all of it to the call site.

The prohibition costs no rule of its own: restricting the left side to a bare
name already excludes it.

**No reassignment of a constructor field.** `type Email(addr) addr := addr.lower
end` is not legal in the first cut. Normalization is a real want and this is a
real loss, but permitting it implies the left side can be an arbitrary place,
which it ought not to be — and the exclusion keeps `:=` in a type body meaning
exactly one thing. Revisitable if it bites; a static constructor covers it
meanwhile.

**It reads like a class constant, and that is the honest cost.** A Python or
Java reader sees `count := 0` in a type body and expects one shared value;
`created_at := $now()` references nothing, so nothing on the line signals
otherwise. What makes this survivable is that mesa's constant slot is already
occupied by statics, so there is no competing meaning *within* the language for
the spelling to collide with. The misread is imported, not internal.

## 26.4 Equality, printing, and the field list

**Body fields participate in structural `Eq`.** They are per-instance state, so
two stacks whose `items` have diverged are not the same value. This has one
sharp consequence for the captured case: `created_at := $now()` makes two
otherwise identical values unequal. Item 9's `Eq` override is the existing
answer and no new mechanism is needed, but the case that most justifies the
feature is also the case that routinely wants an override.

**They print in the debug form, named.** Constructor fields print positionally
and body fields as `name: value` — `Stack(3, items: [1, 2], count: 0)` — so the
output distinguishes what the caller supplied from what the value accumulated,
and the constructor-call resemblance survives for the part where it means
something.

Hiding them was considered and rejected. Since `==` sees them, a `Debug` that
doesn't produces two values that print identically and compare unequal, with
nothing in the output to explain it — the worst case for the reader §25.1 says
`Debug` is written for. Ruby's default `inspect` prints every instance variable
regardless of accessors, and Rust's derived `Debug` prints private fields, for
the same reason: visibility governs what may be depended on, not what may be
looked at.

**The type's own printed form is unaffected.** `rt_print_obj` prints
`type Account(balance)` by walking the constructor fields, and that stays the
constructor signature — body fields are not parameters and don't belong in it.

# 27. There is no visibility (item 24)

Every name a module defines is importable. Every member of every type is
readable. There is no export keyword, no private member, no visibility
modifier, and no naming convention standing in for one.

This resolves §10's fork toward its first option — *everything public, importer
chooses* — but more completely than that option imagined. §10 objected that
under it "the author never explicitly decided anything." Correct, and now
there is nothing to decide: modules are for **naming**, separating names into
contexts so that names may overlap between them, and imports are for bringing
names into scope so a reader can see where each one came from. Neither mechanism
is about access.

## 27.1 Why it holds

**Mesa's privacy could never have hidden existence.** §20.4 had already conceded
this and settled it ahead of the rest — a private-member access would fail with
its own error rather than being disguised as a missing one, because "the full
member surface is readable from the source anyway" (§1, §3.4). So the feature
was always going to be advisory about *dependence* rather than protective of
information. The distance from advisory-with-machinery to advisory-with-nothing
is short.

**Smalltalk is the precedent, and it is the right one to take.** All methods are
public; there are no private methods; categories are advisory. It is the
language mesa cites for minimalism, and it made this call. Lua, the other cited
minimalist, makes modules ordinary tables and leaves privacy to whatever the
author does with locals.

**Privacy's real benefit is evolution across an authorship boundary** — changing
internals without breaking callers you don't control. Mesa is explicitly not a
programming-in-the-large language, and in practice the author controls the
library as well as the program, which is where most of that benefit lives.

**What evaporates rather than getting answered.** §20.4 held item 24 open
because three requirements made it "bigger than this item's framing": a
constant must be exportable, which collided with §3.2's absence of declaration
syntax; a type must be able to say which methods are exposed; and possibly which
fields. All three cease to exist. §21.3's leak — a private field still named at
the constructor as a keyword argument, `Account(balance: 5)` where `balance`
cannot be read — disappears rather than needing the answer it never had.
§20.4's promised private-access error is one fewer variant in §19. And §20.5's
projected keyword list loses the visibility keyword: the second refund after
`next`, and unlike that one it is permanent rather than a deferral, since
nothing is waiting to spend it later.

## 27.2 The cost, which is real and smaller than it looks

**Every helper is a public module name.** Mesa sharpens this rather than
softening it: §3.1 forbids nested `def`, so a proc needing a small helper must
promote it to module level, and with no visibility there is now no lid on that.
§3.1 named the cost itself — promoting a helper "pollutes the module namespace
with a name that's only meaningful in one place."

Three things blunt it, and the third is the one that decides.

§16's proc literals remove some helpers entirely, since a one-use body can be
written where it is used without taking a name at all. §10's imports are
name-by-name copies, so a module's effective interface is what importers name
rather than what it contains — an un-imported helper is unreachable in practice
even though it is reachable in principle. (*That second one expired* — §28 makes
modules values, so `IO.anything` reaches everything and this mitigation is gone.
Two remain.)

And the want itself is rarer than the argument assumes. A helper that recurs
belongs in a library; a helper local to one proc is something the author reports
never having actually wanted. That is worth recording because it is the
empirical counterweight to §3.1's strongest argument *for* nested `def`, which
was exactly this pollution — an argument built on a cost that mostly doesn't
materialize.

**No convention, deliberately.** A leading underscore or similar was considered
and declined. An advisory marker with no mechanism behind it accumulates
privacy's ceremony without its benefit, and it would give readers a distinction
the language does not honour anywhere else.

## 27.3 What this leaves

The import form now carries all the weight, since it is the only mechanism
governing what crosses a module boundary. Its spelling stays open in §0.4,
alongside `impl` — the export half of that thread is gone, so the two that
remain are `impl` and the import keyword.

One thing this deliberately does not decide: whether a module is itself a value.
The stated principle is that wherever you have a name you can hold the value it
points to, and §10's design makes cross-module access a name-by-name copy rather
than an object with members, which points at *no* — a module would be a
namespace, closer to `$`'s side of the boundary than the prelude's. Nothing
forces the question yet, and removing visibility doesn't force it either; it is
noted here because with no access control left, the import mechanism is the
entire module system, and this is the one shape question about it still
unanswered.

*Answered in §28: yes.* Which voids one of §27.2's three mitigations — see
§28.5.

*Partially reversed in §38.5.* There is an export form after all, so "the import
form now carries all the weight" no longer holds. The reversal is narrow and its
siting is the whole of it: the form is a **list** at the head of a declaring
file, never a modifier on a declaration, and it has force at the **package**
boundary alone. Everything §27 rules about type members, about declaration
modifiers, and about visibility *inside* a package stands. What changed is not
this section's reasoning, which was about files and remains right about them, but
that §38 introduces a distribution boundary, where "every module name is
importable" means a library ships its internals as API.

# 28. Modules are values

§27.3 left this as the one shape question the module system still had. A module
is a value: bindable, passable, and accessed through its members.

## 28.1 What that gives, mostly for free

**It follows the positive principle rather than an analogy.** *Wherever you have
a name, you can hold the value it points to* is the rule that already makes
procs and types values, and a module was the conspicuous name that didn't.

**Member access is machinery that exists.** §17.3 gave types members so
qualified variant names would work; a module's members are its top-level
bindings, reached the same way. §14.8 applies unchanged — `IO.read` invokes if
it has zero required arity, `&IO.read` yields it without invoking, and callee
position suppresses invocation as everywhere else. No new rule, and no
special-cased access path (§1).

**Identity equality**, as `Obj::Type` already has. Two references to the same
module are the same value, which §28.4's once-only evaluation is what
guarantees.

`Module` is a type, and it is nameable either way. What is deliberately left
open (§0.4) is whether it sits in the **prelude** — always visible, never
imported, unshadowable, and a name permanently spent — or is an ordinary
importable name in a built-in module, which is precisely the split §20.7 drew
when it kept the prelude to the core types and sent `Error`, its variants, and
most protocols to built-in modules instead.

The considerations are §20.7's, unchanged. The prelude is at nine after §22.3
spent the ninth on `Char`, and its argument for staying small was that a
privileged name is unrecoverable while an importable one costs nothing —
shadowable, aliasable, and free to a program that never names it. Against that,
`Module` is a core type in the same sense the other nine are. The use is narrow:
a `$type` comparison, which is rarer than any use of `Str` or `List`.

## 28.2 Members are read-only from outside

`IO.x := 5` raises `MemberError.ReadOnly`.

Python permits cross-module assignment; mesa refuses it, because it is the
sharpest local-reasoning hazard the module system could acquire — a module's
own top-level state changing from somewhere else in the program, with nothing
in the module's source indicating it. Refusing costs nothing, since a module
that wants to offer mutation exposes a proc that mutates its own state, which
puts the operation in the module's own text where a reader will find it.

The variant already exists and needs no addition. §20.4 described
`MemberError.ReadOnly` as "permission-shaped rather than existence-shaped,"
which is exactly right here: the member is admitted to exist and the assignment
is refused.

## 28.3 One import syntax, two things to import

```
import IO              # the IO module
import IO.File         # the File type, from the IO module
import Error           # a top-level name, if it isn't in a module
```

One rule: a dotted path, where the last segment becomes the bound name and what
you get is whatever the path names — a module, a type, a proc, a value. Whether
`IO.File` names a type or a submodule doesn't change anything, which is why the
ambiguity isn't one.

**Both forms are necessary, and that is what settles the question §10 left.**
That section designed cross-module access as "an explicit name-by-name copy,"
and modules-as-values could have looked like a replacement for it. It isn't:
qualified access alone cannot bring a module into scope, and name-by-name alone
cannot hand you the module value. They do different jobs, and mesa needs both.

§10's stated reason for name-by-name — provenance, so a reader can see where a
name came from — is also served by the qualified form, arguably better, since
`IO.File` names its source at every use site rather than once at the top of the
file. Neither form is the explicit one; they're explicit about different things.

*The keyword is `import`* (§38.7, closing `L5`), and this stays **one** dotted
path binding its last segment: §38.6 puts renaming in the package manifest
rather than adding an `import ... as` form, so a dependency is renamed once where
the decision to depend was made. Collisions *within* a package's own tree are
still resolved this section's way, by importing the parent and qualifying.

## 28.4 Imports are static, and the graph is acyclic

**Static:** a literal dotted path, at the top level of a file, never inside a
proc, with no computed module name. The rationale is **local reasoning**, not
"metaprogramming is uninteresting" as this section first said — a computed
module name is the exact case where a reader cannot tell where a name came from,
which is the test the third clarification to the stated values makes operative.
It is also what makes the import graph knowable before any evaluation happens,
and it extends §3.4's static resolvability from one file to the whole program,
which is worth having on its own.

**Acyclic:** an import cycle is an error, reported against the whole cycle
rather than surfacing later as a missing member.

The distinction that decides this is **load-time versus call-time reference**.
`x := B.compute()` at A's top level needs B finished; `def f() B.compute() end`
needs B finished only when `f` runs. Under a permissive rule, whether a given
cycle works depends on where in the file the reference sits — which is exactly
the kind of thing mesa's local-reasoning value refuses.

**The precedent is one-sided among systems with declared dependencies.** Go
forbids import cycles outright, OCaml requires a DAG, and Java's JPMS — the
most recent of the three, and the layer of Java that corresponds to this one —
makes mutual `requires` a compile error. The permissive systems both have
documented traps: Python's cycles fail loudly but only sometimes, and Java's
class initialization, which is the layer that actually matches a mesa module's
evaluated top-level state, permits re-entry during initialization and hands back
a static field's *default value*. Mesa couldn't take Java's route even if it
wanted to — it has no defaults, so an unevaluated binding would raise
`MemberError.Missing` far from its cause, which is the worse half of both
behaviours.

**§27 is what makes this cheap.** The usual pressure toward a cycle is that the
shared piece "belongs" to one module and shouldn't be exposed. With no
visibility, factoring it into a third module is always available and costs
nothing, because everything is already public.

**Two consequences, accepted.** The graph is module-granular: `import IO.File`
requires all of `IO`, so a cycle is an error even when the two modules need one
declaration each from the other. And mutually recursive types cannot span a
module boundary — they have to share one, which is defensible since mutually
recursive types are cohesive by definition, and §21.2's local types give them
room to sit together inside it.

**Two smaller rules that come with it.** A module is evaluated **exactly once**
and every importer receives the same value; otherwise top-level state duplicates
per importer and §28.1's identity equality answers wrongly for one module
reached by two paths. A diamond is not a cycle and must work. And the entry file
is a module like any other, so it participates in the graph and can be part of a
cycle.

The rule is stated at the level of modules and doesn't prejudge whether mesa
later groups them into a larger unit — a package, a compilation unit — which is
unaddressed either way.

*Addressed in §38.1: they group into packages.* The rule survives at two grains
— modules form a DAG inside a package, packages form a DAG between themselves —
and "the graph is module-granular" stops being a concession and becomes the
accurate statement of the inner grain. The once-only rule lifts to the package.
One consequence above is refunded: mutually recursive types have to share a
*package* rather than a module, because §33's resolver widens to the package and
makes declarations order-independent within it.

## 28.5 What this costs §27

§27.2 gave three things blunting the cost of having no visibility, and the
second no longer holds. It said a module's effective interface is "what
importers name rather than what it contains — an un-imported helper is
unreachable in practice." With modules as values, `IO.anything` reaches
everything, so that mitigation is gone.

It doesn't reverse §27, which was decided on other grounds, but the cost is now
blunted by two things rather than three. The one that carried the most weight —
that the proc-local helper is a want which rarely arises — is unaffected.

# 29. The native-method error channel (item 32)

`NativeMethod`'s `call: fn(&Val, Vec<Val>) -> Val` has no failure path, so a
native method can only panic. §19 has since supplied the variants it would
raise, which is what makes this answerable now rather than when it was first
noted.

## 29.1 One native member kind

`NativeField { get: fn(&Val) -> Val }` and `NativeMethod { arity, call }`
collapse into a single `NativeMember`. `CORE_TYPES`' two member arrays become
one, and `NativeType` holds one map instead of two.

**The reason is that the language already merged them.** §14.8 made "field" and
"zero-arity method" one syntactic idea, and §2.2 recorded that `NativeField`
had thereby become "an implementation detail of how *native* zero-arity members
happen to be stored." Giving methods an error channel while leaving fields
without one would re-separate them at exactly the moment the language says they
are the same thing — and the divergence would be visible, since a native field
would be the one member kind that cannot fail.

What the fold buys beyond that: one arity story, where §21.3's range covers
zero-arity members for free rather than needing a second representation; one
lookup path in `TypeRegistry`; and one error channel rather than two decisions
about whether each kind gets one. Native members lose the ability to drift from
what §14.8 says they are, which is the property worth protecting.

The cost is an empty `Vec` allocated per zero-arity access, which is avoidable
if it ever matters and is not worth pre-empting now.

## 29.2 The signature is `Result<Val, Error>`

```rust
call: fn(&Val, Vec<Val>) -> Result<Val, Error>
```

**Not `Result<Val, Signal>`.** `Signal` is `Return | Error` today and gains a
`break` carrying a `Val` under §24.5. A native member has no frame to return
from and no loop to break, so two of the three variants would be nonsense
coming from it. Taking `Error` makes them unrepresentable rather than merely
unused.

The case that would demand the looser type is a **callback** — a native member
invoking a script proc whose body then unwinds through it. Half of that is
already closed: §16.4 confines `return` to the literal it appears in, so a
callback's `return` is caught at the literal's own call boundary and never
reaches the native member that called it. The other half, `break` crossing a
callback boundary, is intended not to work, but the mechanism for that is being
worked out separately and is not recorded here. If it turned out to require
dynamic propagation after all, this signature is where that would surface.

*Closed by §41.4, in this signature's favour.* The mechanism is a static lexical
check, so `break` never propagates dynamically and cannot reach a native member
under any future capability. The conditional above is discharged rather than
merely still pending: `Result<Val, Error>` stays correct even once the `fn`
pointer gains interpreter access and callbacks become possible.

**And a callback-capable native member forces a bigger change regardless.** The
current `fn` pointer takes `(&Val, Vec<Val>)` and has no access to the
interpreter, so it cannot evaluate anything at all. Choosing `Error` now
forecloses nothing, because the capability that would want `Signal` changes the
signature on its own account. §8 had already separated these as two capabilities
rather than one, noting that `push` "needs neither an error channel nor a
callback."

## 29.3 `Location`, and the ordering this creates

A native member doesn't know its call site, so it cannot construct a *located*
error. That would be an argument for threading a `Location` parameter through
every native signature — a diagnostic passed everywhere to be used almost
nowhere.

§19 removes the need. Its restructuring moves `Location` out of the error
variants and onto `Signal::Error`, so a native member returns a location-free
`Error` and the interpreter attaches the location it already holds at the call
boundary. That is one line at one site rather than a parameter on every native
member.

**So item 32 depends on that half of §19**, and the two should land in that
order or together. This is the sequencing fact worth carrying forward; the rest
of the change is mechanical.

§19.5's lazy reification lines up exactly: `Signal::Error` propagates a
Rust-side `Error` plus a `Location`, converting to a mesa instance only when a
rescue arm matches. A native member returning `rt::Error` is already in that
representation, so nothing is converted at the boundary and the fatal path —
every error until `rescue` exists — still costs no allocation.

## 29.4 What it unblocks, and one thing left beside it

Every native member that can fail, which after §18.2 is most of the ones mesa
still needs: dict `get` and `has`, list and dict accessors that now raise on a
miss, `pop` on an empty list, and §22.4's `slice` with bounds outside the
string. §20.3 called `pop` "the only forcing function, and it forces the one
thing that still needs forcing" — that is now several, and they were all
waiting on this.

Left beside it deliberately: `NativeType`'s `new: Option<fn() -> Val>` has the
same missing failure path. Nothing currently constructed can fail, so it is not
forced — but §22.3's explicit `Str(c)` conversion and a future `Str` built from
a list of `Char`s both could, and when either lands this wants the same
treatment rather than a second mechanism.

# 30. Proc literals capture by value

§16.2 asked whether a closure may escape its frame and answered no. This
revises that: **a literal snapshots its free variables at creation and may
outlive the frame it was created in.** The escape prohibition is removed.

## 30.1 What forced the revisit

Stored callbacks. §16 justified the restriction against the *immediate*
callback population — map, filter, sort, anything that runs during the call it
was passed to, where "the enclosing frame is still live and no heap cell is
required." That reasoning is correct and covers a real majority of callback
use. It simply never examined the other population: a test framework's
`Test.define`, a routing table, an event handler, a comparator kept in a
struct, a memoized thunk, any factory returning a proc. Those are stored and
run later, and every one of them was blocked.

The gap stayed invisible because a **named** proc escapes freely — it closes
over the root scope, which never pops — and item 27's `&` passes it without
invoking. So `Test.define("adds", &test_adds)` worked all along, and the
restriction only bit once the callback needed to *capture* something. That is
the case where naming the proc doesn't substitute, because a named proc
captures nothing.

**One premise was also wrong.** Escape was believed to threaten mesa's static
name resolution. It doesn't: §3.4 names the two things that would break that —
instance-level method replacement and dynamic member addition — and escape is
neither. A closure that outlives its frame still has statically known free
variables. What escape threatened was the *memory* property in §3.1, that every
frame can be a plain stack frame popped unconditionally. The two got fused
because §16.2 put the escape check in the resolver, so it looked like a
resolution concern; it was always a lifetime concern.

## 30.2 The rule

A literal copies its free variables into the `Proc` at creation. Afterwards it
holds values, not references into a frame, so nothing dangles and it may be
stored, returned, and called at any later time.

**What gets captured:** free variables that resolve to enclosing frame locals,
plus `self`. Module-level and prelude names are **not** captured — they resolve
at call time through scopes that never pop, exactly as they do today. That
keeps capture sets small, and it is what keeps recursion working: a
module-level `f := def(n) … f(n - 1) … end` resolves `f` as a module name
rather than snapshotting it.

**Bindings snapshot; objects don't.** Capturing a `Val` that is an `Obj` copies
the `Rc`, so the closure sees mutations to the instance. What is frozen is the
binding, not the thing bound. That is the same line §16.3 drew when it made
`self` captured lexically, applied to every capture rather than one.

This is Java's model, where captured locals are effectively final and copied,
and PHP's `use ($x)` and C++'s `[=]` without their annotations. As a side
effect the classic loop-variable capture bug cannot occur, since each iteration's
literal holds its own snapshot — §13.5's fresh-scope-per-iteration remains right
for its own reasons but is no longer load-bearing for closures.

## 30.3 §3.1's conclusion survives by a different route

§3.1 concluded the frame-arena refactor needs no upvalue conversion **because
nothing escapes**. Under by-value capture things do escape, but they carry
copies, so the conclusion is unchanged: every frame stays a plain stack frame,
popped unconditionally, and no local is ever boxed into a heap cell.

That is worth stating precisely, because the old *reason* is now false while the
*conclusion* it supported is still true, and a reader checking one against the
other would otherwise conclude the debt had arrived.

The resolver keeps the analysis and loses the enforcement. §0.1 lists escape
analysis among its three semantic jobs; the job is now computing each literal's
capture set rather than rejecting literals that escape. Same information, used
to copy rather than to refuse.

The cost is one copy per captured name at literal creation — an immediate, or a
refcount bump. Creating a literal in a hot loop is proportional to how much it
captures, which is visible in the source.

One hazard inherited rather than introduced: a `Proc` holding captured `Rc`s can
participate in a reference cycle with an object that holds the proc. That is the
same `Rc` cycle mesa already has everywhere, and it belongs to the GC work
rather than to this decision.

## 30.4 What stops working

**Mutating an enclosing local through a closure.** `count := 0` in a proc body
followed by a literal that increments `count` updates the literal's copy, not
the outer binding. Ruby, JavaScript, and Lua all do the opposite, so this will
surprise arrivals from any of them.

Two things narrow it. Inside a method, bare `count := count + 1` resolves under
§3.2 to `self`'s field rather than to a local, and `self` is shared rather than
snapshotted — so the natural spelling does the right thing in the place
callbacks usually live. And §26's body fields give a type per-instance mutable
state that a stored callback can reach, which is where memoization lands rather
than on a captured local.

**A recursive anonymous literal bound to a proc-local name.** The name is
captured at creation, before the literal is bound to it. Module-level literals
recurse fine per §30.2, and a named `def` always does.

## 30.5 No marker, and what it would cost to add one later

The snapshot is observable — assign to the local after creating the literal and
the two disagree — so unlike Java, mesa cannot make the distinction invisible
without enforcing effectively-final, which would be a new rule. PHP and C++ both
make you write the capture out.

Mesa doesn't. "Implicit behavior is discouraged" points the other way, and it
loses to the ceremony an annotation would impose on every literal that touches
anything.

*Correcting an overstatement made while deciding this:* adding a marker later
was described as impossible. It isn't — it's breaking. The migration is
mechanical: implement the annotation, make an unannotated capture a static
error, and fix the sites the compiler names. For a language with a small
audience that is a cost worth deferring rather than a door closing.

# 31. Protocols: the declaration form (item 2, first half)

Item 2 has two halves. One is **which protocols the language defines**, which is
a list. The other is a **declaration form**, which §13.4 never designed because
a closed set didn't need one and §17.9 made necessary by routing case-type
required verbs through ordinary user-declared protocols. This settles the form;
the set gets its own pass (§31.7).

## 31.1 The declaration

```
proto Serialize
    def serialize(writer)
end
```

`proto Name`, a body of bodiless `def` signatures, closed by `end`.

**No default bodies.** ~~The first argument for this was that a structural
`Hash` cannot be expressed as a body in any form, since it is generated per type
from the field list and writing it by hand would require reflection.~~ *That
argument is withdrawn* — the third clarification to the stated values holds that
looping a type's fields is **introspection**, which mesa supports, so a user
could hand-write a structural `Hash` or `Eq`. Two consequences: the case for
signature-only protocols rests on the two arguments below rather than three, and
the built-in/user asymmetry is a **choice** rather than the inherent fact this
paragraph claimed. Derivation stays a separate mechanism from defaults because
it is automatic, not because it is inexpressible.

**The two arguments that do stand.** A protocol carrying
implementations is a mixin with a conformance check attached, and §2 records
mixins as one of the two Ruby pressure-release valves mesa ruled out. And the
case that motivates defaults elsewhere — Rust's traits, where one required
method implies a dozen derived ones — is handled in mesa at the operator layer:
a protocol supplies `compare` and the language wires `<`, `<=`, `>`, `>=` onto
it, so the many-from-few problem never reaches the protocol body.

**The alternative considered and rejected** was allowing bodies, with an empty
body meaning "required" and a filled one meaning "default implementation." That
puts a semantic distinction on emptiness rather than on a keyword, in exactly
one place in the grammar.

**On the bodiless `def` being unique**, the obvious comparison cuts the other
way. §17.1 required every `case` clause to close with `end` for decidability —
but a case clause *can contain members*, so a delimiter is needed to know where
it stops. A signature can contain nothing at all. The rule states uniformly as:
`def` takes a body wherever a body is possible, and inside a proto it isn't. No
lookahead is required, because the enclosing context settles it.

---

*Reversed by §34, and this is the second-largest reversal in the document after
§30's.* **A protocol member may carry a body.** An empty body means the member is
required; a non-empty one means it is provided, and an implementing type that
stays silent acquires it.

What this section got right is the mixin *shape* of the hazard. What it missed is
that its second argument — the many-from-few problem never reaching the protocol
body, because the operator layer absorbs it — holds only for protocols whose
derived surface is spelled with punctuation. Four of §32.1's nine have
named-method derived surface the operator layer cannot take, and iteration's is
an entire library. §34.1 has the full accounting.

Two consequences for the text above. The paragraph beginning "**The two arguments
that do stand**" now has one argument standing, and §34.2 answers it — not by
defeating it but by pricing it: the cost is that a type declaration no longer
lists the type's members, which is the same bill §31.4 pays for operators.
And "**The alternative considered and rejected**" is the form now taken, with
one addition that removes its grammatical objection: *every* protocol member is
closed by `end`, bodiless ones included, so the distinction rests on emptiness at
a position where every member is delimited identically. §34.3 shows why the
undelimited form does not parse.

The paragraph immediately above survives with its conclusion inverted. Its
reasoning — a case clause needs a delimiter because it can contain members, a
signature does not because it can contain nothing — was sound about signatures in
isolation and wrong about the tail position, where a bodiless final member and
the protocol's own `end` are indistinguishable. §17.1's comparison turns out to
cut the way it first appeared to.

## 31.2 Signatures fix parameter names, not just arity

§21.3 makes every parameter passable by name, so parameter names are public API.
A protocol declaring `def push(item)` against a type implementing
`def push(thing)` would leave `x.push(item: 5)` broken on a conforming type. So
a signature fixes names.

That is a stronger obligation than most interface systems impose, and it has a
consequence worth stating: renaming a parameter in a protocol breaks callers of
*every* implementor at once. This follows from §21.3 rather than from anything
about protocols — the coupling exists wherever keyword arguments do; protocols
just make it fan out.

~~Left open, small: whether a signature may carry **defaults**
(`def get(key, default: nil)`), and if so whether implementors inherit them or
must repeat them. Inheriting is a default body by the back door for the one
thing a body can express without reflection; repeating is duplication the
compiler could check. Noted in §0.4.~~

*Closed by §34.7: yes, and implementors **repeat** them.* The objection to
inheriting is void — §34 makes bodies the front door, so nothing arrives by the
back one — but repeating is still right, on this section's own grounds rather than
on that objection's. A signature fixes parameter names because §21.3 makes them
public API; a default is part of a signature under §21.3; so a default is part of
what the implementor matches, and a disagreement is a declaration-time error. The
"duplication the compiler could check" is the option taken, with the check
specified.

## 31.3 Conformance is testable; the mechanism is open

**Settled:** a program can ask whether a value conforms to a protocol. That
answers §2's interface question — "nothing lets code say *I need something with
a `.balance`*" — through declared conformance rather than structural typing,
which is the answer "nouns have verbs" points at.

**Open:** whether that is an **operator** (`x is Serialize`) or a **builtin**
(`$protos(x)` yielding a list of protocols). The `$` form has two arguments in
its favour: §20.6 settled that `$` marks names which are not values, with
`$type` as the precedent, so a builtin costs nothing from the prelude budget,
while an operator spends a keyword or a symbol under §20.5's terms. The operator
form reads better at a use site. Undecided.

**Either way, protocols become values.** §19.3 recorded that "protocols being
values is not opposed — it follows from *wherever you have a name, you can hold
the value it points to* — it just isn't needed here." Something needs it now:
they are either the right operand of the operator or the elements of the list.

**And parent membership is reopened**, deliberately. §17 made "is this some kind
of `Expr`" unaskable by having `$type` return the variant rather than the
parent, and §17.9 said the way to ask such a question is to declare a protocol.
Folding parent membership into this mechanism reverses that. It is recorded here
as a reconsideration rather than allowed to arrive as a side effect of choosing
a mechanism, and it is undecided along with the mechanism.

Type identity needs nothing new either way: `$type(x)` returns the type value
and types compare by identity, so `$type(x) == Account` already works.

## 31.4 Operators stay wired through protocols

Ruby-style operator-named members — `def <<(item)`, `def [](i)` — are declined,
and the reasons are two settled decisions rather than taste.

**§13.4 chose opt-in by declaration rather than by magic name.** A member
literally named `<<` is opt-in by magic name in its purest form: define it and
the operator works, with nothing declared.

**Item 13 settled that arithmetic is not overloadable.** A permissive symbol
grammar would therefore need a blocklist — `def <<` permitted, `def +` rejected
— and a name grammar with four excepted symbols is worse than either uniform
answer.

**The call operator is worse still, because it breaks §14.8.** Under paren-less
invocation `f` and `f()` behave identically for invocables. A callable instance
forces one of two readings: either `x` yields the instance while `x()` invokes
it, making it the only value in the language where those spellings diverge, or
the instance is invocable and bare `x` invokes, so `&x` is required to mention
it at all. §30's by-value literals cover most of what callable objects are
wanted for.

So a protocol declares ordinary named verbs and the language maps operators onto
them. The cost is real: reading `def push(item)` does not tell you it powers
`<<`. What bounds it is that the `impl Append` line sits a few lines above in
the same declaration — the locality §20.1 chose `impl` to provide.

**One asymmetry to state rather than discover.** User protocols are pure member
contracts; built-in ones additionally carry operator wiring, because the
operator set is fixed and users cannot extend it. "The set is open" (§17.9) is
true of protocols-as-contracts and not of protocols-as-operator-hooks.

*Narrowed by §34.* Default bodies are available to user and built-in protocols
alike, so "pure member contracts" is no longer what separates the two. The
asymmetry survives at its real boundary and only there: **operator wiring**. A
user protocol may carry a library; it cannot make `<<` or `[]` call into one.
Nothing else in this section changes — operator-named members and callable
instances stay declined, and §34 asked nothing of either.

## 31.5 `?` and `!` are name characters

Permitted as a **suffix** on members, locals, and parameters. Not on type names,
since a type is a noun and neither suffix reads as one. Never as a prefix or
standalone, so a bare `!` remains an error and `not` stays the only negation
word.

The upside is sharpened by §14.8: `x.empty?` invokes on reference, so a
predicate reads as a question with no parentheses anywhere. That is the Ruby
ergonomic that survives mesa's other departures from Ruby intact.

**The `!=` collision has a clean rule here, unlike in Ruby.** Ruby needs
`valid! == x` with a space, because `valid!==x` is genuinely ambiguous there.
Mesa has **no bare `=` token** — assignment is `:=`, and every token containing
`=` is `:=`, `==`, `!=`, `<=`, `>=`. So exactly one reading ever lexes:

- `a!=b` cannot be `a!` `=` `b`, since bare `=` is not a token. It is `a` `!=` `b`.
- `a!==b` cannot be `a` `!=` `=b`, for the same reason. It is `a!` `==` `b`.

The rule: **consume a trailing `!` into the identifier unless it is followed by
exactly one `=`.** Two characters of lookahead, which the lexer already performs
for `<=`, `>=`, `!=`, and `:=`. Ruby's spacing wart does not arrive.

**`?` is spent permanently**, and that is the cost. §18.6 killed postfix `?` but
recorded it "revisitable only if `?` is given a third job, such as
rescue-to-a-case-type"; a name character and a postfix operator cannot both have
the character. This is the same class of cost as §20.5's keyword budget, spent
knowingly on the grounds that §18's absence-only nil removed the main use.

**One oddity, noted rather than fixed.** `type` is a keyword and `type?` is not,
so `event.type?` is legal where item 25 makes `event.type` a syntax error. The
same holds for `end?`. That is an ugly escape hatch from §20.5's permanent
casualties, and someone will use it.

## 31.6 The abbreviation rule

Recorded because it is general and because it closes a thread rather than only
deciding `proto`: **abbreviate frequent things and core types; let infrequent
things be longer; but nothing so long that it is `continue` or `implements`.**

*A second naming rule joins this one in §39.1* — that a protocol is named for
the operation it supports, as a verb with the type as its object. The two
compose and decide different things: this rule sets a name's *length*, §39.1's
sets its *part of speech*. Together they settle §0.4's `L1`, where this rule
alone had only established that the nine could be spelled out in full.

It settles `impl` and `proto`, which are frequent and sit within a few lines of
each other in every protocol declaration. More usefully, it retroactively
explains `def`, `Str`, and `Num`, and it predicts §24.5's rejection of
`continue` — a rule that accounts for decisions already taken independently is
probably the operative one rather than a rationalisation. §20.1's worry that
"`impl` is an abbreviation, which mesa avoids everywhere except `def`" is
answered: the exception was never `def`, it was frequency.

## 31.7 What remains of item 2

The set. Which protocols the language itself defines, and their names — the
subscript protocol (item 11, whose error name moves with it), the append
protocol behind `<<`, the two iteration protocols and their step method
(§20.8), `Order` (§24.2), `Display` and `Debug` (§25), with `Eq` and `Hash`
already named. That is a list rather than a mechanism, and three of §0.4's
threads resolve with it.

*§39 supplies the list.* Two of the names taken as already-settled here did not
survive it: `Eq` is `Equal` and `Debug` is `Inspect`.

# 32. The protocol set (item 2, second half)

The set is **nine**, and their names are deliberately not settled here (§32.3).
This closes item 2, which is the last item on the working list.

## 32.1 The nine

Each was decided elsewhere for its own reasons; this collects them so the set is
countable rather than inferred from twelve sections.

| What it governs | Shape | Settled in |
| --- | --- | --- |
| Equality | Automatic and structural for every user type, and **overridable** | item 9, §24.6, §24.7 |
| Hashing | **Opt-in**, so the mutable-key hazard is visible at the declaration | item 9, §24.6 |
| Ordering | Powers `<`, `<=`, `>`, `>=`; built in for `Num`, `Str`, `Char`. **Coarser than equality** — compare-equal values need not be `==`, so it orders a projection of the value (§43) | item 31, §24.2, §43 |
| Subscript | One protocol, both halves; a read-only type raises `ProtocolError.NotImplemented` from the half it declines | item 11, §20.2 |
| Append | Powers `<<` and evaluates to the receiver | item 16, §24.3 |
| Collection → iterator | A collection yields an iterator | §20.8 |
| Iterator → step | An iterator yields a step: a `Bool`, with the element in a slot on the per-loop state object (§37.4, reversing §20.8's `Maybe`) | §20.8, §37 |
| Display | **Opt-in**, falling back to Debug when absent | §25.4 |
| Debug | Automatic and structural for every user type | §25.4 |

Two patterns are worth seeing side by side now that they are collected. The
**automatic-and-overridable** pair (equality, debug printing) is derivable from
the field list, so declaring it means replacing a default. The **opt-in** pair
(hashing, display) is not derivable in a way that would be right by default, so
declaring it means granting a capability. That is exactly §20.1's observation
that protocols mean two things, and it is why `impl` reads correctly for both —
now visible as a property of the set rather than an argument about a keyword.

*Named by §39.* In table order: `Equal`, `Hash`, `Order`, `Access`, `Append`,
`Iterate`, `Advance`, `Display`, `Inspect`. The `Debug` placeholder does not
survive — §39.1's rule reads `impl X` as "supports being X'd", and a value is
never the object of *debug*.

Ordering and append each require **one verb**, with the operators wired onto it,
which is §31.4's arrangement and §31.1's reason for needing no default bodies.

*Amended by §34.1.* That reason covers the operators and not the rest. Ordering
also wants `min`, `max`, `clamp`, and `between?`; append wants `extend`;
subscript wants `get` and `has`; and iteration wants a whole library. None of
those is an operator, so the wiring layer cannot absorb them, and §34 gives them
somewhere to live as **provided** members. The one-verb observation stands for
what it describes — the operator fan-out — and is not the whole of what these
protocols derive.

## 32.2 What is not a protocol, and the rule that decides it

**Truthiness, explicitly declined.** §18.4 fixed it — `Bool` answers for itself,
everything else is truthy except `nil` — and a protocol would reopen it. Python's
`__bool__` is the cautionary case: a type that decides its own truthiness makes
`when x then` unreadable without knowing the type, which is the local-reasoning
cost mesa refuses elsewhere. Declining costs nothing, but it belongs on the
record rather than being absent, now that every other operator-shaped behaviour
has a protocol.

**Callability and invocability.** §31.4 declined operator-named members and
callable instances, so `Type | Proc | Method` is fixed and no declaration can
extend it. Two consequences for §19:

- `NotCallable` **moves from `ProtocolError` to `TypeError`**, since no user
  declaration could make `5()` succeed.
- `TypeError.NotInvokable(val)` is **added**, for `&x` on something that isn't a
  `Proc` or bound `Method`. §14.8 clause 2 makes that an error "including a
  type," and §19 never gave it a variant. The two are separate because their
  populations differ: `Marker` passes the call test and fails the `&` test.

**Also not protocols:** arithmetic and unary minus (item 13), string
concatenation (§22.5, hardcoded on `+`), and construction (§1, one path).

**The rule that decides all of these**, and any future case: a failure belongs
in `ProtocolError` only if **some declaration a user could write would make it
succeed**. `NotIterable` and `NotAccessible` pass; `NotCallable` and
`NotInvokable` don't. That is §19's group-by-*why* sharpened into something
checkable, and it is the same test §19.3 applied when it predicted `ArithNonNum`
would move *into* `ProtocolError` if arithmetic ever became overloadable — the
rule running in the other direction.

**One tidy observation it costs.** §19.3 noted that after its regrouping
"`ProtocolError` is uniformly `Not*`; `TypeError` is what remains," and called
the earlier spelling inconsistency "a grouping error wearing a naming costume."
With `NotCallable` and `NotInvokable` in `TypeError`, that uniformity is gone.
The regrouping was still right; the naming pattern was a coincidence of the
membership at the time rather than a rule, and it should not be defended by
renaming these two into something worse.

**Deferred rather than declined: a sequence or collection protocol.** The bundle
of size, subscript, and iterability recurs, and the case that would force it —
views interchangeable with `List` — is itself deferred behind §22.6's opaque
`Bytes` type. Decide it when there is demand rather than in advance.

*Still deferred after §34, for one reason instead of two.* Default bodies remove
the cost objection: `size`, `empty?`, `contains?`, `first`, and `last` are all
derivable, so the protocol can provide them rather than require them, and it stops
being a contract that extracts identical boilerplate from every implementor.
§34.1 records it as the place the no-defaults rule bound hardest. The forcing case
is unchanged and still behind `Bytes`, so the deferral stands on demand alone.

## 32.3 Naming is deferred, and that is cheaper than it sounds

*Closed by §39, which names all nine.* The reasoning below stands as the reason
deferring was safe; it is no longer the state of the question.

None of the nine is named here. The names join §0.4's spelling threads,
including the two that were already there: the iterator's step method, and the
subscript protocol whose error name moves with it.

Two things make deferring cheap. §20.7 put protocols in built-in modules as
ordinary importable names rather than in the prelude, so they are shadowable and
aliasable and a program that never imports one never sees it — the cost of
changing a name later is not the cost of changing a prelude name. And §31.6's
abbreviation rule points the same way for all nine at once: a protocol name
appears about once per type declaration, which is infrequent, so these can be
spelled out in full. `impl` and `proto` abbreviate because they are frequent;
what follows `impl` on the same line does not have to.

## 32.4 What closing this releases

Five rows in §0.2 were waiting on a protocol being *named* rather than on any
open question — pull iteration, `<<`, `Order` for `Str` and `Char`, `Str`
iterability, and `Display`/`Debug`. They now wait on the naming pass alone,
which is a bikeshed rather than a design.

And §0.3 has no live items left, which is the first time since the document was
written.

# 33. The resolver phase

§0.1 records the resolver acquiring **three** semantic jobs, "where its original
brief was name-to-slot assignment alone," and adds that "each was adopted for
its own reasons rather than to justify the pass." That is the right way to
acquire a phase. It is also how one arrives without ever being priced, and six
more jobs have settled since that sentence was written.

Nothing below is a new decision; every job is settled elsewhere. What is new is
the phase they add up to, the property that justifies it being a phase at all,
and a rule for admitting the next job rather than absorbing it.

*Read with §35, which names what this section prices.* This section's "phase" is
one **pass**, and §33.2 then hands three of its nine jobs to the parser, the
loader, and `eval_decl` without saying what those three still belong to. §35
answers that: the phase is **semantic analysis**, the pass below is one carrier
inside it and keeps the name *resolver*, and the property in §33.3 is the
phase's rather than the pass's. Nothing below is wrong under that reading; it is
narrower than it sounds.

## 33.1 Three became nine

| Job | Settled in |
| --- | --- |
| Match coverage, and the dead-`else` error | §17.6 |
| Capture-set computation for proc literals | §16.2, §30.3 |
| The unshadowable-prelude check | §20.6 |
| A parameter default referencing rightward | §24.4 |
| A body field referencing rightward | §26.1 |
| An overridden `Eq` requiring a supplied `Hash` | §24.6 |
| Protocol conformance: members present, parameter *names* matching | §31.2 |
| `break` crossing a proc boundary | §29.2 |
| The import graph: literal paths, acyclicity, evaluation order | §28.4 |

Under all nine sits the original brief: name-to-slot resolution, replacing the
`outer`-chain walk with static slot assignment.

That is not the same thing as the **order** those slots are assigned against,
and the two are easy to fuse. `locals → self's members → module names → prelude`
(§14.5, §20.6) is a rule the *current* runtime can implement by marking the root
scope and having `Scope::local` stop there — §14.5 says so explicitly
("implementable now… no imports, exports, visibility rules, or multi-file
support are needed to draw that line") and it is the order fix that §14.5 calls
"step one of that refactor pulled forward, not a new task competing with it."
The pass assigns slots *according to* the order; it does not establish it, and
§14.8's paren-less calls need the order rather than the pass.

The first three are §0.1's. The rest arrived in §§24–31, each inside a section
about something else, and none was counted against the pass. §26.1 is the
clearest instance: "a rightward reference is a declaration-time error, which is
§24.4's rule for parameter defaults applied unchanged." Correct, and the second
of a kind nobody was counting.

## 33.2 Most of them do not need a pass, and one does

Worth deflating before designing, because the honest number of forcing functions
is one.

*Qualified by §35.2.* One job **cannot** be checked without the pass, which is
what this section establishes and it stands. Two others can be checked without it
but not everywhere they apply: §20.6's prelude prohibition is about a *binding*
and §24.4's rule is about a *parameter list*, and §16.6's proc literals and
§3.2's implicit declaration put instances of both inside bodies, where
`eval_decl` does not go. The deflation below holds for every job attached to a
`type` or a `proto`; for those two it delivers most of the rule rather than all
of it.

**Mesa evaluates declarations.** A `type` declaration, a `proto`, a module-level
binding: the interpreter reaches each of them, in order, before anything that
uses them. So protocol conformance, `Eq`/`Hash` pairing, the prelude
prohibition, and both rightward-reference checks can be done in `eval_decl` as
the declaration is evaluated, with no prior traversal. They are
"declaration-time" in a sense mesa already has.

*Reversed by §41.2.* They *can* be done in `eval_decl`, and they may not live
there: the check's error type names the module it belongs in, and `sem::Error`
raised from `rt` is the boundary violation the phase's separateness rests on.
The observation above survives as the reason none of them is *forced* into the
pass; what it can no longer license is the deferral §33.7 prices below.

**Two of the nine are not this pass's work at all.** The import graph is a
loader — it runs over paths and files rather than over an AST, and §28.4's
staticness is what lets it run before any evaluation. And `break` crossing a
proc boundary can be a lexical loop-depth counter in the parser — "would make
it a parse error and need no `Signal` variant." That phrasing is **§0.4's**, not
§29.2's, which says only that the mechanism "is being worked out separately and
is not recorded here." §0.4 attaches a caveat worth carrying with it: the
argument leaned partly on §16.2's escape prohibition, which §30 removed, so it
is weaker than when it was made and wants re-examining. §33.7 records it as
open, and nothing here closes it.

*Closed by §41.4, and only one of the two survives.* The import graph is still
not this pass's work. `break` is: the check raises `sem::Error`, so §41.2 puts it
in `sem` rather than the parser, and it becomes the pass's first body-walking
obligation rather than a job outside it. The re-examination the caveat asks for
came out the other way — §30 makes the prohibition *necessary* rather than
merely chosen, since an escaping closure may have no loop frame at all.

**Capture sets are achievable dynamically, and expensively.** Evaluating a
literal could walk its body, collect the free names against the live scope
chain, and snapshot. That is correct. It is also per *evaluation* rather than
per literal, so a literal created inside a loop re-derives its capture set every
iteration. Memoizing by `ExprId` fixes it, and is a static analysis with extra
steps.

**Coverage is the one that forces it.** A `when e case … end` inside a proc body
is reached only when that proc runs, so checking it at declaration-evaluation
time means walking proc bodies at declaration time, which is the pass under
another name. Deferring it to first execution destroys what it is for: §17.6
keeps `else` "with a rule that preserves the signal it would otherwise cost," so
that adding a variant later turns a previously-unreachable `else` live instead
of letting it silently absorb the new case, and §18.5 leans on the same check
for opt-in optionals — "static enforcement at the use site, with no type system
involved." A check that fires only on the paths that happened to execute is a
runtime error wearing a compile-time costume.

## 33.3 A property justifies the phase, not any check

The reason to make this a phase rather than scatter the deflated seven into
`eval_decl` is one property:

> Every error determinable from the source alone is reported before the program
> produces its first side effect.

That is what a phase buys and what no individual check buys. It is also why the
deflated seven belong in it anyway: a script that prints forty lines and then
dies because it bound a prelude name is worse than one that refuses to start,
and the difference is not the error but when it arrives.

§5 already sets the posture — "fail-fast with a precise `file:line,col` message"
— and §5 was describing runtime errors, where fail-fast means at the first bad
value. This extends the same posture one phase earlier, to everything the source
settles on its own.

Worth naming what it is not. It is not a claim that a resolved program cannot
fail. Mesa is dynamically typed and most failures remain runtime failures; §33.6
is where that line sits. The property is about the subset the source alone
decides, and what makes it worth stating is that the subset is now nonempty and
still growing.

## 33.4 The cost was mostly committed already

**The traversal is on the plan.** The sym table refactor is a resolution pass by
definition, and §14.5 pulled its first step forward for independent reasons:
paren-less bare calls need the resolution order, so the boundary between locals
and module names gets drawn whether or not anything else rides on it.

**The arena is the right shape.** `Chunk` stores `Decl`, `Expr`, and `Block` in
flat `Vec`s addressed by id, with a parallel `Token` per node stored the same
way. A `Vec<Resolution>` keyed by `ExprId`, and a capture set keyed by a
literal's `ExprId`, are that idiom rather than a new one. The AST design
anticipates side tables without having been aimed at them.

So the marginal cost of the phase is the checks and an error type, not the walk.

## 33.5 Shape

**Two sub-passes.** Collect declarations first — types, their variants,
`proto`s, module-level names — then walk bodies. One traversal will not do,
because a match arm may name a case type declared later in the file and §17.6
needs the variant set before it can read the arms.

**Per module, in import-DAG order.** §28.4 makes imports static and the graph
acyclic, so modules can be ordered before any is resolved, and a module's
surface is known once it is parsed. That extends §3.4's static resolvability
from one file to the whole program, which §28.4 already counts as a benefit
"worth having on its own." Before the module system exists there is one module
and the ordering is trivial.

**Its own error type.** Resolver errors are not rescuable, and structurally
rather than by policy: the program has not started, so there is no frame to
unwind and no `rescue` to reach. §19 makes `rt::Error` script-visible public
surface *because* interpreter errors are catchable (§13.3); these are not, so
they do not belong there. They have `syn::Error`'s character — fatal,
pre-execution, diagnostic only — and should follow the same pattern both
existing error types do: a variant per failure mode, a `loc()` accessor, and a
`Display` impl producing `file:line,col: … error: …`.

**Three outputs.** Resolution facts the interpreter consumes, capture sets
proc-literal construction consumes, and diagnostics that abort before either is
reached.

## 33.6 The admission rule

The set will keep growing, so it wants a test rather than a habit:

> The pass may ask questions about **declarations**, never about **values**.

§17.6 is that rule in miniature, and it arrived there as a constraint rather
than a principle: "The natural assumption is that the checker looks at the
scrutinee's type. It cannot." A resolver looking at `when e` has no idea `e` is
an `Expr`, so coverage had to be read off the arms. Generalised, the same
sentence refuses arity checking on a call, member-existence checking on `x.foo`,
and `Member::set`'s rejection of names that resolve to methods (§2.1). In all
three the thing being asked about is a value, and all three stay at runtime.

It also pre-settles something §31.3 would otherwise raise. Conformance testing,
whether it lands as `x is Serialize` or `$protos(x)`, asks about a value and is
therefore runtime. The declaration half of the same feature — does this type
implement what it claims to — is the resolver's. The two separate cleanly on the
rule, and neither needs the other decided first.

**There is already one instance of the rule being needed.** §30.1 records that
escape analysis was believed to threaten mesa's static name resolution and did
not: "What escape threatened was the *memory* property in §3.1… The two got
fused because §16.2 put the escape check in the resolver, so it looked like a
resolution concern; it was always a lifetime concern." A job was filed into the
pass for the wrong reason, and the misfiling survived fourteen sections. A test
that can be applied to a candidate is what stops that recurring, and it is the
same move §32.2 made for `ProtocolError` — "a failure belongs in
`ProtocolError` only if some declaration a user could write would make it
succeed" — which turned a grouping instinct into something checkable.

The failure mode being guarded against is specific. A resolver that starts
asking about values is a type checker, and mesa deliberately has no types to
check. The phase's existence is not an argument for acquiring some.

## 33.7 What this leaves

~~**When it lands.** Not before §17.6's coverage check needs it, which is with
case types. The deflated seven can sit in `eval_decl` until then and move for a
few lines each. The property in §33.3 is worth having once, not worth paying for
ahead of the check that forces it.~~

*Overtaken by events, and then by §41.3.* It landed with `C2`'s prelude check,
well ahead of coverage, because §41.2's rule leaves the deflated seven nowhere
else to go — a check raising `sem::Error` lives in `sem`. So the forcing function
is not coverage but whichever obligation is implemented first, and the deferral
priced here was never available. What survives is the sizing: the pass that
actually landed is 125 lines and produces nothing but diagnostics, so "worth
having once" was right about the cost even where it was wrong about the timing.

~~**The phase's name, and therefore its error type's.** `syn::Error` and
`rt::Error` are named for their phases, so the third follows mechanically once
the phase has a name. This section says "resolver" only because §0.1 does.~~

*Closed by §35: the phase is **semantic analysis**, its error type is
`sem::Error`, and the pass this section prices keeps the name **resolver** as one
carrier inside it. §35.1 is why those are two names rather than one — most of
what the phase does is not resolution, and two of its carriers are not the pass.*

**Whether desugaring joins it.** §20.8 has `each` desugaring onto the two
iteration protocols, and a pass that already walks every body is where a
desugaring would naturally sit. Nothing requires it there — the interpreter can
desugar as it evaluates — and the thing that would decide it is §20.8's own open
question of uniform dispatch against cheap native loops. Not decided here.

~~**Where `break`'s proc boundary goes.** §0.4 records the mechanism as
undecided, and §29.2's lexical-depth argument as weakened by §30's removal of
the escape prohibition. §33.2 assumes the parser can carry it; if that is wrong,
this phase is the other place it fits.~~

*Closed by §41.4: this phase, and `L10` with it.* The parser was wrong for the
reason §41.2 gives rather than for anything about escape — and the alternative
this sentence offers turns out to be the answer.

---

# 34. Protocols carry default bodies

§31.1 declined default bodies. This reverses that: **a protocol member may carry
a body, and a type implementing the protocol acquires it.** The reversal is
narrower than it sounds — nothing about opt-in-by-declaration, the operator
wiring, or the closed-under-no-self-modification property changes — but it is a
reversal, and §31.1's heading is now wrong rather than merely incomplete.

## 34.1 What forced the revisit

§31.1 rested on two arguments after withdrawing its first. One is that a
protocol carrying implementations is a mixin with a conformance check attached,
and §2 records mixins as one of the two Ruby pressure-release valves mesa ruled
out. The other is that Rust's motivating case — one required method implying a
dozen derived ones — "is handled in mesa at the operator layer: a protocol
supplies `compare` and the language wires `<`, `<=`, `>`, `>=` onto it, so the
many-from-few problem never reaches the protocol body."

**The second argument is a special case wearing the clothes of a general one.**
Every protocol has some ratio of derived surface to required verbs, and the
operator layer can absorb derived surface only when it is spelled as an operator
or a language verb. Named methods have nowhere to go. So two protocols with
identical structure get opposite treatment depending on how their fan-out is
spelled: `compare` → four operators is four-from-one and free, while a step verb
→ `map`, `filter`, `fold`, `take`, `zip` is forty-from-one and costs a wrapper.
The difference is punctuation, not design.

**It is not one protocol's problem.** Four of §32.1's nine have named-method
derived surface with nowhere to live:

| Protocol | Absorbed by the wiring layer | Left over |
| --- | --- | --- |
| Ordering | `<`, `<=`, `>`, `>=` | `min`, `max`, `clamp`, `between?` — whose tie behaviour §43.4 makes observable and leaves open |
| Subscript | `a[k]`, `a[k] := v` | `get(k, default)`, `has(k)` |
| Append | `<<` | `extend` / `concat` |
| Iteration (both) | the `each` desugaring | the entire combinator library |

Equality, hashing, display, and debug are genuinely covered, which is why the
argument looked general when it was made. The leftovers are not hypothetical:
§20.2 and §18.2 already plan `get` and `has` as native members on `Dict` alone,
which answers the question for one type and leaves every user type implementing
subscript to hand-write both against §13.3's rescue. And §32.2's deferred
collection protocol is where the pressure is highest, since `size`, `empty?`,
`contains?`, `first`, and `last` are all derivable and would otherwise be five
members every implementor writes identically — the duplication §31.2 says the
compiler could check, arriving instead as duplication the compiler merely
tolerates.

**The wrapper escape hatch does not generalise.** Iteration can put its
combinators on a wrapper type because iteration is *already* two-step: §20.8
hands you a distinct object whose whole job is being a position, so hanging
methods on it costs nothing conceptually. Nothing else in the set has that.
`Ordered(a).min(b)` is absurd, and so is a wrapper for subscript's `get`. For
every other protocol the fallback is a module of free functions, which means
choosing the wrapper for iteration leaves the general question open and answers
it piecemeal by drift.

**The argument that decided it was about the fallback becoming universal.** A
language with robust object facilities that reaches for free functions to carry
its derived surface spends the value it was built on. That is Python's standard
library as a recurring annoyance rather than a one-off, and it is Elixir's
"everything is a function and data" as a loss of expressiveness when made
universal. *Nouns have verbs* is a stated value; a derived surface that can only
be reached as `Ord.min(a, b)` is that value declining to apply to the parts of
the language mesa writes itself. A pipe operator would make the free-function
shape read well, and it would still be the wrong shape.

Worth recording that §31.1's ground had already shifted once. Its *first*
argument — a structural `Hash` cannot be written as a body — was withdrawn there
when the third clarification to the stated values established that looping a
type's fields is introspection, which mesa supports. That withdrawal also
relabelled the built-in/user asymmetry as "a **choice** rather than the inherent
fact this paragraph claimed." Two of three arguments are now gone and the third
is answered below.

## 34.2 The mixin objection is paid for, not defeated

Five properties separate this from `include`:

- **Conformance is declared, not injected.** `impl Iter` on the type asserts it;
  a protocol cannot reach a type that does not name it. §13.4 chose this from the
  start for other reasons and it does the work here.
- **The member set is fixed at the protocol's declaration site.** No reopening,
  because that is self-modification and §0.1's third clarification rules it out.
  A reader who wants to know what `impl Iter` supplies reads one declaration in
  one place, not an open ancestor chain.
- **Conflicts are a declaration-time error, not a linearisation.** §34.6. Ruby's
  genuine implicitness is that ancestor order silently picks a winner; mesa
  refuses to pick.
- **There is no protocol hierarchy.** §34.5 declines composition, which is where
  ambiguity would breed.
- **A protocol cannot be only bodies.** §34.10 rejects the all-provided
  protocol, which is the shape that would be a mixin outright — methods
  attached with no contract to check. Markers, which have no members at all,
  stay legal, because their content is the conformance rather than the bodies.

**One implicitness is genuinely shared, and is the price.** Reading a type
declaration no longer tells you the type's full member list; you have to follow
the `impl` line. That is exactly the bill §31.4 accepted for operators — "reading
`def push(item)` does not tell you it powers `<<`" — and it is bounded the same
way, by the `impl` line sitting a few lines above in the same declaration, which
is the locality §20.1 chose `impl` to provide. Stated plainly so it is not
rediscovered as an objection: this is a cost, and it was taken knowingly.

## 34.3 The declaration form

A protocol member is **required** or **provided**. An empty body means required;
a non-empty body means provided, and an implementing type that does not supply
the member acquires the body.

```
proto Ord
    def compare(other) end
    def min(other)
        when compare(other) <= 0 then self else other end
    end
end
```

`compare` is required; `min` is provided, and a type declaring `impl Ord` and
supplying `compare` acquires it. The spelling of both names is open (§0.4), and
the example is deliberately not an iteration combinator, since those need §20.8's
step verb and sentinel, both of which are open.

**Every member is closed by `end`, including required ones.** §31.1's rule that
"a `def` takes a body wherever a body is possible, and inside a proto it is not"
goes with the decision it justified, and the naive replacement does not parse.
Consider:

```
proto Ord
    def compare(other)
end
```

Reading one: `compare` is bodiless and the `end` closes the protocol. Reading two:
`compare` has an empty body closed by that `end`, and the protocol is unterminated.
Both are grammatical, and only the token *after* `end` separates them. That is
the decidability problem §17.1 refused for `case` clauses, and it is refused the
same way: a delimiter on every member, bodiless ones included.

**This accepts what §31.1 rejected**, which was putting a semantic distinction
on emptiness rather than on a keyword, "in exactly one place in the grammar."
The objection stands as far as it goes; what changed is the alternatives' price.
A marker on required signatures spends a word for one construct, and a
two-section protocol body spends structure. Against those, a `def` with nothing
in it reading as a signature is the cheap answer, and the distinction is not on emptiness
alone but on emptiness at a place where every member is delimited identically.

**It needs nothing from §15.** Uniform delimiting makes the protocol body LL(1)
without significant newlines, so this adds no dependency there.

## 34.4 What a default body may touch

**A provided body may reference its own parameters, the protocol's own members
(required and provided, through `self`), and module and prelude names. It may
not reference members of the implementing type that the protocol does not
declare.**

This is the rule that keeps a protocol a contract rather than a partial class.
Ruby's mixins are fragile because a module body may reference `@items` — state
the module neither declares nor can check — so conformance is a promise about
methods and a silent assumption about everything else. Under the rule above a
protocol is **self-contained**: it can be checked at its own declaration site,
independent of any implementor, and a conforming type cannot be broken by a
protocol reaching past its own signatures.

The check is one the resolver is permitted to make. §33.6's admission rule allows
questions about declarations and refuses questions about values, and this asks
only what names a body mentions and what the enclosing protocol declares.

Three clauses follow:

- **`self` inside a provided body is the implementing instance**, reachable only
  through the protocol's own members. Contrast §21.1, where `self` inside a
  static body is the type.
- **A provided body may call required and other provided members**, which is what
  makes many-from-few work at all, and may recurse.
- **A protocol declares no state.** There is no protocol-level field, and adding
  one would be the `@items` hazard by design rather than by accident.

## 34.5 Composition is declined for now

Composition would be one protocol requiring another — `proto Seq` requiring the
iteration protocol, so that `Seq`'s provided bodies may call its members and
`impl Seq` implies conformance to both. Rust's supertraits. **Declined for now,
and explicitly revisitable.**

**What declining costs is smaller than it looks.** A derived protocol restates
its prerequisite's *required* verbs and stays fully self-contained under §34.4:

```
proto Seq
    def next end            # restated, not inherited
    def size … end
    def empty? … end
end
```

An implementing type writes `impl Iter, Seq` and provides `next` **once** — one
member satisfies both protocols, since conformance is per-signature. There is no
duplicated implementation, only one restated signature line per shared verb in
the protocol declaration, which is stdlib source written once.

The real cost is narrower and worth naming: when a derived protocol wants a
prerequisite's **provided** member rather than its required one. If `Seq`'s
`contains?` wants the iteration protocol's `find`, restating `find` as required
would force every implementor to write it, defeating the purpose, so `Seq`
reimplements it over `next`. That is genuine duplication of default bodies, it
lands entirely in stdlib source, and it is invisible to users.

**It is worth less in mesa than the Rust analogy suggests.** Supertraits earn
most of their keep through generic bounds, propagating requirements through
signatures in `where` clauses. Mesa has no parameter types and no bounds, so the
implication "every `Seq` is an `Iter`" has no signature to travel through. Under
§31.3's conformance test it becomes the difference between asking `x is Seq` and
asking `x is Seq and x is Iter` — expressiveness, not capability. The analogy
oversells it.

**Reversibility, stated precisely, because "cheap later" was overstated when
first said.** Adding composition later is additive to *program validity*: a
protocol that restates its prerequisites' verbs stays legal, so no existing
program becomes invalid. Removing it later is the reverse ratchet, breaking
every program that impl'd a composed protocol. That asymmetry is the argument,
and it is about validity rather than about zero work — the stdlib refactor is
real.

Two preservations make the later migration free, and both are cheap now:

1. **The conflict rule is scoped to *unrelated* protocols** (§34.6). "Related" is
   undefined because it is currently impossible. Writing the rule as a flat
   "two protocols providing the same name is an error" would mean that
   composition, when it arrives, introduces a derived-wins resolution — a
   linearisation-flavoured rule arriving after §34.2 claimed there would be
   none. Wording avoids that.
2. **When composition arrives, `impl Seq` must *imply* `impl Iter`** rather than
   requiring both to be written. Then collapsing the stdlib's restated
   signatures is invisible to existing code.

*A third joins them in §34.10*: the required-member rule must then read against
the **effective** required set, inherited members included, or a derived protocol
that is all-provided over an inherited verb becomes illegal — which is exactly
Ruby's `Enumerable`, the legitimate version of the shape §34.10 rejects.

**The honest residual.** If the protocol set grows deep — iteration, sequence,
sorted, indexed, each building on the last — the restating grows with it and the
later refactor is real work. The judgment rests on the set staying shallow, which
§32.1's nine plus §32.2's deferred tenth supports and a larger stdlib ambition
would not. That is the thing to re-examine if this decision comes back.

## 34.6 Overriding, conflicts, and the namespace

**Overriding.** A type supplying its own member overrides the provided body.
This is §32.1's automatic-and-overridable pattern extended from derived equality
and debug printing to derived methods, so it is a familiar shape rather than a
new one. **No marker**, on §30.5's grounds: the ceremony would land on every
override and the `impl` line already names where to look.

**Conflicts.** A type implementing two **unrelated** protocols that both provide
the same name is a **declaration-time error**, resolvable by the type supplying
its own member. The error names its own fix, and no ordering rule decides
silently. Today every pair of protocols is unrelated; §34.5 explains the wording.

**The namespace.** §21.1 puts statics, variants, and nested types in one member
namespace with instance members, so provided members collide with all of them,
and every collision is a declaration-time error for the same reason.

**Conformance checking amends.** §31.1's rule was that a protocol's required
members either exist or the type declaration is an error. It now reads: every
**required** member must exist; provided members are filled in where the type is
silent. Which is the same rule with the set it quantifies over made explicit.

## 34.7 §0.4's signature-defaults thread closes

§31.2 left open whether a protocol signature may carry parameter defaults
(`def get(key, default: nil)`), and if so whether implementors inherit them or
repeat them. **Yes, and they repeat them.**

The objection to inheriting was that it is "a default body by the back door for
the one thing a body can express without reflection," which is void now that
bodies are the front door. Repeating is nonetheless still right, for a reason
that has nothing to do with that objection: §31.2 already requires an
implementor's signature to fix the same parameter *names*, because §21.3 makes
names public API. A default is part of a signature under §21.3, so it is part of
what the implementor matches, and a disagreement is a declaration-time error.
That is "duplication the compiler could check" — §31.2's own phrase for the
option — with the check actually specified.

No question arises for provided members: their defaults live in the body's own
signature, and there is nothing to inherit.

## 34.8 The defaults are written in mesa

**Built-in protocol defaults are mesa source, shipped with the language.** Not
Rust natives registered under §29. The derived library is therefore readable in
the language it belongs to, which for a stdlib of this size is worth an
interpreter round trip; mesa is not competing on speed, and §13.4's architectural
payoff argued for one dispatch path rather than for a fast one.

**What it needs is not §28's module system.** The minimum is a loader that
evaluates embedded mesa source into the prelude tier (§20.6) before the user's
module runs. No import syntax, no dotted paths, no import graph, no cycle
detection, no `Obj::Module`, no read-only member rules — none of §28.1's
machinery. The dependency is the prelude tier, which §20.6 already settles and
which §14.5's resolution order already has a place for.

**This widens a concession already taken rather than making a new one.**
ROADMAP §3.2a decided that the built-in protocol *names* get a temporary prelude
binding until the module system lands, explicitly as scaffolding "to be removed
when the modules it stands in for exist," and noted that DESIGN_NOTES did not
carry the reasoning. This section carries it, and widens the scaffolding from
names to definitions: the protocols and their provided bodies are evaluated into
the prelude tier now and move into built-in modules when §28 lands. §20.7's
small-prelude principle is not violated so much as its exception is widened, and
the exception's expiry is unchanged.

Three consequences, none large and none previously on the list:

- **Stdlib code needs a source identity.** §19's `Location` derives file, line,
  and column from a byte offset in a `Source`, and a stdlib error has no user
  file to name. It needs a synthetic source name, and §5's traceback will show
  stdlib frames — probably wanted, but it is a presentation decision rather than
  a free consequence.
- **Startup cost becomes a budget.** Every program parses and evaluates the
  stdlib. Acceptable at mesa's priorities; worth knowing before the stdlib grows,
  because nothing else in the language has a per-run fixed cost.
- **The unshadowable check now covers definitions.** §20.6's declaration-time
  prohibition on binding a prelude name applies to everything the stdlib
  declares, which is the intended behaviour and makes the stdlib's namespace a
  compatibility commitment.

**Bootstrapping is not a hazard.** Protocols are declarations, so a provided body
calling a member declared later in the same source is the ordinary forward
reference §33.5's two sub-passes already exist for — the same case as a match arm
naming a case type declared further down. This raises §33's value; it does not
force it earlier.

## 34.9 A seam under §20.8's open question

§20.8 left one thread live: "Uniform dispatch or cheap native loops; probably not
both." §13.4 hoped protocols would collapse the hardcoded matches in
`Expr::Script` and `Expr::Each` into a single dispatch path, and special-casing
`Obj::List` for speed re-creates the split.

A mesa-source derived library puts a seam in it. **Native types implement the
base verbs natively — a `NativeMember` under §29 — and the derived library is
mesa, calling them through the protocol.** So `List` has a native step, and
`map`, `filter`, and `fold` are mesa source written once for everything that
implements the protocol. The two-path split then applies to a handful of base
verbs rather than to the whole collection surface, which is a much smaller
inconsistency than the one the thread was worried about.

What stays open is narrower than before: whether `each` over a `List` pays an
interpreted step per element or takes a native fast path. That is now a question
about one construct, not about the shape of the library.

## 34.10 Protocols made only of bodies

A protocol declaring provided members and **no** required ones is a mixin
outright: methods attached to a type with no contract to check, since there is
nothing for the type to supply. That is the shape §2 ruled out, and the rules
above do not by themselves reject it. This does.

**§34.4 already removes the dangerous half.** An all-provided protocol's bodies
may reference only their parameters, each other, and module and prelude names,
so they cannot observe or mutate the implementing instance at all. Ruby's
fragility is a module body reaching for `@items` — state it neither declares nor
checks — and mesa's cannot. What survives is **namespace injection**: `impl
Helpers` puts `format_thing(x)` on a type, nothing is checked because there is
nothing to check, and a reader seeing `x.format_thing(1)` follows the `impl` line
to arrive at code that could equally have been a module function. That is a
second and worse import mechanism wearing `proto`'s clothes.

**The rule cannot simply be "a protocol needs a required member,"** because that
kills markers. A protocol with **no members at all** is meaningful under §31.3,
which settled that conformance is testable: `x is Serializable` with nothing to
implement is Rust's `Send` and Java's `Serializable`, and §31.3 made exactly that
askable deliberately. Three shapes, not two:

| Members | Reading | Verdict |
| --- | --- | --- |
| None at all | A marker; the conformance *is* the content | **Legal** (§31.3) |
| Provided only, zero required | Namespace injection, nothing checked | **Error** |
| At least one required, any provided | The intended shape | Legal |

**The rule: a protocol declaring provided members must declare at least one
required member.** Provided members exist to be derived *from* something; with
nothing to derive from they are not derived, they are injected. All nine of
§32.1 pass — `compare` for ordering, both subscript halves, append's verb, the
step verb for iteration, one verb each for display and hashing. Equality and
debug derive structurally in the language rather than through provided bodies, so
they never engage the rule.

**What this clarifies about §34's scope.** What defaults buy is leverage over a
type's own contract, not attachment of arbitrary functions to a type. §34.1's
argument was about *derived* surface — surface computed from a type's own verbs
— and a helper deriving from nothing was always a module function. The
free-function objection that decided §34.1 never applied to it, because there is
no receiver being denied its verbs.

**It becomes a third preservation for composition.** When composition arrives the
rule must read against the **effective** required set, inherited members
included, or a derived protocol that is all-provided over a prerequisite's
required verb becomes illegal — and that shape is Ruby's `Enumerable`, which is
the legitimate version of the thing. Under §34.5's declining the question cannot
arise, since a derived protocol restates its prerequisite's required verbs and so
already has a non-empty required set. Recorded beside §34.5's two so it is not
missed when composition is revisited.

**One honesty note.** This makes the mixin shape inconvenient and visible, not
impossible. A determined user declares one trivial required member, implements
it, and has namespace injection anyway. What they still do not have is access to
the implementor's state, and what a reader still has is a type declaration naming
a protocol naming a contract. Same posture as §34.2 — priced and bounded rather
than defeated — and worth stating rather than claiming an airtight rule.

## 34.11 What this leaves open

**Eagerness.** Whether the combinators are lazy or eager is untouched here and
wants deciding before the library is written. Two arguments lean eager. A lazy
chain needs an adapter type per combinator — Rust's `Map<I, F>`, `Filter<I, P>`
— which under §21.2 are local types in the stdlib, and each stage adds an
interpreted step per element. Eager returns a `List`, which has native members,
so a chain lands back on native ground after one stage. Against that, eager
allocates per stage and cannot express an infinite source. Not settled; recorded
in §0.4.

**The collection protocol §32.2 deferred gets cheaper.** Its five derivable
members can be provided rather than required, so the objection that it is a
contract requiring identical boilerplate from every implementor is gone. It stays
deferred — the forcing case, views interchangeable with `List`, is unchanged and
still sits behind §22.6's `Bytes` — but the reason for deferring is now demand
alone rather than demand and cost.

**The library's contents.** Which combinators exist and what they are called is a
list rather than a mechanism, and it joins §0.4's naming threads. It blocks
nothing: a protocol can ship with `next` required and grow provided members
without breaking implementors, which is the one compatibility property defaults
add in mesa's favour.

---

# 35. The semantic analysis phase

§33 priced a *pass*. What has accumulated around it is larger than a pass and
has never been described as one thing: nine obligations settled in nine
different sections, plus §34's four clauses, spread across four different
mechanisms, sharing one property and one admission rule. This section names the
whole of it and states what it is responsible for.

**The name is prescriptive: the phase is called semantic analysis**, and §33.7's
open naming thread closes with it. §33's pass is one carrier inside the phase
and keeps the name *resolver*, because it does resolution; the phase is the
larger thing, because most of what it does is not resolution.

Nothing here is a new decision about the language. What is new is the boundary —
§33 argued for a pass and then deflated seven of its nine jobs out of it — five
into `eval_decl`, two into mechanisms that are not the pass at all — without
saying what the seven then belong to. The answer is *this*, not nothing.

## 35.1 The phase is larger than the pass

§33.2's deflation splits the work across four carriers and never says so
outright:

| Carrier | Runs | Carries |
| --- | --- | --- |
| The parser | During parse | `break`'s proc boundary, if §0.4's thread lands there |
| The loader | Before any module is evaluated | The import graph (§28.4); the stdlib's own evaluation (§34.8) |
| `eval_decl` | As each declaration is evaluated | The deflated set — conformance and its four clauses, `Eq`/`Hash`, the prelude prohibition, both rightward-reference checks |
| The resolver pass | Between `Parser` and `Interpreter` | Name-to-slot resolution, match coverage, capture sets |

*Two carriers, after §41.2.* The parser's only entry was `break`, which §41.4
moves into `sem`; `eval_decl`'s whole row goes with it, since none of those
checks may raise `sem::Error` from `rt`. What is left is the loader and the pass,
and the pass's module holds both — `sem` is named for the phase, not for
resolution, which is already true of what is built. The argument below is
unaffected and gets easier: it explains why mechanisms this different are one
phase, and there are now two rather than four to explain.

Four mechanisms, one phase, because what makes them one thing is not when they
run but what they are permitted to ask and what they jointly guarantee. §33.3
states the guarantee as a property of the pass; it is a property of the phase:

> Every error determinable from the source alone is reported before the program
> produces its first side effect.

That is what the deflated seven belong to. A check living in `eval_decl` is not
a check that has been demoted out of the phase — declarations evaluate before
anything that uses them, so an `eval_decl` check still fires before the first
side effect, which is the whole of what the property asks. §33.2's phrase for
it, "declaration-time in a sense mesa already has," is the point rather than a
concession.

**One carrier is doing work that is not a check.** The loader evaluates the
stdlib into the prelude tier (§34.8), which is not analysis at all. It is in the
phase because §20.6's unshadowable check quantifies over what it produces: the
prelude's contents are not knowable until it has run, so it is a *prerequisite*
of an obligation rather than an obligation. Worth stating so the phase's
boundary is not read as "everything before execution," which would be false —
the stdlib's own evaluation is execution, just of something the user did not
write.

## 35.2 The obligations, and what carries each

The full set, with §33.1's table as its spine and the sections that arrived
since folded in:

| # | Obligation | Settled in | Carrier | Complete there | Forced into the pass |
| --- | --- | --- | --- | --- | --- |
| 1 | Name-to-slot resolution | §3.4, §33.1 | Pass | — | It *is* the pass |
| 2 | Match coverage, and the dead-`else` error | §17.6 | Pass | — | **Yes** — the one forcing function |
| 3 | Capture sets for proc literals | §16.2, §30.3 | Pass | — | No, but per-evaluation without it |
| 4 | The unshadowable-prelude check | §20.6, §34.8 | `eval_decl` | **No** — misses in-body bindings | No, but partial there |
| 5 | A parameter default referencing rightward | §24.4 | `eval_decl` | **No** — misses proc literals | No, but partial there |
| 6 | A body field referencing rightward | §26.1 | `eval_decl` | Yes | No |
| 7 | An overridden `Eq` requiring a supplied `Hash` | §24.6 | `eval_decl` | Yes | No |
| 8 | Protocol conformance | §31.2, §34.4, §34.6, §34.7, §34.10 | `eval_decl` | Yes | No |
| 9 | `break` crossing a proc boundary | §29.2 | Parser (§0.4, open) | — | Not this phase's, if the parser carries it |
| 10 | The import graph | §28.4 | Loader | — | Never — it runs over paths, not an AST |

*Amended by §41.* Every `eval_decl` in the carrier column reads `sem` (§41.2),
and row 9's carrier is `sem` rather than the parser, with "complete there: Yes"
and no longer outside the phase (§41.4). The completeness column is untouched —
§41.2 moved where the checks live, not what they can reach, and rows 4 and 5 are
partial in `sem` for exactly the reason they would have been partial in
`eval_decl`. Row 4 is built and partial today. **And the table is not an
inventory**: §4.6 cl. 12, required parameters preceding defaulted ones, is built
in `sem.rs` and appears nowhere on this list, so the count of nine is what had
been argued about rather than what the phase does (§41.8).

**The completeness column is not a restatement of the forcing column**, and the
two obligations where they disagree are the interesting ones. "Forced" asks
whether the rule is *checkable at all* without the pass; "complete" asks whether
`eval_decl` reaches every place the rule applies. Obligations 6, 7 and 8 attach
to a `type` or a `proto`, and §3.1 keeps those top-level while §21.2 nests local
types inside *types* rather than inside proc bodies, so `eval_decl` reaches all
of them. Obligations 4 and 5 attach to something that is not a declaration:

- **§24.4's rule is about a parameter list**, and §16.6 gives proc literals
  parameter lists. A literal is an `Expr`, so `eval_decl` never sees one — not
  even at module level, where `f := def (a, b: c, c) … end` sits on the right of
  an assignment rather than in a `Decl`.
- **§20.6's rule is about a *binding***, not a declaration, and §3.2's implicit
  declaration makes `Str := 5` inside a proc body a binding. `eval_decl` cannot
  reach it. *Stated as a reading rather than as settled:* §20.6 does not say
  outright that the prohibition covers locals inside bodies. It has to, or it is
  not a prohibition — §14.5 resolves locals first, so an in-body `Str := 5`
  shadows the prelude exactly as §20.6 says user code cannot — but if that
  reading is wrong then obligation 4 is complete in `eval_decl` and only
  obligation 5 carries a gap.

**Both gaps have §33.2's shape.** What `eval_decl` cannot reach is anything
inside a body, which is precisely the argument that forces the pass for coverage.
Coverage is where it was noticed, not where it is confined. So §33.2's "the
honest number of forcing functions is one" is exact for the nine as they stood
and becomes softer once proc literals land: obligations 4 and 5 do not *need* the
pass to be checked, but they need it to be checked everywhere they apply.

**Obligation 8 is five checks wearing one number**, which §0.1 records and is
worth expanding once so the shape is visible. Two fire at a protocol's own
declaration and need no implementor: a provided body referencing outside its
protocol (§34.4), and provided members declared with no required member beside
them (§34.10). Three fire at an implementing type's: required members present
with matching parameter names (§31.2), signature defaults agreeing (§34.7), and
no two unrelated protocols providing the same name — or colliding with a static,
variant, or nested type in the same member namespace (§34.6, §21.1).

They are one obligation because they are one question asked from both ends —
*does this declaration honour the contract it names* — and because they share a
carrier and a failure mode. Counting them as five would put the total at
fourteen and suggest fourteen mechanisms, which would be wrong in the direction
§33 was written to correct.

## 35.3 What the phase quantifies over

The obligations are unremarkable individually. What is not is the structure they
all read, which has grown considerably and is written down nowhere:

| Entity | Declared by | What the phase knows |
| --- | --- | --- |
| Native type | `CORE_TYPES`, prelude-bound (§13.6, §20.6) | Name and member set, fixed before any source is read |
| User type | `type` | Constructor fields, body fields (§26), methods, statics (§21.1), nested and local types (§21.2), the protocols it names |
| Case parent | `type` with `case` members (§17.1) | Its variant set, and that it is not constructible (§17.9) |
| Variant | `case` | Its parent, its own fields, and the one-step method fallback (§17.7) |
| Protocol | `proto` | Required and provided members, each with a signature fixing parameter names and defaults (§31.2, §34.3, §34.7) |
| Conformance | `impl` on a type | Which protocols a type asserts, and therefore what it must supply and what it acquires |
| Module surface | A file (§28) | Its top-level names, and its position in a static acyclic graph (§28.4) |
| Prelude tier | §20.6, widened to definitions by §34.8 | Every name and definition the stdlib declares |

**Every entry is fixed at its declaration and enumerable from source.** That is
§1's "an instance's shape is bounded by its type," widened by §17.7 to include a
parent's methods and by §34 to include a protocol's provided members, and it
holds for the same reason each time: §0.1's third clarification permits
introspection and rules out self-modification, so nothing in the table can be
added to at runtime.

**§3.4 named the one thing that would have broken it**, and it is closed.
Instance-level method replacement (§2.1) would make "field of self" against
"method of self" undecidable before execution — "currently the *only* thing
standing between mesa and full static resolution." §14.8's paren-less calls
closed it as a side effect, since `Member::set` must reject names resolving to
methods for auto-invoke to be coherent. The model above is therefore total
rather than mostly-total, and it got there through a feature that was not aiming
at it.

This table is the honest answer to "mesa's implicit type system is growing." It
is growing, and it is not a type system. What it is is a declared-entity model,
and the next section is why the difference is not a quibble.

## 35.4 A growing check set is not a growing type system

The worry is reasonable on its face. The phase started as name-to-slot
assignment, now performs conformance checking against signatures that fix
parameter names and defaults, verifies exhaustiveness over variant sets, and
enforces scope rules on protocol bodies. Interface checking, exhaustiveness, and
signature matching are things type systems do.

**The distinguishing property is what each obligation quantifies over.** Every
one of the ten ranges over entities in §35.3's table — declarations, their
members, their names, their relations. Not one ranges over what values reach a
given expression. There is no judgement anywhere in the phase of the form *this
expression has this type*, and every obligation would be unchanged if one were
added, because none of them consults one.

Three obligations look closest to typing, and each is explicitly built to avoid
it:

- **Coverage** (§17.6) reads the arms, not the scrutinee, and §17.6 opens by
  saying it cannot do otherwise: "a resolver looking at `when e` has no idea `e`
  is an `Expr`." A construct that *would* need the scrutinee's type is instead
  decided from a syntactic property of the arm set.
- **Opt-in optionals** (§18.5) get their force from coverage, not from a
  declared return: "static enforcement at the use site, with no type system
  involved," with the limit stated in the same breath — nothing requires a
  search to return a `Maybe`, and which APIs opt in is convention.
- **Conformance** (§31.2, §31.3) splits on the rule cleanly. *Does this type
  supply what it claims* is a question about a declaration and is the phase's;
  *does this value conform* is a question about a value and is runtime.

**And the set does not converge on one by accretion.** This is the part worth
stating, because "it keeps growing" implies a destination. It has none: adding
an eleventh obligation about declarations yields eleven obligations about
declarations. Becoming a type system requires a *discrete* move — giving
expressions types and checking their flow — which no obligation here is a step
toward, which no accepted row in §0.2 asks for, and which §33.6 refuses by
construction. The phase can grow indefinitely along its own axis without ever
arriving at the other one.

**The tripwire, named so it is recognisable if it ever arrives.** The move would
announce itself as a *declaration* carrying a type: a parameter annotation, a
declared return, a typed field. Each is a declaration, so each would pass
§33.6's admission rule as written — the rule refuses questions about values, and
an annotation is not one. What makes them the discrete move is not the
declaration but what a checker would then be obliged to do with it: an
annotation nothing verifies is decoration, and verifying it means typing the
expressions that flow into it. §18.5 identifies the exact one that would be
wanted first — "Swift's force comes from the declared return type" — and mesa
declines it there for reasons that are about the language rather than about the
phase's capacity.

So the admission rule holds the line for the cases it was written for, and this
paragraph is the amendment for the one case it does not cover: **a declaration
whose only purpose is to be checked against values is the move, whatever it is
spelled as.**

## 35.5 What stays at runtime

The complement matters as much as the list, because a phase this size invites
the assumption that a program passing it is a program that works. It is not
that, and §33.3 says so — "not a claim that a resolved program cannot fail."

Staying at runtime, each because the question is about a value:

- **Call arity**, and every argument error §21.3's `ArgumentError` group covers.
- **Member existence** on `x.foo`.
- **`Member::set` rejecting a method name** (§2.1, §14.8) — a name resolving to
  a method is a fact about the receiver in hand.
- **Conformance testing** (§31.3), whichever spelling §0.4's thread takes.
- **Index and key errors** (§18.2, §23.1) — out of range, non-integral, missing.
- **Constructing a case parent** (§17.9) — rejected at the call, not at the
  declaration, because the declaration is legal and the call is the error.
- **Every type error** in the ordinary sense: `+` on a non-`Num`, `<` on
  unorderables, calling a non-callable.

The pattern is worth naming: the phase checks that *declarations are coherent*,
and the interpreter checks that *values fit the operations applied to them*. Two
different jobs that happen to share the word "check."

## 35.6 Carriers move; the obligation set is what is stable

§0.2 carries "can live in `eval_decl` until the pass exists" on several rows
independently, which reads as a per-row convenience and is actually one rule:

**An obligation's carrier is an implementation choice; its membership in the
phase is not.** The property in §33.3 is indifferent to which mechanism fires,
so obligations 6, 7 and 8 may start in `eval_decl` and move into the pass when
the pass arrives, with no change to the language, to any program's validity, or
to what the error says. Only the pass's forcing function is fixed: coverage
cannot be carried by `eval_decl`, because checking a `when … case` inside a proc
body means walking proc bodies at declaration time, which is the pass under
another name (§33.2).

**The exception, and it is not cosmetic.** Obligations 4 and 5 are *incomplete*
in `eval_decl` rather than merely housed there (§35.2), so moving them is not a
free relocation: a program with a rightward default in a proc literal, or a
prelude name bound inside a body, is accepted before the move and rejected
after. That is a change in program validity, which is the one thing the rule
above claims migration never causes. The rule holds where the carrier can reach
everything the obligation covers, and these two are the cases where it cannot.

Which makes the honest statement of it slightly weaker and more useful: **a
carrier is an implementation choice wherever it is complete.** Where it is not,
choosing it is choosing a subset of the rule, and the subset should be recorded
as such rather than booked as the check being done.

Two consequences for sequencing. **The phase exists already**, in scattered
form, the moment the first of these checks is built — so building them in
`eval_decl` is not a temporary hack awaiting a real phase, it is the phase with
one carrier under-used. And **§33.7's timing advice is unaffected by this
section**: the pass still lands when coverage forces it, and the deflated set
still moves for a few lines each.

## 35.7 The error type follows from the name

§33.7 made the error type's name mechanical once the phase had one: `syn::Error`
and `rt::Error` are named for their phases, so the third is `sem::Error`, and the
module is `sem`. It keeps everything §33.5 specifies — a variant per failure
mode, a `loc()` accessor, a `Display` impl producing
`file:line,col: semantic error: …`, and no path into `rescue`, structurally
rather than by policy, since the program has not started and there is no frame to
unwind.

~~**One consequence of naming the phase rather than the pass.** The error type
belongs to the phase, so a check firing in `eval_decl` raises a `sem::Error`
rather than an `rt::Error`, even though it fires from inside the interpreter.
That is the right split — the error is not rescuable and is not part of §19's
script-visible surface — but it means `eval_decl` has two error channels for a
while, and the second one disappears into the pass as obligations migrate. Worth
knowing before the first of these checks is built, because getting it wrong puts
a non-rescuable failure into a rescuable enum and §19 makes that enum public
surface.~~

*Reversed by §41.2, and the paragraph's own last sentence is why.* "Worth knowing
before the first of these checks is built" was right, and when that check was
built the conclusion inverted: rather than give `eval_decl` a second channel, the
check moved to a module where it needed only one. The rule is that **a check
lives in the module its error type names**, so no `sem::Error` is ever raised
from `rt` — or from `syn`, which is the case this paragraph's rescue-based
reasoning cannot reach and §41.2's can.

What survives unchanged is the hazard identified: a non-rescuable failure must
not enter §19's public enum. That premise is what forced the reversal rather than
being weakened by it. What does not survive is the inference from it — the error
type does not travel to the check, the check travels to the error type — and with
it §4.7 cl. 12's freedom, which now ranges over carriers inside `sem` only.

## 35.8 What this leaves open

**Whether desugaring joins the phase.** Unchanged from §33.7: §20.8's `each`
desugaring would sit naturally in a pass that already walks every body, nothing
requires it there, and §20.8's own open question about uniform dispatch is what
would decide it.

~~**Where `break`'s proc boundary lands** (§29.2, §0.4). If it stays in the parser
it is the one obligation on the list that the phase does not carry; if §0.4's
re-examination moves it, the phase gains a tenth with no other change. Either answer is
consistent with everything above, which is why it can stay open.~~

*Closed by §41.4: it lands in the phase, and `L10` closes with it.* The second
branch, and it costs slightly more than "no other change" — `break` is the first
obligation that walks a body, so it converts `sem` from a declaration walker into
the traversal §33 priced. What it buys is that §4.7 cl. 13's two partial checks
become completable and `C6` and `C7` arrive to find their traversal built
(§41.5).

**Composition's amendment, if it returns.** §34.5 and §34.10 record three
preservations that keep protocol composition free to add later. The third is an
obligation-level change — §34.10's required-member rule would have to read
against the *effective* required set, inherited members included — so it belongs
on this list as well as on §34's, since it is the only one of the three that
changes what the phase checks rather than what the language permits.

**Nothing here is a naming thread.** §0.4's protocol names and step verb are
names for language surface; the phase's name is settled above, and its error
type's follows.

---

# 36. The Rust/mesa seam

§34.8 settled that the derived library is mesa source rather than Rust natives,
and §34.9 named the seam that creates: native types implement the base verbs
natively, and the mesa library calls them through the protocol. What neither
section says is how the two sides find each other. A protocol declared in mesa
has no compile-time identity for Rust to hold; a native type has no `impl` line
to declare conformance with; and the mesa half is an interpreter round trip over
data structures the Rust half implements directly.

Three questions — **identity**, **conformance**, and **cost** — and the third
has measurements behind it rather than argument.

Nothing here reverses §34. What it does is price §34 and name the pieces it
assumed without describing. One consequence in §36.2 is a genuine narrowing of
the language and is flagged as wanting acceptance rather than presented as
settled.

## 36.1 Protocol identity is fixed at startup, not at compile time

`Sym::TYPE` and `NativeTypeId::LIST` are Rust constants because interning order
and `CORE_TYPES` order are fixed by construction. A protocol declared in
embedded mesa source cannot be one: its identity is assigned when the loader
evaluates the declaration, which is a runtime event.

**The gap is smaller than it looks, because the loader runs before any user
code.** §34.8's loader evaluates source that ships inside the binary, at a known
point, in a fixed order. So the built-in protocols do not get *compile-time*
constants; they get **startup-time constants**, resolved once immediately after
the loader finishes and held in one structure for the rest of the run. Every
Rust site that needs to reach a protocol — the `<` arm, the `<<` arm,
`Expr::Script`, `each`'s desugaring — reads that structure.

**A name alone will not do, and the reason is §13.4.** It is tempting to skip
protocol identity entirely and have `<` look up a member called `compare`. That
is opt-in by magic name in the form §31.4 declines: a user type defining
`compare` for its own reasons, without `impl Ordering`, would silently acquire
the operator. Dispatch has to ask *does this type conform to this protocol*, so
the Rust side holds a protocol identity and not merely a verb `Sym`.

**A missing or misnamed entry is an interpreter bug, not a program error.** The
stdlib is not user input; it is shipped, and if it does not declare what the
interpreter expects, the build is wrong. So resolution failure is a panic, not a
`sem::Error` and not an `rt::Error` — §35.7's split is untouched, because this
is not a check on a program.

**The table is the mirror image of `CORE_TYPES`.** Rust holds a list of names it
expects the stdlib to have declared, in the same spirit as `CORE_TYPES` holding
the names it expects to register. The coupling between the two halves exists
however it is spelled; putting it in one table gives it the property
`CORE_TYPES` already has — one place to change, one place to audit — instead of
scattering it across the operator arms that consume it.

## 36.2 A native type's conformance is declared in Rust

§20.1 attaches `impl` to a type declaration. `List` has no mesa declaration, so
there is nowhere for its `impl` line to go.

**The alternative is worse than the problem.** Letting stdlib source say
something like `impl Iter for List` would be reopening a type from outside it,
which is the self-modification §0.1's third clarification rules out and which
§34.2 leans on directly: "the member set is fixed at the protocol's declaration
site." A stdlib that can monkey-patch native types is a stdlib with a capability
no user code has, and §34.2's five separating properties would be five
properties that hold only for user code.

So **native conformance is declared in Rust, in §36.1's table**: alongside
"ordering is the protocol named `Ord`" sits "`Num`, `Str` and `Char` implement
it, and their required verb is this `NativeMember`." The stdlib stays ordinary
mesa; nothing in it is privileged.

**The consequence is a real narrowing, and it should be taken deliberately
rather than discovered.** A user protocol can never be implemented by a native
type. `proto Json` with `List` conforming to it has no spelling, because `List`
has no declaration to carry the `impl` and the Rust table is closed. Rust's
`impl MyTrait for Vec<T>` has no mesa equivalent.

Three things make this less severe than the bare statement:

- §27 already declines visibility and §0.1 already declines reopening, so the
  capability being denied is one mesa has consistently denied elsewhere. This is
  that policy reaching a place it had not yet been noticed reaching.
- The user's fallback is a wrapper type, which is a real cost but a familiar
  one, and §34.1's argument against free functions does not apply — a wrapper is
  a noun with verbs.
- It is additive to fix later. A form for declaring conformance away from the
  type — Rust's orphan-rule-free impl, Swift's retroactive conformance — could
  be added without invalidating any program, and declining it now costs nothing
  that adding it later would not recover.

*Recorded as an open decision rather than as settled*, because unlike the rest
of this section it is a statement about what users may write. §0.4 gains it.

## 36.3 The seam wants a check, and the check wants parameter names

Nothing in §36.2's arrangement verifies itself. If the stdlib renames the
iteration step verb and the Rust table is not updated, the failure surfaces as a
missing member deep inside a combinator, at the first `each` a user writes.

**The fix is to run the check `impl` already runs.** §35.2's obligation 8
checks, at an implementing type's declaration, that every required member is
present with matching parameter names (§31.2) and agreeing defaults (§34.7). The
native conformance table asserts exactly the same thing about a `NativeType`, so
it should be checked exactly the same way — at boot, once the protocols are
loaded and their required sets are known. It is cheap, it runs over a fixed
table, and it converts a whole class of interpreter bug into a startup panic.

**This gives the phase an eleventh obligation, with a carrier it already has.**
§35.2's table lists ten; this is the eleventh, carried by the loader, which
§35.1 already identifies as a carrier doing work that is not a check. Unlike the
other ten it can never move into the pass, since it reads a Rust table rather
than an AST — the same reason §33.2 gives for the import graph.

**And it forces a shape on `NativeMember`.** The check compares parameter
*names*, and `NativeMethod` today carries `arity: usize` and no names at all. So
§29's fold gains a requirement: a native member records its parameter names, not
a count. Worth noting that §21.3 forces the same edit independently — keyword
arguments make parameter names public API, and §0.2 already pairs the arity
change with §29's fold as one pass over `CORE_TYPES`. Two unrelated sections
asking for the same field is the usual sign it is the right field.

## 36.4 The receiver widens to `Val`

This one is a straightforward consequence that nothing currently records, and it
blocks the first provided body that runs.

Ordering's provided members are `min`, `max`, `clamp` and `between?` (§34.1),
and the types that most want them are `Num`, `Str` and `Char`. Running a mesa
provided body with `self` bound to `5` does not fit the runtime as built: the
interpreter's `self` receiver, the bound-method value's user arm, and `self` as
an expression all assume the receiver is a heap object, because until now every
receiver was a user instance. `Num` and `Bool` are immediates, and §22.3 makes
`Char` one deliberately.

**So `self` widens from an object handle to a `Val`**, and with it the
save-and-restore around a call and the field-fallback paths that read it. The
fallback needs an answer for a receiver with no fields — a `Num` has none —
rather than treating that case as impossible.

Two things worth stating about it. It is **not** a protocol feature: it is the
removal of an assumption that only held because provided bodies did not exist.
And it is **not** optional or deferrable, because §34.1's motivating example for
the whole of §34 — ordering's four derived members — is the case that hits it.
Any plan that lands the derived library before this lands the iteration half
only.

## 36.5 One member map, two kinds of member

§34's wording is that an implementing type *acquires* a provided member, which
reads as flattening at conformance time: copy the member into the type's own map
unless the type supplies its own. That makes §34.6's two rules fall out of the
data structure rather than needing to be implemented as rules — overriding is
"do not insert where a member exists," and a conflict is an insert collision.

For a user type this is free; its methods are already a map of mesa procs. **For
a native type it is not**, because that map holds Rust function pointers. A
`NativeType` that acquires a provided member has to hold a mesa proc beside its
natives.

So §29's fold goes one step further than §29 describes. It is not only
`NativeField` and `NativeMethod` becoming one `NativeMember`; it is the member
map becoming able to hold **either** a native member or a mesa proc. The
precedent is already in the tree on the static side, where one enum spans procs,
types, and both native kinds — instance members want the same enum the static
side already demonstrates.

**And it pushes user and native types toward one member map.** Both then hold
the same kind of entry, differing in how they were populated rather than in what
they contain. That is the sym table refactor the implementation notes already
defer; §34 raises its value, and this section is where at least its member-map
half stops being optional. *Stated as a recommendation rather than as settled* —
flattening could be declined in favour of searching the conformance list at
lookup, which costs a search per member access and saves the duplication.
Flattening reads as the better trade because lookup is hot and conformance is
fixed, but the alternative is coherent.

**Conflict checking then covers native types too**, at boot: two protocols
providing the same name with `Dict` conforming to both is a collision caught at
startup. That is also where §34.1's `get`/`has` duplication surfaces concretely
rather than as a scheduling note.

## 36.6 The boot sequence, and the one invariant it needs

Everything above orders itself into four steps:

| Step | What runs | What it produces |
| --- | --- | --- |
| 1 | `CORE_TYPES` registration | Native types with their native members only |
| 2 | The loader (§34.8) | Protocols and provided bodies in the prelude tier; protocol identities assigned |
| 3 | The seam resolution | §36.1's table bound; §36.3's check run; provided members flattened onto conforming native types |
| 4 | The user's module | — |

Step 3 is the whole of the seam, and it exists because step 2's output is not
addressable by Rust until something addresses it.

**The invariant this needs: the stdlib source is declarations only.** No
top-level executable code. A stdlib statement that evaluated `<` or `<<` during
step 2 would reach for a protocol identity that step 3 has not yet bound. The
constraint is cheap — the library is protocol declarations and their bodies, so
there is nothing it wants to execute at load — it is statically checkable, and
stating it removes a category of ordering bug that would otherwise be found by
hitting it.

**§34.8's bootstrapping argument does not cover this**, and the distinction is
worth keeping. That argument is about *forward references* inside declarations:
a provided body calling a member declared later in the same source is the
ordinary case §33.5's two sub-passes exist for. The invariant here is not about
references at all; it is about evaluation order between the loader and the seam.
Both are fine, for different reasons.

## 36.7 What the arrangement costs

§34.8 accepted an interpreter round trip on the ground that mesa is not
competing on speed, and §34.9 narrowed §20.8's open question to whether `each`
over a list takes a native fast path. Neither put a number on it. Measured
against the tree as it stands, in a release build:

| Measurement | Cost |
| --- | --- |
| An `each` step with a simple assignment body | ~390 ns/element |
| An interpreted proc call, on top of that | ~+400 ns/call |
| Parsing and evaluating declarations | ~0.4 µs/line |

*Method:* a 50,000-element list literal iterated 20 times, differenced against a
fixture that parses and allocates the same literal without looping; the load
figure from 2,000 lines of `def` declarations over 200 runs. One machine, wall
clock, no warmup control. **The absolute numbers will move** — the loop step is
high for the work it does, and scope lookup currently pays a cryptographic hash
to find a local — so what matters below is the ratios, which move much less.

**The startup half is settled by this, and is smaller than §34.8 feared.** A
2,000-line stdlib costs under a millisecond to load; 10,000 lines would cost a
few. §34.8's "startup cost becomes a budget… nothing else in the language has a
per-run fixed cost" is true as a statement about structure and does not need
treating as a constraint on the library's size. The budget is real and generous.

**The per-element half splits, and not along the axis §34.9 draws.** What
decides the cost of a provided member is whether its per-element work includes
calling a user proc:

| Shape | Examples | Mesa provided body against a native equivalent |
| --- | --- | --- |
| Takes a callback | `map`, `filter`, `fold`, `take_while`, `zip` | **~4x** |
| Does primitive per-element work | `size`, `contains?`, `first`, `last`, `empty?` | **~100–400x** |

The first row is the good case, and it is most of §34.1's motivating surface. A
mesa `map` pays a loop step, two protocol dispatches, and the callback; a native
`map` pays the callback. The callback is the dominant term and is unavoidable on
both sides, so the ratio is bounded — and it *improves* as the interpreter does,
because the overhead is the part that shrinks.

The second row is where it goes bad. A Rust loop does single-digit nanoseconds
per element; a mesa provided body does several hundred. On a 100,000-element
list that is a fraction of a millisecond against something close to a tenth of a
second.

**Interpreter work moves the threshold and does not remove it.** Plausible gains
without leaving the tree-walker — a non-cryptographic hasher, and slot-resolved
locals once §33's pass makes names static — might be five to ten times over.
Applied to both rows: the first improves from ~4x toward ~2.5x, and the second
from ~400x to perhaps ~50x. The deciding ratio is interpreted overhead against
real per-element work, and optimisation only ever moves the numerator. Native
code stays where it is. So there will always be derived members for which a mesa
body on a native type is the wrong answer; what changes is where the line falls,
not whether there is one.

*The dispatch costs above are extrapolated from proc-call cost, since no
protocol machinery exists to measure. A native member dispatch should be cheaper
than a mesa proc call, so 4x is more likely an overestimate than an
underestimate.*

## 36.8 Provided where there is a callback, native where there is not

§36.7 does not argue against §34. It argues for a rule about which members get
provided bodies, and **the mechanism it needs already exists**: §34.6 lets a
type supplying its own member override the provided body, with no marker.

> A member may carry a provided body when its per-element work includes calling
> a user proc. Where it does not, a native type overrides it natively.

Applied to the set: `List` inherits the combinators and overrides `size`,
`contains?`, `first`, `last`, `empty?`. The provided bodies still earn their
place, because a *user* type implementing the collection protocol gets all five
free and correct — which is exactly what §34.1 wanted, and what §34.11 means by
the collection protocol getting cheaper. That claim holds for user types and
inverts for native ones: providing `size` in mesa would be a large regression
against the native `size` `List` has today.

**The rule costs no new mechanism and no new concept.** It is a judgement
applied when the library is written, and §34.6's override rule is what makes it
a judgement rather than a constraint.

**One correction to §34.9's framing.** It says the two-path split "applies to a
handful of verbs rather than to the whole collection surface," and that the open
question narrows to whether `each` over a list takes a native fast path — "one
construct, not the shape of the library." The first half stands. The second is
too narrow twice over. Every combinator body is mesa, so *every* combinator pays
an interpreted step per element by construction, not only `each`; and the
members in §36.7's second row will be natively overridden on every native type,
so the split is base verbs plus roughly that set on each of `List`, `Dict` and
`Str`.

That is still far narrower than the whole collection surface, so §34.9's
conclusion survives — but the handful is larger than the word suggests, and it
is a predictable list rather than an open-ended one. What genuinely narrows is
the *consistency* question. The cost question does not narrow; it gets accepted,
and §36.7 is what it costs to accept it.

## 36.9 What this leaves open

**Whether a native type may implement a user protocol** (§36.2). The only item
here that is a decision about the language rather than about its implementation.
Declining is consistent with §0.1 and §27 and is additive to reverse; it is
listed because it should be accepted on purpose.

**Whether provided members are flattened or searched** (§36.5). An
implementation choice with no user-visible consequence, recorded because §34's
"acquires" wording implies flattening without settling it.

**How stdlib frames present in a traceback.** §34.8 lists the synthetic source
identity as a small consequence and the traceback question as a presentation
decision. §36.7 makes it less peripheral than that: once native base verbs can
fail, an error raised by a base verb called from a provided body has its
innermost frames in stdlib mesa, and a user calling `d.get(k)` sees them. That
is the normal path for the entire derived surface rather than an occasional
stdlib frame, so it wants deciding with §5's traceback rather than after it.

**Nothing here touches the laziness thread.** Whether the library is lazy or
eager is unaffected by everything above: both shapes have the same seam, the
same boot sequence, and the same split in §36.7. §36.8's rule applies to either,
and the adapter types a lazy library needs are user-level types, so they sit on
the mesa side of the seam by construction.

---

# 37. The exhaustion sentinel

§20.8 recorded the step sentinel as "plausibly `Maybe`" and never came back to
it; §4.15 cl. 4 carried it forward as `L3`, the last open mechanism thread in
`D7`. This settles it, and the answer is no.

The objection that started it was that a sum type feels incongruent with the
rest of the language. That reading is right, but the argument underneath it is
sharper than taste, and it applies to only one of the two jobs `Maybe` was
carrying.

## 37.1 One name, two jobs, joined by an economy claim

`Maybe` appears in two unrelated places, and §20.8 joined them: "one case type
serving both optionals and iteration exhaustion is real economy."

- **User-facing optionals** (§18.5). A return value an API opts into, matched at
  every use, enforced by §17.6's coverage check.
- **The step sentinel** (§4.15 cl. 4). Invisible, because `each` is surface
  syntax that desugars onto the protocol pair. No one writes `Maybe.Some` while
  looping.

**The economy claim is what makes the second look free**, and it is the join
that should be broken first. The two jobs answer to different criteria: the
first to whether mesa should have two ways to say *absent*, the second to cost
and to what an iterator author has to write. Decided separately they come apart,
and §37.3 and §37.4 decide them separately.

## 37.2 It would have to be mesa, and that is forced

Worth settling before the rest, because "put it in Rust" is the obvious way to
make a sentinel cheap and it is not available here.

A native `Maybe` buys nothing it exists for:

- **§17.6's coverage check is syntactic and reads the arms**, deciding that all
  arms name variants of one case type. A native type with variants would need a
  variant set visible to `C0`, which is machinery no `CORE_TYPES` entry has and
  which would have exactly one client.
- **§17.3's qualified names need variants that are types with members and
  constructors.** A native twin reimplements the case-type form beside the real
  one.
- **§18.5's enforcement *is* the mesa-level coverage check.** A native `Maybe`
  delivers none of the guarantee that motivates it.

So it would be an ordinary stdlib declaration plus an entry in §36.1's
startup-resolved table, declared in §36.6's step 2 and bound in step 3. The boot
sequence accommodates that. **What it would mean is that `each` — the most
common construct in the language — depends on a stdlib type declaration being
present and correctly named**, checked by §36.3. That is a heavier coupling
across the seam than anything else in §36, spent on a construct no user sees.

## 37.3 The sum type is declined, and the reason is §18's own

§18 spent a section making nil mean exactly one thing, *absent*, and truthiness
ask exactly one question, *is this present*. §18.5 then offers a second way to
express absence, differing from the first only in ceremony and in whether the
API author opted in.

**That is §13.7's problem in mirror image.** §13.7 named one value accumulating
six jobs. This is one job acquiring two values, with `when x then` on one path
and a two-arm match on the other, and nothing but convention deciding which an
API uses. §18.5 already concedes the guarantee is opt-in and "should not be
mistaken for Swift's". What it buys is enforcement at the use sites of the APIs
that opted in; what it costs is that the scripty reading stops being the reading
everywhere those APIs are used.

**Declining costs nothing structural, which is what makes this cheap.** `Maybe`
needs no language support: it is a case type any user or library may write, and
a library wanting un-ignorable absence can define and return one. What is
declined is making it *the language's* answer to absence, and making the
interpreter depend on it. §18.5 stands as a description of what a library may
do; it stops being a recommendation.

**And the economy claim falls with it.** With optionals out, the sentinel's case
rests on its own merits, which §37.4 finds are worse than an alternative.

## 37.4 A step is a `Bool` and a slot

**Settled: the iterator's step verb returns a `Bool`, and the element it
produced lives in a field on the state object §4.15 cl. 3 already allocates per
loop.** True means an element is in the slot; false means exhausted.

The property `L3` required is kept. §13.4's objection to nil-as-sentinel was
that a collection containing nil would end iteration early; here the signal and
the value travel on separate channels, so a yielded nil is a `true` with `nil`
in the slot and is not confusable with exhaustion. The same reasoning is why the
element cannot simply be the return value with a falsy signal: §18.4 leaves
`false` as a legitimate element, so **no value can serve as the signal**, and
the separate slot is exactly what buys that.

What it costs, named rather than discovered: **reading the slot after the verb
returned false is an invalid-state read**, which a sum type would have made
unrepresentable. The exposure is narrow, since the read is generated by `each`'s
desugaring in every ordinary case and the hand-written consumers are library
code. It is a real cost and is accepted.

What it saves is one heap allocation, one `Rc` pair, a match, and a field read
**per element, per stage** — which matters less today than it will. §36.7's case
for the interpreted round trip rests on overhead being the term that shrinks;
per-element allocation is the term that does not, and under a lazy library
(`L12`) it multiplies by pipeline depth.

## 37.5 Three alternatives, and why each is worse

**Exhaustion as a raised error**, Python's `StopIteration`. Fits `F2`, costs one
raise per loop rather than per element, and is the most "scripty" of the
options. Declined on Python's own experience: an error escaping an iterator body
becomes indistinguishable from exhaustion, and PEP 479 changed the language to
contain it. Repeating a mistake whose fix is documented is not a trade.

**Two verbs — a `done?` predicate and a read.** Declined, and the reason is the
one worth recording, because it is invisible until you look at what a source can
answer. Sources split in two:

- **Extent known independently of production.** A list compares an index against
  `size`; a dict compares a bucket cursor. `done?` is arithmetic and nothing is
  fetched.
- **Exhaustion discoverable only by attempting production.** A read returning
  zero bytes *is* how EOF is learned. `filter` cannot know whether a next
  element exists without running its predicate over upstream until one passes.
  `take_while` cannot know without evaluating its predicate on the element that
  may end iteration. A socket cannot know without waiting.

For the second class the predicate must perform the fetch and keep what it got,
so the iterator holds a slot **and a readiness flag**, because `done?` must be
callable twice without consuming twice. Four consequences:

- **The query is the effectful verb**, and §31.5's `?` suffix reads as a promise
  that it is cheap. On a socket, under §24.1's blocking model, `done?` is the
  operation that waits on the network.
- **§34.4 forbids the protocol from supplying the buffer.** A protocol declares
  no state and there is no protocol-level field, so the slot and flag are
  hand-written per iterator. `B13`'s body fields make that two declarations
  rather than much ceremony, but it is per-type, in the interpreted layer, and
  it is the boilerplate Guava's `AbstractIterator` exists to hide behind a
  four-state enum.
- **Indexable and streaming sources end up shaped differently**, where the slot
  form gives both one shape.
- **It does not remove the invalid-state read**, it relocates it: to calling the
  read verb when exhausted, which raises. So it pays the buffer and keeps a
  version of the cost that was its argument against §37.4.

Per element it is two dispatches against one dispatch and a field read.
Extrapolating §36.7, a second interpreted dispatch is order 400 ns against
~390 ns for an entire bare loop step, so for the mesa-written iterators that
streams and adapters will be, it roughly doubles the loop's per-element
overhead. Native iterators dispatch more cheaply and §36.7's dispatch figures
are extrapolated rather than measured, so the direction is firmer than the
magnitude.

**A unique sentinel singleton** compared by identity. Revives §13.4's "a new
kind of value in a language deliberately short on kinds of values" in its
cheapest form, and leaks: a user who stores the sentinel in a list silently
truncates iteration.

## 37.6 Generators are the other axis, and are deferred

*This subsection is the flag: revisit generators when the condition at the end
of it changes, and not before.*

Iterators-as-coroutines are a solution to how an iterator's state is
**written**; the sentinel is how exhaustion is **signalled**. They are
orthogonal, and Python has both separately — `yield` for the first,
`StopIteration` for the second. So generators would not have resolved `L3`, and
§37.4 does not have to be revisited if they ever arrive.

**What they would fix, and it is the real cost in §37.5.** A file's line
iterator becomes a `while` with a `yield` in it: no slot, no flag, no `done?`,
and the "exhaustion discoverable only by production" problem dissolves, because
production is the control flow and exhaustion is falling off the end of the
body. `filter` becomes a loop with an `if`; `take_while` a loop with a `break`.
That is precisely the boilerplate §34.4 forbids a protocol from supplying.

**Why they are not on the table now.** The appeal is push iteration's authoring
ergonomics with pull's consumption semantics, and that combination is the
expensive one, for a mechanical reason: push is cheap because the producer keeps
the stack, pull is cheap because the consumer keeps the stack, and a generator
wants both to keep theirs. Something has to hold a second one. Mesa's frames are
Rust stack frames, since `eval_expr` and `eval_block` recurse and unwind through
`Signal`, so suspension needs a thread per generator (absurd per `each`, and
§24.1 has no concurrency to build on), a stackful coroutine with its own stack
per loop (replacing §4.15 cl. 3's small state object), or an explicit
interpreter stack with frames as heap objects — an evaluator rewrite, not a
feature. CPython's generators are cheap because it already had heap frames and
an instruction pointer; my understanding is that Ruby, the nearer influence,
made `Enumerator#next` fiber-backed and kept the block as its primary idiom,
which is §13.4's push design. Mesa chose pull in §20.8 for unrelated reasons and
does not inherit Ruby's answer.

Two further costs worth having on record. **Cleanup is worse here than in
Python**: an abandoned generator holds a suspended frame that may hold a
resource, and Python needed generator `close()`, `throw()`, and finaliser
integration to make `try`/`finally` work inside one, where mesa has neither
`finally` nor destructors (§4.15 cl. 7). And **generators make single-pass
sources ordinary**, which sharpens a gap that already exists and that nothing
records: §4.15 cl. 1 splits collection from iterator so that a collection can be
iterated more than once, and a file is not a collection. Python's silent
empty-second-loop over a spent generator is the failure that shape invites.

**The revisit condition: if the evaluator ever moves to explicit heap frames for
another reason, generators become cheap and should be reconsidered as the
authoring form for iterators.** Until then they are deferred. §37.4 is
forward-compatible with them at no cost — the step verb plus slot is exactly a
resumption boundary, where yielding fills the slot and returning false ends the
loop — and the two-verb form of §37.5 is mildly forward-hostile, since a `done?`
would force the runtime to buffer permanently.

## 37.7 One thing here does not wait for `D7`

§4.15 cl. 6 says a borrow is taken and released inside the step call, so the
loop body runs with nothing held, and §20.8 lists that as one of two costs pull
iteration avoids. **The tree does not do this today**, and the gap is not
theoretical: `Expr::Each` matches `&*rf.borrow()` and holds the guard across
`eval_block` for the whole loop, so mutating the collection under iteration
panics the interpreter with `RefCell already borrowed` rather than raising
anything a program could see or `rescue`.

That is §6.2's hazard, still live, presenting as an interpreter crash where §5
wants a located error. **It needs no protocol to fix** — iterating by index, or
cloning the element and releasing the borrow around the body, converts it into
correct behaviour or a proper `rt::Error`. Recorded here because §20.8 credited
pull iteration with removing a hazard that is in fact removable now, and letting
the fix wait for `D7` would be paying a feature's schedule for a bug's cure.

## 37.8 What this closes

**`L3` closes.** The sentinel is not `Maybe`; a step is a `Bool` plus a slot on
the per-loop state object. `D7`'s remaining threads are naming only (`L1`,
`L2`).

**§18.5 is demoted, not deleted.** Opt-in optionals remain expressible and are
no longer proposed as the language's answer to absence. §18.4's truthiness rule
is the answer, and it is the only one.

**Nothing else in the document depended on `Maybe`.** Case types keep their
justification from §17's own examples and from `F2`, where a raised value is a
case-type instance. The one claim that falls is §20.8's economy argument, which
is marked there.

**One thread is opened and immediately deferred**: generators, under §37.6's
revisit condition. One gap is *named* without being closed: single-pass sources
against §4.15 cl. 1's re-iterable collections, which §37.6 raises and which has
no row anywhere.

# 38. Modules, files, and packages

§28 made a module a value and left three things unaddressed: where a module's
name comes from, how modules sit on disk, and whether they group into a larger
unit. This settles all three, and reopens §27 in exactly one place.

The shape in one paragraph: **a module is a file; a directory holds the children
of the module its sibling file declares; a package is a whole library and is the
unit of compilation, evaluation, and distribution; names are written in the
files rather than derived from them; and visibility exists only at the package
boundary.**

## 38.1 The package is the whole library, and it is the unit

A package is what a crate or a gem is — one library, one manifest, one
distributable thing. It is not a namespace layer inside a program's naming
scheme, and there is no unit between it and a module.

**It is the evaluation unit, and that buys one specific thing.** §33's resolver
phase widens from a file to a package, which splits §28.4's load-time/call-time
distinction cleanly in two:

- **Declarations** — `type`, `proto`, `def` — are resolved package-wide and are
  order-independent.
- **Top-level evaluated bindings** stay order-dependent and need a DAG, exactly
  as §28.4 says.

That refunds §28.4's concession that "mutually recursive types cannot span a
module boundary — they have to share one." They have to share a *package*, which
is a much weaker requirement and does not press §21.2's local types into service
as a workaround. This is the argument for the package being the evaluation unit;
the distribution story would be true of any grouping.

**Acyclicity is now one rule applied at two grains.** Modules form a DAG inside a
package, for §28.4's state-ordering reason. Packages form a DAG between
themselves, which separate compilation forces anyway. §28.4's "the graph is
module-granular" stops being a concession and becomes the accurate statement of
the inner grain.

**A package is evaluated exactly once**, which lifts §28.4's once-only rule from
the module to the unit that now carries it. Diamonds work; §28.1's identity
equality answers correctly for a package reached by two paths.

## 38.2 A module is a file, and a directory holds its children

```
json-rpc/
	package.toml
	src/
		package.ms          module JsonRpc
		codec.ms            module JsonRpc.Codec
		transport.ms        module JsonRpc.Transport
		codec/
			frame.ms        module JsonRpc.Codec.Frame
```

**Pairing rule: a directory holds the children of the module declared by its
sibling file.** `codec.ms` declares `JsonRpc.Codec`; `codec/` holds its children.
The precedent is Rust's 2018 scheme, which Rust moved *to* after living with
`mod.rs`.

**Every module has exactly one declaring file.** That is the property this shape
is chosen for. A scheme where directories declare themselves — a reserved
`module.ms` in every directory — has two kinds of declaring file with two sets of
rules; this has one kind, and a directory is only ever structure.

**Two layout invariants, checked at resolve time.** A directory with no sibling
file is an error, since its module name would be unrecoverable. And `src/` holds
exactly one file that no directory pairs with — `package.ms`, the root module —
because nothing can sit *beside* a package root. That is the single hole in the
pairing rule and the only exception in the scheme: `package.ms` is the one
declaring file that sits inside its own directory rather than beside it. It
earns the name by pairing with `package.toml`, which is the distribution head to
its module head.

**`src/` is a build root, not a module.** It has no name in the language and
never appears in a dotted path. It exists so the package directory is not also
a namespace, which is the arrangement where the package's name appears twice on
disk.

**The root module may have a body**, and is an ordinary module in every respect:
declarations, imports, top-level state, an export list. The cost of allowing it
is named rather than avoided — see §38.4's last paragraph.

**Containment is not a load-time edge.** A module value may hold children as
members without those children being evaluated; `Codec.decode(x)` inside a proc
body is a call-time reference, which §28.4's rule already covers. A parent
depends on a child only when the parent's *top-level* code names it. So a
parent/child cycle is an ordinary cycle, caught by the ordinary check, and no
rule against children importing parents is needed.

The hazard worth naming, because it is the first thing an author reaches for:
a root whose top level assembles something from its children, plus children that
import the root at load time for a shared helper, is a cycle. The fix is
§28.4's own — factor the shared piece into a third module — and it stays cheap
because §38.5 leaves everything inside a package reachable.

## 38.3 Names are declared, never derived

Files and directories are lowercase. Module names are capitalized, as types and
protocols are. **No mechanical mapping bridges the two: a module's name is
written in its declaring file, and on-disk names never enter the namespace at
all.**

```
module JsonRpc.Codec
```

**The declaration carries the full dotted path, and lowercase directories are
the reason.** With capitalized directories the prefix would be recoverable by
reading the path, and a leaf-only declaration would cost nothing. It is not
recoverable here: a reader who opens `src/codec.ms` and sees `module Codec`
cannot learn that the module is `JsonRpc.Codec` without walking up the tree and
opening each ancestor. That is a local-reasoning failure created by the
lowercase rule, and the full path is what pays for it.

**The redundancy is load-bearing rather than decorative.** Each file's prefix is
checked against the name its parent's declaring file declares — `frame.ms`'s
`JsonRpc.Codec` against `codec.ms`'s declaration, and that against
`package.ms`'s `JsonRpc`, which is checked against nothing because it is the
root and the sole authority for the package's namespace root. A misfiled or
misnamed module is a caught error at a known site.

**Two things fall out.** Initialism casing (`JSON` against `Json`) stops being a
mechanism question and becomes style, since nothing derives a name from a
filename. And two packages may declare the same root module name, discovered at
integration rather than at publication — handled by §38.6's dependency renaming,
which is where the collision was always going to be resolved.

**One thing is left open**: whether a declared name that diverges from its
filename should draw a warning. Nothing in the mechanism needs it —
`src/stuff.ms` declaring `module JsonRpc.Codec` checks out fine — but a reader
browsing the repository then cannot find a module by its name, which is the
visual-cohesion cost this scheme otherwise avoids. A lint is the right home for
it, since any rule strict enough to be an error is the case-mapping rule the
lowercase constraint exists to avoid; and `src/package.ms` would need a
permanent carve-out either way.

## 38.4 Nesting is a fact about names, not about lookup

**There is no lookup inheritance from a parent module to a child.** §10 rejected
letting the `outer` chain reach across modules "to keep `outer` meaning exactly
one thing," and §28.3 restated it when modules became values: qualified access
goes through member lookup, never through the scope chain. Being a parent
confers nothing that being a stranger would not.

**A file sees exactly three things**: the prelude, its own top-level
declarations, and what it imports. `$`-builtins are a fourth in mechanism but
not in kind, since §10 already made them bypass the scope chain entirely — which
is why `$print` and a module-provided `print` have deliberately different
shadowing rules.

**Membership and scope are different sets, and child modules are the one thing
that is a member without being in scope.** Inside `src/package.ms`:

```
export Codec          # works with no import — Codec is a member of JsonRpc
Codec.decode(text)    # unbound name without the import — Codec is not in scope
```

A module's own declared name is not in scope in itself either, so
`JsonRpc.Codec.decode(x)` is not an escape hatch from inside `package.ms`. That
keeps the three sources exact, with no self-import case to define.

**The member namespace is flat and shared.** A module's members are its own
top-level declarations *and* its child modules, in one namespace, which is why an
export list mixes kinds freely:

```
module JsonRpc
export call, Client, Codec, Transport

import JsonRpc.Codec

type Client(url, timeout: 30)

def call(url, method, params)
	Codec.decode(Http.post(url, Codec.encode(method, params)))
end
```

A declaration whose name matches a child module's — `def Codec` here — is a
duplicate-member error at resolve time. It fires essentially never, since procs
are lowercase and modules capitalized, and it exists because the namespace is
shared rather than because anyone expects to trip it. `def codec` collides with
nothing: the directory is named `codec/` on disk, but its module is `Codec`, and
the disk name is not in the namespace.

**§19.2's deferred question can now be answered.** That section left
`NameError` against `UnboundIdent` "to be settled with the module spec (§10),"
on the grounds that "names in a module" and "names in a scope" would acquire a
precise distinction. They have, and it is exactly the one above: a bare
identifier that resolves against nothing is a *scope* failure, and `JsonRpc.nope`
is a *member* failure, which §28.2's `MemberError` family already covers. The two
are different errors about different mechanisms, so the scope one may take the
narrower name. Recorded as now-decidable rather than decided, since §0 reads the
name as settled and §19.2 disagrees with it; that conflict is §1.5's to close.

## 38.5 Visibility exists, at exactly one boundary

§27 ruled that mesa has no visibility. **That is reversed in one place and
upheld everywhere else.**

**The form is a list at the head of a declaring file, never a modifier on a
declaration.**

```
module JsonRpc.Codec
export decode, encode, Request, Response
```

You never write `export` beside a `type`, a `def`, or a `proto`, and it never
reaches inside a type. §27's ruling that every type member is readable survives
untouched, and the principle it now rests on is sharper than "types are
special": **visibility governs the reachability of names; it never governs what
you can see of a value you can already name.**

**Omitting the list exports everything.** That is §20.7's argument reapplied —
the mechanism costs nothing to a package that never uses it, exactly as an
importable name costs nothing to a program that never names it. Rust took the
opposite default and pays ceremony on every declaration in every file forever. A
package with nothing to hide writes one line:

```
module JsonRpc
```

**The lists compose, and they have force at the package boundary only.** A name
is reachable from outside the package iff every segment on its path is exported
by its parent, up to `src/package.ms`. *Inside* a package the lists have no
force at all: §27 holds unchanged, every module is reachable from every other,
and there is no gradient.

**That siting is the whole of the reversal, and it is what makes it minimal.**
§27 was reasoned at file scale and remains right there — its arguments from
§20.4 and §3.4 are untouched. What changed is that a package is a *distribution*
boundary, where "everything is public" means a library ships its internals as
API and any refactor of any file is a breaking change to consumers. A mechanism
is warranted exactly where the reasoning changed and nowhere else.

**It also avoids the gradient that would otherwise be forced.** If export lists
bound siblings and children too, then a helper in `package.ms` used by both
`codec.ms` and `transport.ms` would have to be exported to be shared, and would
thereby become public — reopening the hole the feature exists to close, for its
most common case. Rust patches that with `pub(crate)` and `super::`; mesa does
not need either, because the boundary is drawn once and in the right place.

**Deep paths in an export list are declined.** `export Codec.decode` from the
root, lifting a name to the package's surface, is a re-export — a *binding* form
rather than a list — and it would let `JsonRpc.decode` name something declared
nowhere near the root, with two mechanisms jointly deciding its meaning. An
export list names its own members, at every level, and composition does the rest.
This is the one alternative here that costs something structural rather than
ergonomic.

**Exports gate names, not values.** A package may export a type whose fields hold
values of a dependency's types; a consumer holds and uses those values normally
and simply cannot write those types' names without taking the dependency. That
is the same principle as the type-member rule, applied across the boundary.

## 38.6 The manifest

```toml
name    = "json-rpc"
version = "1.2.0"
entry   = "Main"              # binary packages only

[deps]
http        = "0.4"
tls-backend = { version = "2.0", as = "Tls" }
```

**The manifest holds no language keys.** The root module's name and export list
live in `src/package.ms`, like every other module's, so the manifest is purely
distribution metadata and every module in the system is declared in a `.ms` file
without exception.

**It is data rather than mesa, and §28.4 is the reason.** A mesa manifest could
compute its dependency list, and §28.4 refused computed module names because a
reader cannot then tell where a name came from. The argument applies more
forcefully one level up, since the dependency graph must be knowable before any
mesa evaluates at all. TOML fits a lowercase, hyphenated, registry-shaped world;
the principle is the data-ness, not the syntax.

**`as` is the only aliasing mechanism in the language, and it is not in the
language.** A dependency is known locally by the root module name it declares,
or by `as` when that collides with the importing package's own root, with
another dependency, or simply reads badly. It is stated once, where the decision
to depend was already made. So §28.3 stays one dotted path binding its last
segment, with no `import ... as` form — and collisions *within* a package's own
tree are still resolved §28.3's way, by importing the parent and qualifying.

**Dependencies are package-wide.** Any module in a package may import any
declared dependency; nothing in the manifest gates that.

**The checks are static and cheap**: every dependency's local root name is a
valid module name, unique among dependencies, and distinct from the importing
package's own root; and an import naming another package fails if any segment on
its path is unexported. Whether that last one is a semantic-analysis obligation
in §35's set or a resolver check in §33's is left to whichever phase owns
cross-package name resolution; it is static either way.

## 38.7 Three keywords, and `import` (closes `L5`)

`module`, `export`, and `import` land together — the largest single spend since
§20.5 drew the budget, and worth stating as such.

§31.6 spells all three out. Each is written at the head of a file rather than in
its body, so all three are infrequent by the rule's own measure, and none is a
word an application wants for its own names.

**`L5` resolves to `import`.** The three are six characters each and read as one
family; abbreviating any one of them to save a keystroke on a line written at
most once or a handful of times per file would break the set for nothing. The
abbreviation rule predicted this outcome and the family makes it obvious.

## 38.8 What this closes

**`L5` closes.** The import keyword is `import`.

**§1.5's "whether modules group into a larger unit" closes.** They group into
packages, and the package is the unit of compilation, evaluation, and
distribution. §28.4's rule is restated at two grains in §38.1.

**§27 is partially reversed, and §27.3's "the import form now carries all the
weight" no longer holds.** There is an export form, it is a list rather than a
modifier, and it has force at the package boundary alone. Everything §27 says
about type members, about declaration modifiers, and about visibility inside a
package stands.

**§28.5's cost accounting is unchanged.** Modules-as-values still reaches
everything inside a package, so the mitigation §28.5 voided stays void; the
export list restores it only across the distribution boundary, which is not what
§28.5 was measuring.

**§20.7's and §34.8's scaffolding can now expire.** The built-in protocol names
and their provided definitions were given a prelude binding "until §28's modules
exist," because `impl Eq` cannot require an import from a module system that is
not built. The system is now specified, so the expiry is unblocked and the
permanently-spent prelude count stays at nine.

**§19.2's naming question becomes decidable**, on the distinction §38.4 draws.
The conflict between §0 and §19.2 is §1.5's to close, not this section's.

**`L8` is untouched.** Whether `Module` sits in the prelude is the same question
it was; nothing here needs the type named, and a package does not become a value
— `Package` is not a type and packages are not in the namespace. *(Closed by
§42.6, and not by anything this section needed: what settles it is the
membership rule the expiry unblocked two paragraphs above, once §42.6 says what
the tier is left holding.)*

**One thing is opened**: §38.3's filename-correspondence lint, which has no row
anywhere and is a diagnostics question rather than a language one.

# 39. The protocol names (`L1`)

§32.3 declined to name any of the nine and sent them to §0.4 as `L1`, the last
naming thread carrying more than a spelling. This settles all nine, the two
printing verbs, and the subscript protocol's error name, which §32.3 said was
not a separate decision and moved with the protocol.

The names came out of a rule, which is the only reason to record the reasoning
rather than just the list.

## 39.1 The rule: a protocol is named for the operation it supports

**`impl X` reads "supports being X'd."** The name is a verb, and the
implementing type is that verb's *object*, not its subject.

The rule was found rather than chosen, and it accounts for every protocol name
this document had already arrived at independently: `impl Call` is "can be
called", `impl Append` is "can be appended to", `impl Order` is "can be
ordered". It also explains after the fact why the placeholders that read badly
read badly — which is the test §31.6 applied to itself, and the reason to trust
it as operative rather than as a rationalisation.

Three ways a candidate fails it:

- **A noun** names what the type *is* rather than what it supports. A reader
  meeting `type Collection impl Item` asks whether the collection is itself an
  item. This is what disqualified `Subscript`, which was otherwise fine, and
  `Item`, which was briefly its replacement.
- **A verb with the wrong object** is a subtler form of the same error, and it
  is what disqualifies `Debug`. You do not debug a *value*; you debug a program
  while looking at one. The verb is real, but its subject is the programmer and
  its object is the code, and the type is neither. Rust's `Debug` has the same
  wart and is where the placeholder came from.
- **A truncation** fails §31.6 rather than this rule, and it is why `Eq` and
  `Iter` do not survive. §31.6 abbreviates the *frequent*; a protocol name
  appears about once per type declaration, so nothing in this set earns one.

**`Equal` is the acknowledged exception**, worth stating rather than hiding.
Equality's English verb is homographic with its adjective, so no form passes
cleanly: `Equal` reads as an adjective, and the verb-shaped alternatives are
either stilted (`Equate`) or already spoken for (`Compare`, by `Order`). It is
taken on §31.6's grounds instead — at five characters it sits with the rest of
the set, where `Eq` would have been the only truncation left standing and would
have read as the odd one out *because* it was short. Length consistency is doing
the work the part-of-speech rule could not.

## 39.2 The nine

| What it governs | Name | Verb | Note |
| --- | --- | --- | --- |
| Equality | `Equal` | — | §39.1's stated exception: an adjective, taken on length |
| Hashing | `Hash` | — | A whole word at four characters, not a truncation, so §31.6 leaves it alone |
| Ordering | `Order` | `compare` | Verb already settled (§32.1, §34.1); the operators wire onto it |
| Subscript | `Access` | — | Two halves, verbs unnamed; `get` and `has` are provided members per §32.1 |
| Append | `Append` | `push` | Verb already settled (§31.4) |
| Collection → iterator | `Iterate` | — | |
| Iterator → step | `Advance` | — | The step method stays `L2`; see §39.4 |
| Display | `Display` | `display` | |
| Debug | `Inspect` | `inspect` | Renamed from the `Debug` placeholder, which fails §39.1 |

**`Access` is the document's own word for the concept.** *Structure and access,
yes; algebra, no* has been the phrase for where the protocol line sits since
§13, and it is neutral between a position and a key in the way `Index` is not —
§23.1 having already ruled that dict keys are keys rather than indexes, an
`Index` protocol would have made `Dict` an exception at the one place the error
taxonomy says it is not one.

**The subscript protocol's error name resolves to nothing.** §20.2 made the
declined half raise the general `ProtocolError.NotImplemented(val, name)`, so
there is no subscript-specific variant left to name.

*Amended in the tree.* `rt.rs`'s `NotScriptable` was renamed to
**`NotAccessible`** rather than left in place to be superseded later. Two
reasons, neither anticipated above. It is still constructed today, on a path the
protocol does not yet exist to replace, so "superseded" describes a future the
code does not live in yet. And its message joins an established family —
`is not callable`, `is not iterable`, `is not orderable` — in which a `Script`
name was the one member named for notation rather than for the operation, which
is §39.6's complaint arriving in the error taxonomy. It remains scheduled for
supersession when `Access` lands; the rename buys consistency in the interim
rather than a permanent name.

**Two protocols name their own verb** — `Display`/`display` and
`Inspect`/`inspect` — and that is a property of those two, not a rule. Where a
verb was already settled for its own reasons it stands. The shape being avoided
is Rust's `Display`/`fmt`, where the protocol name tells you nothing about the
member you have to write.

## 39.3 What this costs

**`Show` was the leading candidate for the display protocol and was declined.**
Haskell's `Show` is the round-trippable form — closer to this document's
`Inspect` than to its `Display` — and §25.1 has already declined the round-trip
guarantee that makes Haskell's `Show` what it is. Taking it would have meant a
name that reads backwards to one audience, and required §25.1 to say so.
`Display` avoids the inversion at no cost once the verb moves with the protocol,
which is what keeps the protocol naming its own member.

**`Iterate` and `Advance` carry the least evidence**, being the only two names
with no prior use in the document. Everything else either survived from an
existing placeholder (`Hash`, `Order`, `Append`, `Display`) or was already the
document's word for the concept (`Access`).

## 39.4 `L2` is narrowed, not closed

The iterator's step method is still `L2`, and §39.2 leaves its verb blank
deliberately. Naming the protocol `Advance` bears on it in a way the row did not
anticipate: applying the printing pair's protocol-names-its-verb shape here would
make the method `advance` and close `L2` as a side effect. It should not. §24.5
refunded `next` specifically so it could be taken, and §0.4 records that taking
it is not automatic. The two names are compatible either way — `proto Advance`
requiring `def next` is well-formed — so nothing about `L2` changes except that
it now has a second candidate, which arrived for free.

## 39.5 What closing this releases

§32.4's five rows were waiting on a protocol being *named* rather than on any
open question — pull iteration, `<<`, `Order` for `Str` and `Char`, `Str`
iterability, and the printing pair. They now wait on nothing.

`L1` closes. Of §0.4's naming threads, `L2`, interpolation's spelling, and the
import keyword remain, and none of the three blocks a row.

## 39.6 The AST node takes the protocol's name

`Expr::Script` becomes **`Expr::Access`**, and `Place::Script` follows it. This
is a separate decision from naming the protocol, and it was very nearly made
wrongly. The obvious repair for `Script` is `Subscript` — it fixes the *reading*,
since `Script` collides with *script file* in a repo whose input files are
scripts — and it leaves the actual defect untouched.

The defect is only visible against the node's siblings. Every other `Expr`
variant names either **the keyword that introduces it** (`Each`, `When`,
`Return`, `Self_`) or **the operation or thing it denotes** (`Call`, `Member`,
`Assign`, `Binary`, `Name`, `Lit`). `Script` alone names the *notation*, and
`Subscript` would have kept it there — a better spelling of the same category
error. Naming it for the protocol that governs it puts one word across all three
layers: `Expr::Access`, `Precedence::ACCESS`, `impl Access`.

**`Member` sitting beside it is the one thing a reader will query** — surely
member lookup is access too? In English yes; in mesa no. §32.2 places member
lookup among the things that are deliberately *not* protocols, so `Access`
governs `a[k]` and nothing else, and the pair marks a real line rather than
blurring one. But it is a **learned** distinction rather than a self-evident one,
and that is this name's honest cost, in the same way §39.1's `Equal` carries
its own.

*This is a note about the implementation rather than the language*, recorded
here because §39's rule is what decided it and because the near-miss is the
useful part: a syntax-level name and a protocol-level name answer to different
rules, and `Subscript` passes neither.


---

# 40. Significant newlines look ahead too (`B1`, amended)

§15.3 took a deliberate position: a newline ends an expression only by
looking at the token *before* it, and looking ahead as well "is more
forgiving and meaningfully more complex." That was true of Ruby's rule,
which looks ahead unconditionally. It is not true of a rule that only looks
ahead for tokens that cannot also start an expression — and most operators
are exactly that.

## 40.1 What was one-sided about §15.3

The guard lives in `parse_expr_prec`'s postfix/infix loop and, until now,
consulted only `terminates_expr(prev_tag)`. That asymmetry had a consequence
nobody had named: **trailing continuation already worked.** `total := a +`
newline `b` parses fine, because when the loop is sitting on `+` there is no
newline *before* `+` — the newline is before `b`, and `terminates_expr` is
never asked about `+`. Only *leading* continuation — `total := a` newline
`+ b` — was rejected, and only because nothing checked the far side of the
newline at all.

So "break after the operator, never before" was not a symmetric house style
being chosen over Ruby's; it was the accidental shape of a rule that only
ever looked backward. Once you ask what looking forward would cost, the
answer turns out to depend entirely on which tokens you'd be looking for.

## 40.2 The three-way split

Extend the guard with one more conjunct:

```rust
if nl_before(cur)
	&& terminates_expr(tag(prev))
	&& !continues_expr(tag(cur))
{
	break;
}
```

`continues_expr` is a second `TokenTag` → property table, `terminates_expr`'s
shape exactly. The question is which tokens belong in it, and the answer
splits the operators into three classes rather than two:

- **Terminates** — unchanged from §15.3: `Ident`, `Str`, `Char`, `Num`,
  `Bool`, `Nil`, `)`, `]`, `}`, `end`.
- **Continues** — infix-only tokens: `or`, `and`, `==`, `!=`, `<`, `>`, `<=`,
  `>=`, `+`, `*`, `/`, `.`. Twelve tags, all of them precedence-bearing, none
  of them appearing anywhere in `parse_expr_unit`.
- **Neither** — `-`, `[`, `(`. These end nothing and continue nothing; a line
  starting with one is, as before, a fresh statement.

The old prose's "everything else continues" collapses under this split — it
was only ever true because nothing in the terminates set overlapped with a
prefix-capable token. Once the forward direction is added, that stops being
true by construction: the "neither" class exists precisely because those
three tokens *are* prefix-capable, and admitting them would mean a newline
silently choosing which of two grammatical jobs a token is doing.

`:=` is infix-only and therefore safe to admit on the same grounds as the
twelve, but it is left out on taste: `x` newline `:= 5` has no use this
document can find, and reads as a typo more often than as an intentional
continuation.

## 40.3 Why this is cheap where Ruby's version isn't

§15.3's cost estimate was about *unconditional* lookahead — the token after
the newline decides regardless of what it is, which is Ruby's rule and which
does need real disambiguation machinery (Ruby's parser famously special-cases
several of these). Restricting the forward-looking set to tokens with no
prefix use sidesteps that machinery rather than building it: every one of
the twelve is, today, a syntax error in expression-starting position
(`parse_expr_unit`'s match has no arm for any of them). A newline followed by
one of them was unparseable before this change and continues an expression
after it — the accepted-program set can only grow, so no fixture or existing
script can change meaning. `tests/newlines.ms` and `main.ms` were checked and
confirm it (§15.5, amended below).

The three excluded tokens are excluded for the opposite reason: each is
already meaningful in expression-starting position, so a newline in front of
one is genuinely ambiguous between "continue" and "begin a new statement
using this token's other job."

- **`-`** is unary negation (§8, arithmetic). Admitting it would make
  `total := a` newline `- b` silently mean subtraction instead of two
  statements, the second of which negates `b` and discards the result. That
  second reading is what the rule now protects — silently, since it's not an
  error, just a statement whose value nobody asked for. Worth a real fixture
  precisely because it doesn't fail loudly.
- **`[`** is list-literal syntax as well as index-postfix. `tests/newlines.ms`
  already pins the case that would flip: `x := y` newline `[1]` must stay two
  statements (`y` isn't indexed; `[1]` is a fresh list literal), not become
  `y[1]`.
- **`(`** is call-postfix today and becomes grouping-prefix once `B2` lands —
  this is §14.8 clause 6's ambiguity arriving from the newline side instead
  of the precedence side, and it's why the `B1 → B2` edge (§0.2) now carries
  a second reason: `B2` is exactly what forecloses `(` from ever joining the
  continues set.

## 40.4 The one asymmetry worth flagging explicitly

`+` continues; `-` does not. That looks inconsistent until you notice it
tracks a real fact about mesa rather than an arbitrary choice: mesa has no
unary `+` (§8 gives `Num` only unary `-`), so `+` at the start of a line has
exactly one grammatical reading and `-` has two. The rule isn't treating the
two arithmetic operators differently on a whim — it's reporting that they
aren't actually symmetric in this grammar.

`<<` (`B6`, unbuilt) is infix-only exactly like the twelve and joins
`continues_expr` when it lands.

## 40.5 `return` keeps its own rule, deliberately unreconciled

§15.4's special case is untouched: `parse_return_expr` tests `nl_before`
before any left operand exists, so the infix loop this section amends never
runs for it. A bare `return` still terminates at a newline regardless of
what follows — `return` newline `.foo` does not turn into `return
(receiver).foo`, because there is no receiver yet for `.foo` to attach to.
The two rules now answer "does a newline stop me" differently by design: one
is a leftward look at what a value can follow, the other is a hazard-avoidance
special case with no leftward operand to consult at all. They were already
inconsistent under §15.3 (`return` terminates despite not being in the
terminates set); this section doesn't change that, just widens the rule
`return` is the exception to.

---

# 41. The carrier follows the error type (§35.7 reversed, and `L10`)

§35 named the phase, gave it an error type, and left the carrier free: "a
carrier is an implementation choice wherever it is complete." Then the first
check was built, and it did not go where that freedom pointed. `C2`'s prelude
prohibition could have lived in `eval_decl` — §35.2 marks it complete there for
declarations, and §4.7 cl. 13 says outright to build it there first — and it
went into a new module instead.

§4.7 cl. 13's amendment records the reason as cost: "building the minimal pass
cost less than the second error channel `eval_decl` would have needed." That is
true and it undersells itself. The second error channel was not expensive; it
was *wrong*, and the cheapness of avoiding it is a consequence rather than the
argument. This section states the rule that was actually applied, which reverses
§35.7's, and then spends it on `L10`.

## 41.1 What is built

*Checked against the source rather than inferred.* `src/sem.rs` is 125 lines.
`sem::check(&mut Interner, &Chunk, &Source) -> Result<(), Error>` runs at
`main.rs:31`, between `Parser` and `Interpreter::new`, and returns on the first
error exactly as the parser does. `sem::Error` has two variants, each carrying a
`Location`, with clause 5's shape and a `Display` producing
`file:line,col: semantic error: …`.

What it traverses is the part worth stating precisely, because it is smaller
than "the pass" suggests:

    chunk.top → ModuleItem::Type   → TypeItem::Type (recursive)
                                   → TypeItem::Method → param list
              → ModuleItem::Def    → param list
              → ModuleItem::Expr   → skipped

It never reaches a `Block` or an `Expr`. **What exists is a declaration walker**,
which is §33.2's deflated set built as a module rather than left in `eval_decl` —
the same shape the deflation described, at a different address. §33's *pass*, the
thing that walks bodies, does not exist yet.

Two consequences follow from the last line of that diagram and are already
visible in the code. `ModuleItem::Expr(_) => continue` is where a module-level
proc literal will sit, so `B7` opens §4.7 cl. 13's second gap the moment it
lands. And the prelude check reads only the *names* of top-level items, so
`Str := 5` inside a body is unreached — cl. 13's first gap, present today.

## 41.2 The rule

> A check lives in the module its error type names.

Not "wherever it is complete." The error type is the phase's, so raising it from
`rt` or `syn` puts one phase's vocabulary in another phase's module, and the
module boundary is the only thing keeping the three phases from becoming a
gradient. §35.7 saw the same hazard from one side — it worried about a
non-rescuable failure entering a rescuable enum — and answered by letting the
error travel to the check. The answer that was actually built moves the check to
the error, and it generalises where §35.7's does not: `syn` raising `sem::Error`
is the identical violation with none of §19's rescue argument attached to it, so
§35.7's reasoning cannot reach it while this rule can.

**§4.7 cl. 12 survives, narrowed.** A carrier is still an implementation choice —
among carriers *inside the phase*. What it may no longer do is choose a carrier
in another module. That leaves `sem` and the loader, and §35.1's four-carrier
table loses two entries: the parser (which was only ever holding `L10`, resolved
below) and `eval_decl`.

## 41.3 The deflation mostly evaporates, and this is the cost

§33.2 argued the honest number of forcing functions was one, and §33.7 priced
the deflated seven as able to "sit in `eval_decl` until then and move for a few
lines each." Under §41.2 they cannot sit there at all, because they would have
to raise `sem::Error` from `rt`. So the thing forcing `sem` into existence is
not §17.6's coverage check but *the first obligation anyone implements* — which
is precisely what happened, a year of argument about coverage notwithstanding.
`C2` forced it.

This is a real cost and it should not be read as a tidy-up. §33 spent its length
establishing that the pass could be deferred, and the rule above removes the
deferral. Three things keep the cost small, and they are the reason to accept it:

- **The pass is separable from its outputs.** `sem::check` produces nothing but
  diagnostics. Name-to-slot resolution changes the signature; every pure check
  added before then does not.
- **`eval_decl` never pays for two channels.** §35.7 accepted that it would
  carry two "for a while," and a while in this document has a way of becoming
  permanent — obligations 6, 7 and 8 have no forcing function pushing them out.
- **§33.4 already conceded most of it.** "The cost was mostly committed
  already." What §41.2 changes is when the commitment comes due, not its size.

## 41.4 `L10` closes here

`break` crossing a proc boundary is **statically rejected, by a lexical
loop-depth counter over the body walk, reset at every proc body, raising
`sem::Error`.** The same counter rejects `break` outside any loop, which is not a
second rule but the depth-zero case of this one.

**The semantic half.** §0.4 records the depth argument as weakened by §30's
removal of §16.2's escape prohibition. It is strengthened, and the row has it
backwards. Under §16.2 a literal could not outlive its frame, so a `break`
crossing into one always had a live enclosing loop to unwind to: refusable, but
meaningful. Under §30 a stored closure may be called with no such frame in
existence, so the construct has no meaning to refuse. §30 moves the prohibition
from a choice to a necessity.

What §30 *did* weaken is narrower and worth naming, because it is the part the
row was groping at. Pre-§30, lexical and dynamic nesting could not diverge, so a
lexical refusal rejected exactly the set with no meaning and cost nothing.
Post-§30 the lexical rule over-approximates: it also rejects the *immediate*
callback, invoked during the call it was passed to, where the frame is live and
a dynamic `break` would have worked. The cost went from zero to Ruby's
break-from-block.

**That population is empty, and §20.8 is why.** Ruby needs break-from-block
because iteration is push — `xs.each { … break … }` is the loop. Mesa's
`each n in xs do … end` is surface syntax over a pull protocol, so its body is a
block in the enclosing frame and its `break` is lexically inside its own loop,
untouched by this rule. §20.8's early-exit story for the derived library is
"stop calling the iterator," which is a `return` inside `find` rather than a
`break` through a user callback — the section says so while arguing *for* pull,
and read the other way it is pull paying this section's bill in advance. The
residue is hand-written higher-order code, `xs.map(def (n) … break … end)`, where
the loop being exited belongs to `map`.

**The carrier half never depended on §16.2.** Two arguments got fused in §0.4's
caveat: whether the prohibition should exist (semantic, and §30 settles it) and
where the check lives (§41.2 settles it, and it has nothing to say about escape).
The check asks about neither a declaration nor a value, only about token nesting,
so §33.6's admission rule admits it trivially and §35.4's clause 14 extension
does not reach it.

**Runtime is excluded, not merely inferior.** Depth is determinable from source,
so §33.3's property requires it before the first side effect. Ruby's
`LocalJumpError` is the shape being declined, and it is what a dynamic carrier
costs: a runtime error class reporting a condition the source already fixed.

**And the precedent runs with it.** Languages with one static error bucket call
this a syntax error because they have nowhere else to put it — JavaScript's
early errors and Python's `SyntaxError: 'break' outside loop` are both that.
Languages that separate parsing from semantic analysis file it on the semantic
side: C makes it a *constraint* violation rather than a syntax rule and Clang
raises it from Sema, and Rust has a dedicated pass with `E0268` for the
loop case and **`E0267` for the closure case** — the exact rule here, given its
own error code, in a language whose closures capture by value and escape freely.
Mesa has three error types, so it is in the second camp by construction.

## 41.5 It is the first check that walks a body

This is the implementation fact that matters more than the rule. `check_def`
currently receives `Def(_, params, _)` and discards the body. `break` forces the
first exhaustive match over `Expr`'s fourteen variants outside `syn.rs` and
`rt.rs` — mostly one-line recursion, but a surface every future variant must be
added to.

Paid once, and three obligations are waiting behind it:

- **§4.7 cl. 13's two gaps close.** The prelude prohibition reaches in-body
  bindings and the parameter rule reaches proc literals only from a body walk.
  Both are recorded as partial; this is what completes them, and cl. 13's "moving
  them into the pass changes program validity" is the same event seen from the
  other side.
- **`C7`'s coverage check** becomes arms on an existing traversal rather than a
  new one. §33.2 called it the one forcing function; if `L10` lands first it
  arrives to find the traversal built.
- **`C6`'s capture sets** need the identical barrier. A proc body is where free-
  variable collection stops treating enclosing locals as ordinary, and it is
  where loop depth resets. The two obligations are one question asked twice —
  *what does a proc body sever?* — one about values, one about control flow.

## 41.6 Traversal is not environment

Worth separating, because `C0`'s row currently reads as though one thing is
missing when it is two. `B15`'s rightward-default check and `B13`'s body-field
check both stalled on the same wall: they need to know what a name resolves to,
and "a naive walk rejects valid programs" because a default referencing a
module-level `g` is legal while one referencing a later parameter is not.

`break` needs no environment at all. It needs a traversal and one integer. So it
does not unblock those two — it builds the skeleton they hang on and leaves the
scope chain to `C1`. Stating it this way keeps the row honest: `L10` landing does
not make `B15` cl. 5 cheap, it makes it *reachable*.

## 41.7 The loader shares the module, not the traversal

§35.2 marks the import graph "never" forced into the pass, and §41.2 might look
like it forces it anyway. It does not, because `sem` is the phase's module rather
than the pass's — which is already true of what is built, since a prelude check
is not resolution. The loader is a second entry point beside `check`, with its
own signature: it runs before any `Chunk` exists, over paths and files, and it
*produces* a module map rather than returning `Result<(), Error>`. Same module,
same error type, same §33.3 property, no shared traversal. That is §35.1's claim
about what unifies carriers, holding in the one case that looked like a
counterexample.

## 41.8 What this leaves

**`sem::check` bails on the first error.** Fine for a declaration walk over one
file; a body walk over a package is where users start expecting more than one
diagnostic per run, and `Vec<Error>` is a signature change worth making once
rather than twice. Not forced now, and named so it is not discovered twice.

**The obligation count was never nine.** §4.6 cl. 12 — required parameters
precede defaulted ones — is built, in `sem.rs`, and appears on no obligation list
in this document. §35.2's table is a snapshot of what had been argued about, not
an enumeration of what the phase does, and it should stop being read as one.

**Where `each`'s desugaring goes** is untouched (§35.8), and now has a second
reason to be decided: a desugaring pass and a body-walking checker are the same
traversal, so the question of whether they are the same *pass* becomes live as
soon as this one exists.

---

# 42. The loader, the prelude's arc, and where the stdlib lives

§38 settled what a package is and how modules sit on disk. §36 settled how a
protocol declared in mesa acquires an identity Rust can hold. Neither says how
the two meet, and the join decides three things this document has carried as
open or as scaffolding: how much of the module system is a *pass* at all, what
the prelude tier is for once §34.8's bindings expire, and whether the stdlib can
ever be files a reader can open.

The shape in one paragraph: **the loader is eager over a whole package and
cannot be otherwise; what it produces is a module map, which is both the
resolver's environment and the seam's lookup structure; the prelude tier holds
the native types and nothing else, permanently, so everything §34.8 puts there
is scaffolding that expires into an ordinary package; and that package may be
embedded in the binary or read from disk, because nothing above the loader's
first step can tell the difference.**

Nothing here reverses §38 or §36. Two things are closed — `L8`, and §4.18
cl. 31's open carrier — and one recommendation is made against the fixture
scheme and left to the roadmap's §1.5 rather than settled here.

## 42.1 Two mechanisms share the phase, and only one has a traversal

§41.7 says this and it is worth stating where a reader looking for "the module
resolver" will find it, because that phrase names two things with different
inputs, different outputs, and no shared code:

| | The loader | The resolver (`sem::check`) |
| --- | --- | --- |
| Runs | before any `Chunk` exists | over the package's `Chunk`s |
| Reads | `package.toml`, paths, files, headers | declarations, and under `L10` bodies |
| Produces | a module map | `Result<(), Vec<Error>>` and §33's side tables |
| Shares | module `sem`, `sem::Error`, §33.3's property, §33.6's admission rule | the same four |
| Shares not | the traversal | — |

**Name resolution splits three ways across them**, and only the third is the
pass's:

1. **Path to file.** The loader, from the `module` headers it reads. Failures:
   an import naming nothing, a misfiled module, a cycle.
2. **Declared name to module, and a module's member set.** The loader, building
   §38.3's namespace tree from the dotted declarations and checking it against
   §38.2's on-disk tree. Failures: prefix mismatch, duplicate member.
3. **Identifier to binding.** The pass, extending §14.5's order. `G1` **adds no
   tier**: an import binds a name at `Tier::Module` in exactly one file's scope,
   and that is the whole of its effect on lookup.

That third line is the deflation worth noticing. The module system is a large
specification and almost none of it lands in the scope chain `C1` already built.

## 42.2 The loader cannot be lazy, and §38.3 is why

§38.3's names-are-declared rule has a consequence it does not draw:

> `import JsonRpc.Codec` carries no information about which file to open.

Rust resolves `mod codec;` by opening `codec.rs`, because the name derives from
the path. §38.3 removes exactly that bridge — files are lowercase, module names
are capitalized, and no mechanical mapping joins them — so the only way to learn
that `src/codec.ms` declares `JsonRpc.Codec` is to read `src/codec.ms`.

**Three requirements force the same conclusion independently**, which is why it
is a structural fact rather than an implementation preference:

- **Import resolution.** The path-to-file map exists only after every header is
  read, per the above.
- **§38.2's layout invariants.** "A directory with no sibling file is an error"
  is a claim about the whole tree, not about any file reached from an import.
- **§38.1's order-independent declarations.** The resolver's environment is a
  union over every module in the package, so every module must be parsed before
  any body can be checked against it.

So the loader's shape is fixed: walk `src/`, read every `.ms`, lex and parse
each, extract the header, run §38's four families of static check, build the
map, topologically sort the import edges. Nothing is opened on demand and
nothing is skipped because no one imported it.

**Two things follow.** The first is a cost, and it is the honest price of
§38.3: compile time is linear in package size with no way to avoid it, which is
the commitment §38.1 already made by choosing the package as the evaluation
unit. It is named here so it is not mistaken later for an implementation defect.
The second is a simplification: since every file is parsed in full regardless,
there is no reason to build a header-only parse mode. One parse per file, a
complete `Chunk`, and the header read off the front of `chunk.top`.

## 42.3 Imports gate scope; the DAG gates evaluation order

§38.1's split reads, on a quick pass, as though imports had stopped mattering
for visibility. They have not, and the distinction is worth one paragraph
because getting it backwards produces a language nobody specified.

- **Declarations are order-independent** (§38.1). A `type` may name a `type`
  declared in a module that is evaluated later, or in the same file further
  down. The import DAG does not order declarations at all.
- **Imports still gate scope** (§4.18 cl. 22). A file sees the prelude, its
  own top-level declarations, and what it imports. Order-independence buys you
  the right to *name a thing not yet evaluated*; it does not buy you the right
  to name a thing you did not import.

Which leaves the import graph with two jobs and no third: detecting cycles, and
sequencing top-level evaluated bindings. It is not a visibility structure, and
§28.4's rule survives as a statement about state rather than about names.

## 42.4 A module's members are not its scope

§4.18 cl. 23 says membership and scope are different sets and gives child
modules as the case — a member that is not in scope. The other direction is the
one that decides a data structure:

- **Scope** = the module's own top-level declarations, plus its imports, plus
  the prelude above it.
- **Members** = the module's own top-level declarations, plus its child modules.

Neither contains the other, so `Obj::Module` **cannot be a handle to the
module's `Scope`**. If it were, an `import Utils` at the head of `codec.ms`
would make `Codec.Utils` readable from outside — an accidental re-export, which
§4.18 cl. 27 declines by name when it is written deliberately.

**The separate map is also the arrangement with fewer conditionals**, which is
the reason to prefer it beyond correctness. The loader builds one
`HashMap<Sym, Val>` per module, once, at load; member lookup is then `D2`'s
existing machinery over a plain map, the same shape `Obj::Instance` already
uses. The alternative tags every `Local` with declaration-or-import and asks
that question on every member read — a predicate evaluated forever, at the exact
point `C1` finished reducing lookup to three tiers. The overlap between the two
sets is not duplication either: both hold `Val`s cloning the same `Rc`s, as
`Proc.scope` already does.

## 42.5 Identity is not scope residency

§36.1 fixes a protocol's identity at startup and holds it in one table; §36.1's
last paragraph makes that table the mirror of `CORE_TYPES`. Reading those
together, it is easy to conclude that the built-in protocols must live in the
prelude — that the seam needs their names resolvable before any user file has
imported anything. **It does not**, and the distinction is what makes §42.6
possible.

The seam has two requirements and they are separable:

- **Evaluated at boot** — yes, unconditionally. The declarations must run before
  user code so that §4.19 cl. 1 has something to assign an identity to.
- **In scope at boot** — no. The table holds identities, and §36.1's dispatch
  rule asks *does this type conform to this protocol*. No operator arm consults
  the scope chain, and §36.1's argument against magic names is precisely that
  the Rust side holds an identity rather than a verb's `Sym`.

**So the language-visible rule is: you import a protocol to write `impl Order`,
never to use `<`.** A conforming type acquires the operator because the arm
holds the identity; a program that writes no `impl` imports no protocols. The
precedent is exact — Rust imports `std::ops::Add` to implement it and never to
use `+` — and §36.1's opt-in-by-magic-name prohibition is untouched, because
conformance is still the question asked.

The mechanical change is one table key. §36.1's structure stops being keyed by a
bare `Sym` and becomes keyed by the declared dotted path, resolved once against
§42.1's module map. That is a lookup in a structure the loader already produces,
not a new mechanism, and it is the change §36.1 chose a single table to make
cheap.

**This also removes an apparent collision with §4.19 cl. 12's boot order.**
The order stands exactly as written — `CORE_TYPES` registration, the loader, the
seam, the user's module — because the stdlib package being *loaded and
evaluated* at step two is orthogonal to any user file *importing from it* at
step four. §4.19 cl. 13's declarations-only invariant is what keeps step two
safe, and it goes on doing that job unchanged.

## 42.6 The prelude's arc, and `L8` closes

§38.8 unblocked §20.7's and §34.8's expiry and left the destination unstated.
Here it is: **the prelude tier holds the native types and nothing else.**

Those cannot be imported away. They are registered in Rust at fixed ids before
any source is read, they have no declaring file, and `x is List` has to name
something. Everything else the prelude has ever been asked to hold — §34.8's
protocol names, the derived library's provided members — is scaffolding with a
known expiry, and after `G1` it lives in an ordinary package behind an ordinary
`import`.

Which gives the tier a shape over time, worth writing down so the middle is not
mistaken for the design:

| Stage | The prelude holds | §20.6's check protects |
| --- | --- | --- |
| Today | `CORE_TYPE_NAMES` | nine names |
| Under `H1` | plus the protocols and the derived members | the whole stdlib namespace |
| After `G1` | `CORE_TYPE_NAMES`, `Module` now among them | ten names |

**The tier balloons and contracts back**, and `H1`'s row prices the ballooning
as a compatibility commitment it does not have to be: the interim *rejects*
programs that bind `map`, and the endpoint *accepts* them. The change runs in
the loosening direction, so nothing written against the interim breaks at the
expiry. Running the other way would have been a reason to skip the interim
entirely.

**`L8` resolves to the prelude**, and by a rule rather than a budget argument,
which is the shape §39.1 found for `L1`. Once the tier's membership rule is *the
native types*, `Module` is in it by construction: it is registered in Rust like
`Type` and `Proc`, it has no mesa declaration, and §36.2 is explicit that a
native type has nowhere to put one. The alternative — an ordinary importable
name in a built-in module, as the row frames it — would need a mesa binding
standing in for a Rust-registered type, splitting the native type names across
two mechanisms for one member of the set. That is the same trade §31.6 refuses
elsewhere: breaking a set to save something the set was not costing. The count
is ten rather than the row's tenth-against-nine only because `Char` landed since
§38.8 wrote nine.

`Package` remains not a type and packages remain outside the namespace (§38.1),
so nothing here widens what the answer covers.

## 42.7 The stdlib may live on disk, and one thing gets cheaper

`H1` specifies embedded source because §34.8 needed the library to exist before
the module system did. Once `G1` lands, the stdlib is a package like any other,
and where its bytes come from stops being a language question.

**Files on disk are the better long-run answer**, and the argument is §34.11's
own. A derived library written in mesa pays for itself in readability; a library
nobody can open is paying that price without collecting. Two consequences:

- **`H1`'s synthetic source identity requirement disappears.** Its row lists
  three things the loader needs beyond itself, and the first is a fabricated
  `Source` because "stdlib code has no user file to name." On disk it has a
  file, a path, and an honest `Location`. `A2`'s traceback through stdlib frames
  stops being a presentation decision and becomes ordinary frames.
- **The mechanism is already specified.** A package at a known root, taken as an
  implicit dependency of every package in the sense Rust's `std` is. §38.6's
  rule — a dependency is known by the root module name it declares (§4.18
  cl. 30) — applies unchanged; *implicit* means only that it is not written in
  `[deps]`.

**The cost is §42.2's, and it is the one thing that could make this a mistake.**
The loader cannot lazily open `core/order.ms` when it sees `import Core.Order`,
so every process start parses the entire stdlib package. §36.7 measures roughly
0.4 µs per line, which puts a 2,000-line stdlib under a millisecond and a
20,000-line one near ten. That is a *parse* cost; reading the files is the cheap
half. If it ever binds, the answer is a cached pre-parsed module map, which is a
project rather than a tweak and is not designed here.

**Neither move should be built for in advance.** A source-provider abstraction
with one implementor is machinery standing in for a single call site: `H1`
takes a `&str` of source, and the line producing it changes from `include_str!`
to a file read when the time comes. Likewise the seam's table key stays a `Sym`
until a path exists to key it by. Both are mechanical widenings of one place,
which is the property §36.1 chose one table to preserve.

## 42.8 Errors arrive in bundles now

§41.8 flags that `sem::check` bails on the first error and that a body walk over
a package is where that stops being acceptable. §42.2 extends the problem one
stage earlier: **every file in a package is lexed and parsed unconditionally**,
so a package with syntax errors in three files has three errors in hand and
today would report one.

The policy that follows: **collect across files, at both stages.** Nothing is
saved by stopping early, since the parse is happening regardless, and "fix this
one, run again, find the next" across a package is the failure mode §41.8
already names. This costs no parser recovery — `Parser::parse` still returns at
its first error, one error per file, and the loader accumulates across files —
so `syn` is untouched and the change is at the pipeline seam. `sem::check`'s
`Vec<Error>` is the same signature change §41.8 wants, made once for both
stages rather than twice.

For a one-module package this is invisible: one file, one error, the output
`main.rs` produces today.

## 42.9 What this closes

**`L8` closes.** `Module` sits in the prelude, by §42.6's membership rule rather
than by a budget decision.

**§4.18 cl. 31's open carrier closes: the loader.** The clause leaves it to
"whichever phase owns cross-package name resolution" whether an import naming an
unexported segment is a semantic-analysis obligation or a resolver check. It
reads two module maps and two export lists — headers and manifests, never an
AST body — which is §33.2's own criterion for keeping the import graph out of
the pass. It is the loader's, and it joins §35.2's row 10 rather than adding an
obligation.

**§34.8's expiry acquires a destination.** §38.8 unblocked it; §42.6 says where
the names go and what is left behind.

**`H1`'s "scaffolding by construction" acquires a precise meaning.** The prelude
tier does not expire — it *stops growing*. It keeps the native types, which the
seam and the operator arms depend on, and hands everything else to `G1`. That is
a smaller claim than the row makes and a more defensible one: a tier holding the
language's own vocabulary is the floor of the scope chain, not scaffolding.

**One thing is opened, and it is a process question.** §38.3's names-are-declared
rule plus §42.2's loader means every program is a package, so a bare
`mesa foo.ms` script mode would need a name from nowhere, a synthetic root
§38.2's pairing rule does not describe, and a second entry shape for the loader
forever. Declining it is right, and it lands on `tests/*.ms`, whose fixtures are
single files today. The specifics belong to whoever rewrites the harness and are
recorded in §1.5 rather than settled here.

---

# 43. Ordering is coarser than equality

§24.2 closed the built-in comparison set and §39.2 named the protocol `Order`,
but neither says what `compare` returning zero *means* about `==`. Rust's `Ord`
documents the strong answer — compare-equal exactly when equal — and a reader
arriving from there will assume mesa inherits it. **It does not.** `Order` is a
total order on whatever the implementor compares, which may be coarser than
equality, and the rule that decides this is §24.6's rather than a new one.

## 43.1 One direction is free; the other is the whole question

The two implications are not symmetric, and only one of them costs anything.

**`a == b` ⟹ `compare(a, b) == 0` holds by construction.** Equality is
structural (§24.6) and `compare` is a function of the same fields, so two values
equality cannot distinguish are two values `compare` was not given enough to
distinguish either. Nothing needs requiring: there is no way to write the
violation without writing an impure `compare`, which §43.3 rules out for
unrelated reasons.

**`compare(a, b) == 0` ⟹ `a == b` is the demand**, and it costs the most
ordinary reason to implement the protocol at all — ordering by a key.

```
type Task(priority, name)

impl Order
  def compare(other) return self.priority - other.priority end
end

Task(1, "write") == Task(1, "review")     # false, and correctly so
```

`compare` says zero; `==` says false, because the values differ in a field the
user can plainly see. Requiring the implication leaves that type two exits and
both are worse. It can override `Equal` to ignore `name` — which under §24.6
invalidates the derived `Hash`, so the type stops being usable as a key, and
`==` begins lying about a field `inspect` will happily print. Or it can decline
`Order` and lose `<` over an ordering that was perfectly good. **A rule whose
enforcement pushes ordinary types into misreporting their own contents is the
wrong rule.**

Worth naming the shape plainly, then: `compare` orders a *projection* of the
value rather than the whole of it. Java's `Comparator`, not Rust's `Ord`.

## 43.2 §24.6's test, applied a second time

It is tempting to read §24.6 as mesa taking a general position that protocols
must cohere, and to conclude that `Order` and `Equal` owe each other the debt
`Equal` and `Hash` do. That misreads why §24.6 holds.

**The coupling there exists because a mechanism depends on it.** Equal values
hashing equally is what dict lookup *is*; break it and lookup silently misses,
which is a correctness failure with no error attached. And note what §24.6
declines to require — the reverse direction — precisely because nothing
depends on the reverse direction. The section is not stating a hygiene
principle about protocols. It is paying for one mechanism.

So it transfers as a question rather than as a conclusion: **what breaks if
compare-equal values are not `==`?** Dicts key on `Equal` and `Hash` and never
call `compare`. The provided members — `min`, `max`, `clamp`, `between?`
(§34.1) — stay coherent over a coarse order, answering about the ordering key,
which is what an implementor who wrote a key-based `compare` meant to ask. The
four operators wire straight onto `compare` and have no opinion. Nothing in the
language uses `compare` as an identity, so there is no mechanism to pay for and
nothing to couple.

**The point where this changes is a sorted container** — a `SortedDict`, or a
set that dedupes by `compare` rather than by `Hash`. That is the mechanism the
invariant would protect, and it is exactly the case Java documents as
"inconsistent with `equals`", where a `TreeSet` and a `HashSet` disagree about
membership of the same values. If mesa grows one, the fix is §24.6's shape
scoped to the container — *this type* requires the invariant of its key — and
not a blanket clause on the protocol. Which is the same answer arrived at
twice: couple where a mechanism needs it, and nowhere else.

## 43.3 What the protocol requires instead

Three obligations, none of which mentions `==`:

- **Totality.** Any two values of the type compare. A `compare` that raises for
  some pairs of its own type is an ordering with a hole, and `min` over a list
  would find the hole at an arbitrary element. §19.3's `NotOrderable` is for
  operands the type was never given, not for a gap inside it.
- **Transitivity, and antisymmetry on the sign.** `min`, `max`, and `clamp` are
  meaningless without them, and so is any sort written on top.
- **Purity.** No observable effects. `<` calls `compare` on every comparison,
  and the operators promise nothing about how many times or in what order.

The relationship to `Equal` is then a note rather than a rule: equal values are
always compare-equal; compare-equal values need not be equal.

**None of the three is checkable the way §24.6's is**, and that is a second
reason for the note, worth separating from the first. §24.6 is a
declaration-site check — did you override `Equal`, did you declare `Hash`, did
you supply both — which is what lets it say "checked at declaration, with no
runtime cost". Every obligation here is a property quantified over pairs of
values: undecidable statically, and reachable at runtime only by calling `==`
alongside every `<`. Doubling the cost of the hot operator arm to enforce a rule
that protects nothing is a trade mesa declines elsewhere.

## 43.4 What this opens, and what it closes

**One thread opens, and it is small but real.** Because ties need not be `==`,
which operand `min` and `max` return on a tie becomes *observable* — under the
strong contract the two answers are indistinguishable, under this one they are
different values. §34.1's provided members want one sentence pinning it down,
and §36.4's widened receiver means the answer is spelled the same way for `Num`
as for a user type. Receiver-wins pairs pleasantly with §24.3's `<<` evaluating
to its receiver, but that is a resemblance and not an argument, so it goes to
§0.4 as a question rather than being settled here.

**What closes is the last clause of `E6`.** The roadmap row carries the built-in
comparison semantics, the mixed-operand rejection, and `NotOrderable`; what it
did not carry was the contract a *user* implementation signs, which §24.2
explicitly handed to "item 2's `Order` protocol". This is that clause. Nothing
about the `<` arms changes: `Num`, `Str`, and `Char` satisfy the strong
invariant anyway, each deriving ordering and equality from one representation —
which is why the strong contract looks obvious from inside `CORE_TYPES`, and
stops looking obvious the moment a user type orders by one field.

---

# 44. The phase describes, the interpreter constructs

§42 gave the loader an output and §42.4 gave that output a shape. §33 gave the
pass three outputs, of which the roadmap records one as built. Neither says
which phase is permitted to **construct a runtime value**, and the omission is
not academic: §42.4's map is written as holding `Val`s, which would put
`Rc<RefCell<…>>` allocation inside a phase whose admission rule refuses to so
much as ask a question about a value.

`B14` is what made the question concrete rather than tidy. Its conformance check
needs an environment — *which protocols exist, and what does each require* — and,
there being nowhere to put one, it builds a `Sym → &Proto` map per call, scoped
to a single chunk. §38.1 makes that environment package-wide, which §44.6 says a
map of borrows cannot be; and §33's outputs, as clause 4 lists them, have nowhere
to keep one that is.

The shape in one paragraph: **the semantic phase produces descriptions and
constructs nothing; the interpreter constructs every `Val`, every `Scope`, and
every `Obj` from them; the declaration tables the phase builds are a third
output, distinct in kind from §33's `ExprId`-keyed side tables; and what the
interpreter holds afterwards is an id into those tables paired with the one
thing a description cannot carry — the scope the declaration was made in.**

Nothing here reverses §38 or §42. One sentence of §42.4 is amended, `C0`'s
clause 4 gains a third output and clause 7 a second job, and one recommendation
is made about when to build it.

## 44.1 The line is clause 7's, applied one stage later

`C0` cl. 7 admits the pass to questions about **declarations** and refuses it
questions about **values**. It was written as a diagnostics rule — it is why
arity checking on a call, member-existence checking on `x.foo`, and `Member::set`'s
rejection of method names all stay at runtime — and it answers a second question
it was not aimed at:

> A phase that may not *ask* about a value has no business *building* one.

Read that way it fixes construction as well as inspection, and the three stages
separate cleanly:

| | The loader | The resolver | The interpreter |
| --- | --- | --- | --- |
| Runs | before any `Chunk` exists | over the package's `Chunk`s | over the package, in DAG order |
| Reads | `package.toml`, paths, files, headers | declarations, and under `L10` bodies | all of it |
| Produces | a module map of **descriptions** | diagnostics, resolution facts, declaration tables | side effects |
| Constructs | nothing | nothing | every `Val`, `Scope`, and `Obj` |

**The tree already half-agrees, and the naming hides it.** `eval_type_decl`
evaluates nothing. Field initialisers go in as `body_fields.insert(name, init)`
— the `ExprId`, unevaluated, because an initialiser runs per construction and
not per declaration; methods become `Proc`s holding `BlockId`s; a nested type
recurses into the same non-evaluating function. It is a lowering pass that
happens to run inside the walk, and its name is the only thing suggesting
otherwise.

## 44.2 §42.4's map holds descriptions, not values

The sentence to amend:

> The loader builds one `HashMap<Sym, Val>` per module, once, at load.

**It has two problems and they are one problem.** If the map holds `Val`s then
the loader is constructing runtime objects, against §44.1; and it cannot be
finished at load anyway, since a top-level `OS_NAME := when … end` needs string
comparison and conditional evaluation to determine its value. Either half alone
would be a wording defect. Together they say the map is the wrong structure.

**The correction is one substitution.** The loader builds `Sym → MemberDesc` —
what kind of member this is and where its declaration lives — and the
interpreter materialises `Sym → Val` from it. Declaration members are then
constructed rather than evaluated (`type Foo` is an id in a wrapper, `def foo`
is AST ids plus a scope handle), and evaluated bindings are filled in when their
module is evaluated.

**§42.4's argument survives untouched**, because it was never about who builds
the map. Its point is that members and scope are different *sets* — neither
contains the other — so `Obj::Module` cannot be a handle to the module's
`Scope`, on pain of an `import Utils` in `codec.ms` making `Codec.Utils`
readable from outside. Its second point, that a separate map is the arrangement
with fewer conditionals, is a claim about member-read shape. Both are indifferent
to the builder.

**That §42.4 was the outlier rather than the rule is visible in §4.19 cl. 12.**
Boot runs `CORE_TYPES` registration → the loader → the seam → the user's module,
and cl. 11's flattening of provided members into a type's member map — which is
`Rc<RefCell<Proc>>` work — sits at step *three*. The order already puts value
construction strictly after loading. One sentence in §42.4 disagreed with it.

## 44.3 The keys are static, and that is what makes a half-built map safe

Split the map in two and the phases stop competing for it:

- **Keys** — every member's name. Recoverable syntactically: `type`, `proto` and
  `def` names, child module names from the loaded headers, and the left-hand
  side of a top-level `x := …`.
- **Values** — a `Val` per key. Declaration members get theirs by construction;
  an evaluated binding gets its by running an expression.

**Everything before the interpreter reads keys only.** §38.4's duplicate-member
check compares a declaration's name against a child module's. §38.5's export
lists compose over names. §38.6's manifest checks are over dependency names.
§38.3's prefix checks are over declared paths. Not one of them needs a value,
which is cl. 7 holding rather than a coincidence.

**`B14` supplies the worked example.** Its `defaults_agree` compares default
parameter expressions syntactically and concedes agreement for anything that is
not a literal. That reads as a punt and partly is, but comparing `x: MAX_DEPTH`
against `x: MAX_DEPTH` *properly* would mean evaluating `MAX_DEPTH` during the
check. §34.7's "checked at declaration" is a syntactic claim, and the concession
is the admission rule showing through.

**§42.3's two jobs for the import DAG turn out to be one job seen twice.**
Detecting cycles and sequencing top-level evaluated bindings are given as
separate, and the second is exactly what makes a partially-valued map safe: a
module reading `Foo.OS_NAME` at its own top level imported `Foo`, so `Foo` was
evaluated first; a read inside a proc body is call-time, and §38.2's
"containment is not a load-time edge" covers the parent/child case by the same
argument. There is no window in which an unfilled slot is observable.

**One question is deliberately not opened here.** §0.1 makes a module's
top-level bindings members, which is what lets another module observe the
ordering the DAG exists to fix; confining them to module-private state would
close that hazard and cost the ability to export a computed constant. It
reverses a §28 clause and belongs to a section that argues it, not to this one.

## 44.4 The registry is already the description, minus one field

`rt::UserType` is not a runtime object that happens to hold AST ids. It is a
declaration record: `ctor_fields`, `body_fields` as `ExprId`s, `chunk`,
`enclosing`, `variants`, `protos`, and method maps of `Proc`s over `BlockId`s.
`rt::Proto`, added by `B14`, is the same in miniature — a member map whose
`Option` encodes required against provided, which is the fact `sem::is_provided`
recomputes from the AST by asking whether a block is empty.

**Two fields are not descriptions**: `scope: Rc<RefCell<Scope>>`, and
`Static::Proc`, which holds a closure. They are the environment a declaration
was made in, which is precisely what a description cannot carry — and they are
the whole of what stops the registry being built before evaluation.

So the split is not a judgement call, it is cl. 7 applied to a struct:

```rust
struct UserType { desc: TypeDescId, scope: Rc<RefCell<Scope>> }
```

**That a design rule written about diagnostics and a field welded in for closure
capture pick out the same boundary is the argument for the boundary.** Neither
was chosen with the other in view.

**Clause 4 acquires a third output, and it is a different kind from the other
two.** Cl. 4's outputs are resolution facts and capture sets, both keyed by
`ExprId` — per-expression facts, and the sym-table refactor's product. A type
or protocol table is keyed by declaration. The `C0` row already noticed the
distinction when `B14` landed, recording its environment as "one map, `Sym →
&Proto`, not clause 4's side tables, and it lives for the call rather than being
an output." This promotes it to an output and keeps the kinds apart.

**§42.2 is what makes it affordable.** The loader is eager over the whole
package and cannot be otherwise, so every `Chunk` exists before anything
resolves. A package-wide declaration table is not an optimisation added to that
arrangement; it is the environment §38.1's order-independent declarations
already require, given a place to live.

## 44.5 What the split deletes

**`resolve_proto` goes, and with it a wart.** `B14` resolves an `impl` name by
walking the ordinary scope chain at type-declaration time, carries a
`TypeError::NotProtocol` for when that finds a non-protocol, and documents that
a top-level binding can shadow a protocol between `eval`'s pre-pass and the
lookup. §42.5 already rules the other way — identity is not scope residency, you
import a protocol to write `impl Order` and never to use `<`, and no operator arm
consults the scope chain. Resolve `impl P` in the phase against the module map
and the function, the error variant, and the shadowing window all disappear
together.

**`eval`'s protocol pre-pass goes too.** It exists so a type can implement a
`proto` declared below it, which is §38.1's order-independence solved once, badly,
for one chunk.

**The four-tier method lookup becomes a candidate for flattening.**
`TypeRegistry::user_method` probes declared, then parent's declared, then
acquired, then parent's acquired. §4.19 cl. 11 already recommends flattening
provided members into a type's own member map at conformance time, so that
override and conflict "fall out of the data structure rather than being
implemented as separate rules"; §36.5 records searching at lookup as the coherent
alternative it is not recommending. `B14` implements the alternative. A
phase-built table is where the recommendation becomes available, since every
input to the flattening is static and dispatch is by `UserTypeId`. Recorded as
available rather than decided — it is a separate step and should not ride along.

## 44.6 An id is only meaningful with its arena

A description crosses a declaration boundary and a borrow does not, and that is
the property doing the work rather than the indirection itself.

**An `ExprId` or a `ProtoItemId` names a slot in one `Chunk`'s arena and nothing
else.** Held as a borrow out of the chunk being checked, that is invisible: the
id and the arena arrive together and the pairing cannot come apart. Stored in a
table spanning a package, the pairing is exactly what a description has to
carry, because the type being checked and the protocol constraining it need not
share a chunk — and a conformance check reads the second while walking the
first.

**So every id in a declaration table is a `ChunkId` and an index**, and a
cross-arena read stops being a thing that can be written. `Proc` has always been
shaped this way, holding `chunk` beside `body`, because the interpreter never
got to assume a proc's body and its call site shared an arena. The pass never
had that pressure and could take one chunk for the whole program; §38.1's
package-wide declarations are what withdraw the assumption.

**Locations need the same widening and are already equipped for it.** `Span`
carries a `SourceId` and `Span::loc(&Package)` resolves it, so a description's
spans stay meaningful across files with no new machinery — only a signature that
takes the package rather than a single `Source`. What the widening does open is
that a cross-module diagnostic wants *two* locations where every `sem::Error`
variant carries one: **`size` on `Ring` here does not match `Sized` declared
there**. §35.7's variant-per-failure-mode shape is unaffected — a second
`Location` is a field, not a redesign — and it is an ergonomic gap rather than a
correctness one, so it goes on the row rather than being settled here.

## 44.7 Sequencing, and one signature change with three reasons

**`Vec<Error>` is now overdetermined.** §41.8 wants it because a body walk over a
package is where one diagnostic per run stops being acceptable; §42.8 wants it
because every file is parsed unconditionally, so a package with three syntax
errors has three in hand; and a package-wide conformance check wants it because
conformance failures are exactly the kind that arrive in clusters. Three reasons,
one change, still worth making once.

**This is `G1`-shaped work and should not be pulled forward.** Before the loader
exists there is one module, and a phase-built registry is bit-for-bit the
registry `rt` already builds — the split would be paid for with no second chunk
to justify it. §42.2's eager whole-package parse is the precondition that makes
it mean anything.

So the recommendation is in two parts, and they are the same threading done to
two structures. **First**, at whatever grain the protocol environment currently
has: give it a `ChunkId` and let `sem` resolve spans through the package rather
than through a single `Source`. That is contained to one file and withdraws
§44.6's assumption at the only place that has yet made it. **Then, with `G1`**:
build the declaration tables in the phase, split `UserType`, and delete §44.5's
list. Doing the second first buys nothing the first does not, and the first is
not wasted by the second — the narrow structure is the wide one with a single
entry.

## 44.8 What this closes

**§42.4's sentence is amended**: the loader builds a map of member
*descriptions*, and the interpreter materialises the `Sym → Val` map from it.
Everything §42.4 argues — members and scope as different sets, `Obj::Module` not
being a `Scope` handle, the fewer-conditionals preference — stands unchanged,
since none of it is about the builder.

**`C0` cl. 4 gains a third output**, declaration tables keyed by declaration
rather than by `ExprId`, and cl. 1's "side tables keyed by `ExprId`" is narrowed
to cl. 4's first two outputs so the kinds stay apart.

**`C0` cl. 7 acquires a second job.** The admission rule governs construction as
well as inspection: the phase describes, and the interpreter builds. That is the
rule §42.4's sentence violated and the one §44.4's struct split follows.

**The `C0` row's `B14` amendment is superseded on one point.** It records the
`Sym → &Proto` environment as living "for the call rather than being an output";
under this section it becomes an output, and the borrow becomes a `ChunkId`-
carrying description.

**Nothing in §38 or §42 is reversed**, and `G1`'s specified surface is unchanged
— the loader still reads headers, still runs §38's four families of check, still
produces a module map, and still cannot be lazy.

**One thing is opened**: whether §4.19 cl. 11's flattening extends from provided
members to the whole of `user_method`'s four tiers, collapsing them into one map
per type at table-build time. Cl. 11 recommends flattening for the case it
covers and §36.5 leaves the general question open; a phase-built table is what
makes the general case decidable, and it is a separate decision from this
section's.

---

# 45. What reads the disk is not what decides

§41.7 placed the four families of static check in the loader and gave a reason:
the loader "runs before any `Chunk` exists". §42.2, written later, removed that
reason without noticing — it made the loader the thing that *creates* every
`Chunk`, and put the `module` header at the front of `chunk.top` rather than in
a header-only parse. Both sentences are still in the document and they cannot
both be true.

The seam matters because §41.2's rule keys a check's module to the error type it
raises, and the loader as built raises a fourth error vocabulary of its own —
`NoManifest`, `NoSrcDir`, `Unreadable` — which is neither `sem::Error` nor
anything §41 or §42 anticipated. So the arrangement that was specified and the
arrangement that exists disagree about which of them the four families belong to.

The shape in one paragraph: **the loader is filesystem and parsing and nothing
else; the four families are pure functions over what it produces; `load::Error`
narrows to the failures that happen before any source exists and therefore carry
no `Location`; every check raises `sem::Error` from `sem`, which is what §41.2's
rule asked for in the first place; and `Package` is storage rather than syntax,
which `syn` turns out not to reference once `Span::loc` is inverted.**

Nothing here changes `G1`'s surface, adds a check, or removes one. §41.7's
description of the loader is retired, §42.1's table is redrawn, §42.9's
resolution of `§4.18` cl. 31 is restated against a different carrier, and one
sentence of §44.6 is respelled.

## 45.1 The distinction was temporal and is now dimensional

§42.1's table separates the loader from the resolver by when they run and what
they read. After §42.2 the first column is gone: nothing runs before a `Chunk`
exists except `read_dir` and `read_to_string`, because a header *is* an AST node
and reading one means having parsed the file that holds it.

What survives is the second column, and it is a different kind of claim:

| | Reads | Raises |
| --- | --- | --- |
| The loader | `package.toml`'s bytes, paths, file contents | IO failure |
| The four families | paths, headers, the manifest, top-level names | `sem::Error` |
| The pass (`sem::check`) | declarations, and under `L10` bodies | `sem::Error` |

**An input set is a function signature, not a module boundary.** The families
read no bodies, which is §33.2's criterion for keeping them out of the pass's
traversal — and §41.7 is right that the traversal is what the loader does not
share. But not sharing a traversal is an argument for a separate entry point,
which the families already are. It was never an argument for sharing a module
with `read_dir`.

## 45.2 The families are pure, and that is worth more than the placement

Each family is a total function from the loader's output to a `Vec<Error>`:

- **Layout invariants** (cl. 11) read the path list alone.
- **Prefix checks** (cl. 15) read the path list and each file's declared name.
- **Duplicate members** (cl. 23) read declared names and, per §44.3, the *key* of
  every top-level binding — never a value and never a body.
- **Manifest dependency names** (cl. 31) read `package.toml`, the module maps and
  the export lists.

None of the four needs a directory to exist. A prefix mismatch, a misfiled
module, a duplicate member and a malformed dependency name are all checkable
against a literal table of paths and headers, which is the difference between a
unit test and a temporary directory plus a subprocess. §38 specifies four
families with many failure modes each; the harness cost of testing them through
the filesystem is the largest avoidable cost in `G1`, and it is avoidable only
if the checks do not themselves read the filesystem.

**This is the same move §44.1 made one stage earlier.** The phase describes and
the interpreter constructs; here the loader observes and the phase decides. Both
are the same rule — a stage that acquires facts should not also be the stage
that rules on them.

## 45.3 `load::Error` narrows, and §41.2's rule stops being strained

The loader's remaining failures are `package.toml` absent, `src/` absent,
`src/package.ms` absent, and a path that will not read. **None of them can carry
a `Location`**, because a `Location` names a position in a source and at that
point no source has been read. They are also not claims about a program's
meaning: "there is no package here" is the shape of `usage:`, not of a
diagnostic.

So `load::Error` is not a fourth phase vocabulary; it is the IO vocabulary of
one function, and §41.2 has nothing to say about it, since §41.2 governs
*checks*. Every check raises `sem::Error` from `sem`. The rule holds exactly
rather than by the courtesy of a submodule, which is what made the arrangement
uncomfortable to describe.

## 45.4 Two things the split must not lose

**Directories have to survive into the loader's output.** Cl. 11's first
invariant — a directory with no sibling file — is recoverable from a file list
only when the directory holds at least one `.ms` file, since `src/codec` is then
derivable from `src/codec/frame.ms`. An empty `src/codec/`, or one holding only
a `README`, appears nowhere in a list of parsed files, and the check that exists
to catch exactly this silently passes. A walker that checks as it descends sees
those directories for free; a pure check sees them only if the output records
them. **The loader's output is therefore the directory tree and the parsed
files, not the parsed files alone** — and that is the one place where making the
checks pure costs something rather than saving it.

**`Location` needs a form that names a file and no position.** A missing sibling
file has no text to point at, and under this section the layout invariants are
`sem::Error`s, whose `Display` writes `{file}:{lin},{col}: semantic error:`
unconditionally. Whether that is an `Option` inside `Location`, a second
constructor, or a distinct type is an implementation choice `§4.7` cl. 12 still
governs; that it is needed is not. This joins §44.6's observation that a
cross-module diagnostic wants *two* locations: both are `Location` growing to
fit a package, and neither is a redesign.

## 45.5 `Package` is storage, and `syn` does not reference it

The struct holds `Vec<Source>` and `Vec<Chunk>` — the sources a package was
built from and the chunks they parsed to. It is named for §38.1's unit, and
§38.1's unit is a compilation, evaluation and distribution concept, not a
syntactic one. It sits in `syn` for one reason: `Span::loc(&Package)`.

**Invert that call and the dependency disappears.** `syn` names `Package` in
exactly three places — the struct, its `impl`, and `Span::loc` — and names
`ChunkId` only inside that `impl`; `Chunk` carries its `SourceId` and needs
nothing else. So `Package::loc(span)` in place of `Span::loc(pkg)` leaves `syn`
with no reference to either type, and `syn` becomes what it should have been:
text to tokens to one `Chunk`, indifferent to how many files a program has.
`§4.7` cl. 12's carrier freedom is not engaged, since this is a method's home
rather than a check's.

**The landing site is its own module, not `sem`.** `rt` holds a `&Package` for
the arena it walks, and `sem` reads `rt::CORE_TYPE_NAMES` for the prelude check,
so `Package` in `sem` makes the checker and the interpreter mutually dependent.
A storage module below both — where `intern` already sits — leaves `sem` and
`rt` as independent branches off `syn`, and the interpreter with no reason to
name the checker's module at all.

**One edge has to move for that to be true, and it is pointing the wrong way
regardless.** The prelude's names are the language's vocabulary, not the
runtime's — §42.6 admits `Module` to it by a rule about the language, and §42.9
calls the tier "the language's own vocabulary… the floor of the scope chain".
`sem` reaching into `rt` for that list is the only edge in the tree that inverts
the phase order, and it is what currently forces `sem` above `rt` when nothing
else about the two requires it.

## 45.6 Cl. 31 is mostly unbuildable until a second package exists

Clause 31 carries four checks and they do not land together. **Three are
manifest-local** — every dependency's root name is a valid module name, unique
among dependencies, and distinct from the importing package's own root — and
need only `package.toml`. **The fourth is cross-package**: an import fails if any
segment on its path is unexported, which requires a second package's module map
and export lists to read.

There is no second package until `H1` puts the stdlib on disk (§42.7), which
`G1` explicitly does not include. So the fourth check has nothing to fire
against and should be scoped out rather than written blind against an interface
no caller implements yet. The three that remain are worth having and are cheap.

**One consequence for sequencing**: the manifest stops being a marker and starts
being a document the moment the trio is built. Locating `package.toml` without
reading it is correct exactly until cl. 31, and no longer.

## 45.7 What this closes

**§41.7's description of the loader is retired.** "It runs before any `Chunk`
exists" was true when written and false after §42.2. What §41.7 gets right and
keeps is that the loader shares the module and not the traversal; what it loses
is the inference that the four families therefore belong to it.

**§42.1's table is redrawn** along inputs rather than time, and gains a third
row, since the families are neither the loader nor the pass.

**§42.9's resolution of `§4.18` cl. 31 is restated.** The clause asked whether
the unexported-segment check is a semantic-analysis obligation or a resolver
check; §42.9 answered "the loader" on the grounds that it reads headers and
manifests and never an AST body. That reasoning is untouched and its conclusion
narrows: the carrier is **the phase, over the loader's output** — still not the
pass's traversal, so §33.2's criterion is satisfied exactly as §42.9 argued.
`§4.18` cl. 31's roadmap text carries the superseded carrier and should be
amended to match.

**§44.6's sentence is respelled**, not amended. It observes that `Span` carries
a `SourceId` and that `Span::loc(&Package)` resolves it "with no new machinery —
only a signature that takes the package rather than a single `Source`". The
signature is now `Package::loc(span)` and the observation is unchanged.

**Nothing in §38 is reversed.** The four families are the same four families,
checking the same things, at the same point in the pipeline, and the module map
is still the loader-side structure §42.2 and §44.2 describe.

**One thing is opened.** Cl. 11's second layout invariant — `src/` holds exactly
one file that no directory pairs with — has no enforceable form yet distinct
from "`src/package.ms` exists". A sibling file with no directory is legal
(`transport.ms` in §38.2's own tree), so the clause cannot mean what its
sentence most readily suggests. It is a spelling question rather than a design
one, and it belongs to whoever writes the layout family.

---

# 46. The stdlib is a package, and packages meet at the scope boundary

§34.8 designed `H1` for a world where `G1` did not exist: an embedded `&str` of
mesa source, parsed and evaluated into `C2`'s prelude tier, with a synthetic
source identity and an unshadowable-check extension over everything it declared.
§42.6 gave the scaffolding a destination and §42.7 named files on disk as the
long-run answer. With `G1` built, the long run has arrived early and the interim
never needs to exist.

**The stdlib is an ordinary package**, loaded by the same pipeline as the user's
— `load::find`, `load::collect`, `load::parse`, `sem::check` — and distributed
as an on-disk directory with a `package.toml`, a `src/package.ms`, and as many
child modules as it needs. A user program reaches it through `import`, not
through the prelude. The prelude holds the native types and nothing else, which
is §42.6's settled rule, and this section does not amend it.

The consequence is that `H1` collapses into the dependency infrastructure and
`H2`'s protocols live in importable modules rather than prelude bindings. The
scaffolding phase — prelude ballooning, unshadowable-check extension,
scaffolding-then-retraction — is deleted by never having been built.

Nothing in §38, §42, §44, or §45 is reversed. The stdlib is a package as §42.7
recommends; the seam resolves identities as §42.5 requires; `G1`'s surface is
unchanged; and `§4.19`'s boot order stands.

## 46.1 One package per `Modules`, and packages meet at imports

Each package — the stdlib and the user's — is its own `Package`, its own
`Modules`, its own `Types`, its own scopes. The arena ids that index those
structures (`ModuleId`, `ChunkId`, `TypeId`, `ProtoId`) are local to one package
and meaningless outside it, which is the property this section is shaped around.

The only place two packages meet is at import resolution, and the resolution
immediately produces a value rather than storing a cross-package id.

**The rule**: a cross-package import is resolved once during setup and the
resulting `Val` is inserted directly into the importing module's scope locals.
After insertion, `resolve_name` finds it as an ordinary `Tier::Module` local.
No downstream type — `Binding`, `Place`, `Member`, `Target` — carries a
cross-package variant; the package boundary is invisible after setup.

This is the same operation `bind_module_item` already performs for declarations:
`scope.locals.insert(name, val)`. Cross-package imports are one more kind of
thing to bind during setup, and the mechanism is the same line used throughout.

## 46.2 Why pre-binding is correct

The concern with pre-binding — as against the live lookup intra-package imports
use — is that it is a snapshot. Three properties make the snapshot correct:

**Dependency scopes are frozen.** The stdlib is fully evaluated before any
user declaration binds. No user code can reach into a dependency's scope to
rebind a name, per §4.18 cl. 3 (members are read-only from outside). The
snapshot is permanent.

**Object-level mutation is shared.** A pre-bound `Val::Obj` shares the
`Rc<RefCell<…>>` with the dependency's scope. Mutations to the object's
contents — fields on an instance, items in a list, a call to a proc that
mutates its own module's state — are visible through both references. The
mechanism §4.18 cl. 3 describes for exposing mutable state (a proc that
mutates its own scope from inside) works unchanged, because the proc's
closure captures its declaring module's scope, not the importer's copy.

**Name-level rebinding does not cross the boundary.** Rebinding a name in
the importing module's scope — `Order := something_else` — writes to the
local copy and does not reach back into the dependency's scope. For
intra-package imports this would be a divergence: the live lookup would see
the source module's binding, not the importer's. For cross-package imports
no mechanism in the language permits it, so the distinction is inert. If a
future need arose — "change the defaults of some package" — the answer is the
same proc-based mutation §4.18 cl. 3 already provides, not rebinding a name
across a package boundary.

## 46.3 What changes in `sem`

`sem/modules.rs` gains a notion of dependency packages available to import
resolution. Each dependency is identified by its root module name (a `Sym`),
per §4.18 cl. 30.

**Import resolution** widens from one `Modules` to one `Modules` plus a map of
dependencies. The first segment of an import path is checked against the
dependency root names; if it matches, the remaining segments are resolved within
that dependency's `Modules`. The result is stored separately from intra-package
imports:

A `DepImport` records the validated cross-package reference: the dependency's
root name (`Sym`), the resolved `ModuleId` within the dependency, optionally a
member name (`Sym`) if importing a member rather than a module, the name it
binds in the importing module's scope (the last path segment), and the `Span`
for error reporting.

`Module` gains `dep_imports: Vec<DepImport>` alongside the existing
`imports: Vec<Import>`. The existing `Target` enum is unchanged — it represents
intra-package targets only.

**Cycle detection and evaluation ordering** skip dependency edges. A
dependency is already fully resolved and evaluated; it cannot participate in a
cycle within the user's package, and its evaluation is not ordered by the user's
DAG.

**`binding()`** is unchanged — it sees only intra-package imports. The pre-bound
values are in the scope by the time `resolve_name` calls it, so they are found
as locals before `binding()` is consulted.

## 46.4 What changes in `rt`

The interpreter holds a dependency table: a map from each dependency's root
module name to its loaded runtime state (`Package`, `Types`, `rt::Modules`).

**`eval()` gains one step** between "bind declarations" and "evaluate
expressions": iterate each module's `dep_imports`, look up the dependency by
root name, resolve the target module or member in the dependency's `rt::Modules`
and scopes, and insert the resulting `Val` into the importing module's scope.

After this step, the importing module's scope contains the dependency values as
ordinary locals. `resolve_name`, `Place`, `Binding`, `Member` — none of them
change.

## 46.5 The prelude is shared

The prelude scope is built once and shared across all packages. It holds the
native type names (`CORE_TYPES`) per §42.6 and nothing else. The stdlib
package's module scopes chain from it; the user package's module scopes chain
from it. Protocols live in the stdlib's modules and are reached by import, not
by the prelude.

## 46.6 The seam survives, and its position in the boot order is unchanged

§4.19's boot order stands:

1. `CORE_TYPES` registration — unchanged.
2. The stdlib package — loaded, checked, and evaluated by the same pipeline as
   the user's. Its `proto` declarations bind `Obj::Proto` values in its module
   scopes.
3. The seam — the interpreter builds the protocol-identity table by looking up
   known protocol names in the stdlib's scopes, declares which native types
   conform, checks conformance at boot, and flattens provided members into
   conforming native types' member maps. §4.19 cls. 1–7 and 11–13 apply here
   exactly as written.
4. The user's package — loaded, checked, and evaluated. Cross-package imports
   from the stdlib are pre-bound during setup.

§42.5's separation holds: the protocols are evaluated at boot (step 2) but need
not be in scope at boot. The seam table holds identities, and operator arms ask
about conformance rather than consulting the scope chain. A user writes `import
Core.Order` to spell `impl Order` and never to use `<`.

## 46.7 The stdlib on disk

The stdlib is an ordinary package directory:

```
<stdlib-root>/
  package.toml
  src/
    package.ms
    order.ms
    equal.ms
    hash.ms
    iterate.ms
    advance.ms
    display.ms
    inspect.ms
    access.ms
    append.ms
```

The root module name and the location of this directory on disk are open. For
the root name, the choice is between `Core` (Swift's precedent), `Std` (Rust's),
and the language's own name. For the location, the options are a path relative
to the binary, an environment variable, a compiled-in path, or a combination.
Neither decision is load-bearing for this section's architecture; both are
spelling.

## 46.8 `[deps]` in the manifest

The stdlib is an **implicit** dependency — loaded without a `[deps]` entry, as
Rust's `std` is. Explicit `[deps]` support can wait until a second package
exists that is not implicit. The three manifest-local checks of §4.18 cl. 31
(valid module name, unique among dependencies, distinct from own root) are cheap
and worth building with `[deps]`; they are the checks §45.6 declared buildable.

The fourth check — an import failing when any segment on its path is unexported
— becomes buildable for the first time, since a second package's module map and
export lists now exist to read.

## 46.9 What this closes, and what it opens

**`H1`'s row is collapsed into this section's infrastructure.** The stdlib
loader is the general dependency loader applied to one implicit dependency.
The three needs `H1`'s row listed — synthetic source identity, fixed startup
cost, and prelude-check extension — are revised. The synthetic source identity
is deleted by using real files (§42.7's recommendation). The startup cost is
unchanged in magnitude and now includes an on-disk read, which §42.7 prices as
the cheap half. The prelude-check extension never happens, because the prelude
never grows beyond the native types; the stdlib's names are ordinary imported
names, unshadowable only in the same sense any module member is — by §4.18
cl. 3's read-only rule at the package boundary, not by a scope-chain prohibition.

**`H2`'s prerequisites are reduced by one.** `H2` still needs `L12`, `D7`,
`E6`, `D9`, and `F3`; it no longer needs `H1` as a separate step, since the
loader it would have used is the one this section builds.

**§42.6's prelude arc simplifies.** The three-stage trajectory — today's ten
names, then a balloon under `H1`, then a contraction after `G1` — becomes two
stages: today's ten names, then the same ten plus `Module`. No balloon, no
contraction. The unshadowable check protects a fixed set and the direction is
never reversed.

**One thing is opened**: cross-package import of *types*, not only modules. A
user writing `import Core.Order` gets the `Order` module as a value; writing
`import Core.Order.Order` would get the `Order` *protocol* as a value, since a
protocol is a member of its declaring module. Whether the shorter spelling
should reach the protocol directly — a re-export from the module, or a rule
that a single-member module exposes its member — is a convenience question the
architecture does not constrain.

---

# 47. Native types beyond the core set

§36 designed the seam between mesa-declared protocols and Rust-implemented
types, and §46 made the stdlib a package loaded by the same pipeline as the
user's. Both assume the only native types are the core set — `Nil`, `Num`,
`Bool`, `Char`, `Str`, `List`, `Dict`, `Proc`, `Type`, `Proto`, `Module` —
registered at compile time in `CORE_TYPES` with fixed `NativeTypeId` constants
and residing permanently in the prelude. Nothing in the stdlib or in a
third-party package can introduce a type backed by Rust data.

That boundary is the subject of this section. Types like files, sockets, and
HTTP parsers want Rust implementations behind them — OS resources, byte
buffers, protocol state machines — while living in importable modules rather
than the prelude, and participating in the protocol system as any user type
would. The core types earned their position because language constructs need
them by compile-time identity (`each` needs `List`, operators need `Num` and
`Str`, `$type` needs `Type`); nothing in the language needs `File` by identity,
so it has no reason to be in `CORE_TYPES` and no reason to be in the prelude.

The shape in one paragraph: **a new `extern` declaration gives a native type a
mesa-side declaration site, which is the one thing §36.2 said was missing; that
declaration carries `impl` lines, dissolving `L14`'s restriction; an
`Obj::Native` variant holds type-erased Rust data behind the same
`Rc<RefCell<…>>` the existing `Obj` kinds use; extended native types share
`NativeTypeId`'s index space with the core set, their ids assigned at
registration rather than fixed at compile time; and the `extern` body may mix
mesa methods with the native members, choosing per member whether the
implementation is Rust or mesa.**

Nothing here reverses §36, §38, §42, §44, or §46. §36.2's consequence —
`L14`, native types cannot implement user protocols — is reversed by this
section and by this section alone, on the ground that `extern` removes the
premise §36.2 reasoned from. §4.19's boot order stands unchanged for the core
types; extended native types' conformance is checked during normal evaluation
rather than at a separate seam step.

## 47.1 `Obj::Native` and the id space

`Val`/`Obj` gains one variant:

```
Obj::Native(NativeTypeId, T)
```

where `T` is type-erased (`Box<dyn Any>`, or whatever the concrete erasure is —
the choice is implementation, not design). A single erased variant rather than
one variant per native type: `Obj::File(File)`, `Obj::Socket(Socket)`, etc.
would proliferate `Obj` arms every time the stdlib or a third-party package adds
a type, and every `match` on `Obj` would grow. One variant with an id scales
without touching the enum.

**The id space is already dynamic.** `NativeTypeId` is a `u32` indexing
`Vec<NativeType>`. The core types occupy the first eleven slots with
compile-time constants (`NativeTypeId::NIL` through `NativeTypeId::MODULE`);
extended native types are appended after them, their ids assigned at
registration. The constants stay as they are. Nothing in the lookup path — 
`Types::native`, `Types::member`, `Val::type_id` — assumes a bounded id space;
the `Vec` grows.

**`Val::type_id` returns `TypeId::Native(id)` for an `Obj::Native`.** Member
lookup already dispatches through `NativeType.members` for native types, so an
extended native type's members are found by the same path `Str.size` uses today.
No new dispatch arm, no new lookup tier.

## 47.2 The `extern` declaration

```
module IO

extern type File
  impl Iterate

  def summary
    "<File: " + self.path + ">"
  end
end
```

**`extern type` is the declaration site §36.2 said was missing.** §36.2's
argument was: "`impl` attaches to a type declaration (§20.1), and native types
have none." The conclusion — native types cannot implement protocols — followed
from the absence of a declaration site. `extern` provides one, and the
conclusion no longer follows.

**The body accepts `def` and `impl`, but not constructor fields or body
fields.** Constructor fields assume an `Obj::Instance` with a mesa-managed
field map; body fields assume per-instance mesa storage. An `Obj::Native` has
opaque Rust data — its layout is Rust's. Construction is always native: a `new`
function or static factory methods on the Rust registration. So `extern type`
has no parameter list after the name, and the body is a subset of a regular
`type` body.

**`self` in a mesa method is opaque.** Inside `def summary`, `self` is a `Val`
wrapping `Obj::Native(id, data)`. The method cannot read or write the Rust data
directly; it reaches it through native members (`self.path`, `self.size`) that
know how to downcast. This is the same encapsulation user types already have —
fields are accessed through methods — enforced here by the Rust/mesa boundary
rather than by convention.

**`extern def` on a non-extern type is declined.** A native method on a user
type would receive an `Obj::Instance` whose fields are mesa-managed; everything
the method could do is expressible in mesa, since the data is mesa's. The case
where native helps — touching Rust data structures, calling Rust libraries,
holding OS resources — requires a native type, because the data needs to live
in Rust. `extern` marks a type whose data is in Rust; methods follow the data.

## 47.3 `§38.3` holds: names are declared, never derived

The `extern type` declaration lives in a `.ms` file, written by the package
author, with a `module` header and optionally an `export` list. The loader sees
it as a member of the declaring module — same as a `type` or a `def` — and
§38.3's invariant is untouched: every name in the module map came from a `.ms`
header.

**The alternative — Rust-side injection into the module map — would break
§38.3.** A module map entry not sourced from any `.ms` file is a name that
cannot be found by reading the source, which is the local-reasoning failure
§38.3 exists to prevent. `extern` avoids it: the mesa file declares the name
and the Rust side fulfils it.

**The loader needs no special treatment for `extern`.** It reads the file,
parses the declaration, records the member in the module's member set. The
`extern` keyword is visible to `syn` and `sem`, and the interpreter binds the
type from the Rust registration rather than building a `UserType`. The loader
does not distinguish native from non-native members, and never needs to.

## 47.4 Hybrid modules are ordinary modules

A module containing both `extern type` declarations and mesa `def`s is not a
special case.

```
module IO

extern type File
  impl Iterate
end

def read_all(path)
  file := File.open(path)
  content := file.read
  file.close
  content
end
```

The loader sees two members: `File` and `read_all`. Both are in the member set,
both participate in export-list composition, both are reachable by import. The
interpreter binds `File` from the Rust registration and `read_all` from the
parsed `def`. `read_all` can reference `File` because `File` is in the same
module's scope — it was bound during the declaration-binding step, which is
order-independent (§38.1), so it is available before any evaluated bindings run.

**No new loader step, no new module variant, no ordering exception.**

## 47.5 The member map and lookup order

An `extern type`'s members come from three sources:

1. **Native members** — from the Rust registration. These are the functions that
   know how to downcast the `Obj::Native` payload and operate on it.
2. **Mesa methods** — from `def`s in the `extern type` body. These are ordinary
   `Proc`s that call native members through `self`.
3. **Acquired provided members** — from `impl`'d protocols. These are mesa
   `Proc`s flattened into the member map at conformance time, per §4.19 cl. 11.

**Collisions between (1) and (2) are errors**, detected at bind time when the
interpreter merges the Rust registration with the parsed body. A native `read`
and a mesa `def read` in the same `extern type` is a broken declaration, caught
before any user code runs.

**Collisions between (1)/(2) and (3) are overrides** — the type's own member
wins over a provided body, which is §4.5 cl. 15 unchanged. `List` overriding
`size` natively while acquiring it as a provided member is this rule applied.

**The per-member split is the point.** A package author chooses, member by
member, whether each one is native or mesa. Performance-critical operations
touching Rust data (`read`, `write`, `close`, `path`, `size`) are native.
Convenience methods, formatting, anything that composes over the native API
(`summary`, `display`, `read_lines`) are mesa. The split can shift without
changing the public API — moving a member from mesa to native or vice versa is
invisible to callers.

## 47.6 `NativeType` gains statics

Core types today have `new: Option<fn() -> Val>`, a zero-argument factory. An
extended native type like `File` needs richer construction — `File.open(path,
mode: "r")` — which is a static method on the type value, not a member on
instances.

**`NativeType` gains a `statics` map, parallel to `UserType`'s.** Static
members on a native type are `NativeMember`s invoked when the type value is in
callee position with a qualifying name: `File.open(path)` is a static-member
access on `File`'s type value, resolved through `Types::member`'s existing
`namespace_type_id` path. The pattern is exact: `UserType` already has
`statics: FxHashMap<Sym, Static>`, and the static-member lookup arm in
`Types::member` already checks `has_static` before falling through to the
native member path.

## 47.7 Protocol conformance via `extern` dissolves `L14`

§36.2 argued: native types have no mesa declaration, so `impl` has nowhere to
attach, so native types cannot implement user protocols. `extern type` provides
the declaration. The argument's premise is removed and its conclusion no longer
holds.

**The mechanism is the existing one.** An `impl Iterate` on an `extern type`
resolves `Iterate` through the module map (§42.5), exactly as `impl Iterate` on
a user type does. The conformance check — each required member present, with
matching parameter names (§36.3) — runs over the merged member map at bind
time. Provided members are flattened per §4.19 cl. 11. Nothing in the check
distinguishes a native member from a mesa method; it asks whether the name
exists with the right signature.

**What this changes about the seam.** The seam (§4.19 cl. 12, step 3) survives
unchanged for core types: their conformance is declared in Rust, in §36.1's
table, and checked at boot. For extended native types, conformance is declared
in mesa via `impl` on the `extern` declaration and checked during normal
evaluation — during step 2 for stdlib types, during step 4 for user-package
types. The seam step does not widen; the `extern` path handles everything
outside the core set.

**`L14`'s narrowing becomes a dependency-graph question.** A native type can
implement any protocol it can `import`. For the stdlib, that means its own
protocols (same package, order-independent). For a third-party package, it means
protocols from any dependency. A native type in package `foo` implementing
`Core.Iterate` writes:

```
import Core.Iterate

extern type NativeWidget
  impl Iterate
end
```

The dependency ordering makes `Iterate`'s identity available. No special case.
A native type implementing a protocol from its own package works identically —
the protocol is an order-independent declaration in the same package.

The one structural impossibility — the stdlib implementing a *user* protocol on
a native type — is not a limitation of the mechanism but of the dependency
graph: the stdlib has no dependencies on user packages, so it cannot import
their protocols. This is the same constraint any package without the dependency
would face, applied uniformly.

## 47.8 Third-party packages and the registration seam

A third-party package may bring its own native types. The mechanism is the same
as the stdlib's:

1. **A Rust registration** provides the native types — their names, native
   members, statics, and constructors — into a registry the mesa runtime looks
   up. This is `CORE_TYPES` generalised from one compile-time table to a
   per-package runtime table.
2. **`extern` declarations in `.ms` source** provide the namespace entry points
   and protocol contracts. The loader sees these as ordinary members; the
   interpreter binds them from the package's Rust registration.
3. **A matching step** at bind time pairs each `extern type` with its Rust
   registration, keyed by package identity and declared name. A missing
   registration is a panic — a broken build, same as §36.1's rule for a missing
   protocol entry.
4. **Per-package conformance checking** runs during that package's evaluation,
   not at a single global seam. Each `impl` on an `extern type` is checked
   against the protocol's requirements at the point the declaration is bound.

**The concrete Rust API for registration — whether it is a table, a builder, a
trait, or a proc macro — is an implementation question.** It is not designed
here because it can be built when the first non-stdlib native type is built, and
the design it must satisfy is specified by (1)–(4) above rather than by its own
ergonomics.

## 47.9 The `NativeMember.call` signature is unchanged

`NativeMember.call` is `fn(&Val, Vec<Val>) -> Result<Val, Error>`. For core
types this works because the member function knows what the `Val` is — `Str::size`
knows it holds a `Str`. For an extended native type, the function receives a
`Val` wrapping `Obj::Native(id, data)` and downcasts `data`. The signature is
general enough; the downcast is an implementation step, not a design change.

**Whether to add a typed wrapper** that does the downcast once and presents a
`&T` is an ergonomic question for the registration API, not for this section.

## 47.10 What this closes, and what it opens

**`L14` is dissolved.** A native type may implement any protocol it can import.
The limitation was structural — no declaration site for `impl` — and `extern`
removes it. The dependency graph is now the only constraint, applied uniformly
to native and user types alike.

**`H2`'s seam is scoped.** The seam (§4.19 cl. 12, step 3) remains for the core
set; extended native types go through `extern`+`impl` during normal evaluation.
The seam could eventually be folded into the `extern` mechanism if core types
acquired `extern` declarations in stdlib source, but that is an optional
simplification and not a goal of this section.

**`NativeType` gains two things**: a `statics` map for construction and factory
methods, and a capacity for mesa `Proc`s alongside native members in the merged
member map (§36.5's dual member map, resolved here as a consequence of `extern`
bodies containing `def`s and `impl`s that flatten provided bodies).

**One thing is not designed here**: the registration API. Its shape is
constrained — it must produce the data §47.8's four steps consume — but its
ergonomics are an implementation question that can be deferred until the first
extended native type is built.

**One thing is opened**: whether core types should eventually gain `extern`
declarations in the stdlib, unifying the two paths and allowing the seam step
to be deleted. The core types' compile-time constants and prelude residency are
not obstacles — `extern type List` in a stdlib module could coexist with
`NativeTypeId::LIST` in `CORE_TYPES`, one providing the declaration site and the
other the compile-time identity language constructs need. Whether the
unification is worth the ceremony is a question for after both paths exist.

---

# 48. Cleanup is a region, not a resource

Mesa has no cleanup mechanism, and the gap is recorded in three places without
ever being designed. §4.15 cl. 7 accepts that an iterator holding a resource
gets no cleanup hook on `break`, calling it "a general no-destructors gap
rather than an iteration one." §20.8 repeats it while pricing generators:
mesa has "neither `finally` nor destructors." And §4.17 settles `raise`/`rescue`
end to end across eleven clauses without a cleanup clause among them.

§47 makes the gap live rather than theoretical. An `extern type File` puts an
fd behind the same `Rc<RefCell<…>>` every other `Obj` uses, so release becomes
refcount-driven: prompt in the common case, non-deterministic under any
indirection, and absent entirely inside a cycle. §30.3 already records that a
`Proc` holding captured `Rc`s can cycle with an object holding the proc, and
files up-rate that from a leaked allocation to a leaked descriptor. This is
CPython's model, and CPython still needed `with`.

The shape in one paragraph: **cleanup attaches to a lexical region (`ensure`)
rather than to a value's type (`with`); the two share their entire runtime, so
this decides a surface and not a mechanism; the hardest question is shared and
is left open here deliberately; and `with` remains available later as an
additive construct that this section does not foreclose.**

## 48.1 Two shapes, one runtime

The two candidate designs differ only in where the cleanup is written.

| | `ensure` | `with` |
| --- | --- | --- |
| Cleanup attaches to | a lexical region | a value's type |
| Cleanup is | an inline block | a protocol member |
| Found by | position | dispatch |
| Body's `return`/`break` | ordinary — same frame | ordinary — same frame |

What they do not differ on is the machinery. Both need cleanup to run on **all
four exit paths** — normal completion, `Signal::Return`, `Signal::Break`, and
`Signal::Error` — without corrupting the signal in flight or the trace it
carries. That is one implementation, built once, whichever surface arrives
first.

The shape already exists in the tree. `eval_block` matches on exactly those
four outcomes and each arm does restore-then-propagate for `self.scope`. A
cleanup construct is that same pattern with a block evaluation where the
restore sits.

So the choice recorded below is which surface to design, not which mechanism to
build, and the sunk work is the same either way.

## 48.2 Decided: the region form

Three reasons, in the order they carry weight.

**`ensure` is strictly more general.** A `with` handles only cleanup that can be
attached to a value's *type*. Restoring a working directory, putting a flag
back, decrementing a counter, undoing a temporary state change — none of these
has an object to hang on, and declaring a type to carry a two-line undo is
ceremony in a language whose stated values push the other way. The region form
covers the resource case *and* those; the resource form covers only the first.

**`F2` forces the region question anyway.** §4.17 specifies rescue's arm syntax,
its inverted totality, `else`, grouping, and structural equality, and never says
what region a rescue *protects*. Ruby has `begin … end`; mesa has no `begin`,
and `do` is spoken for. That question has to be answered to build `rescue` at
all. Once it is, `ensure` is a third clause on a construct that already exists
rather than a construct of its own — and the cost of the region form drops to
roughly a keyword.

**A `with` designed now would be designed against a hypothetical.** There is no
`File` in the tree; `lib/src/` holds a `Counter` and a `print` wrapper. Shaping
a protocol around an imagined resource is the one kind of mistake protocols make
expensive to correct: §34 makes *adding* a provided member additive, and changing
a required one is not. A resource protocol is worth writing when there is a
resource to write it against.

**The argument on the other side, recorded rather than dismissed.** §47's native
exit means a `with` could ship with no mesa-level `ensure` at all, the
interpreter running an `extern` type's exit on unwind. That argument becomes
strong the moment a real `File` lands and weak until then, which is exactly why
it is a sequencing observation and not a refutation.

## 48.3 The registration form, and why it is not taken

`defer` (Go, Swift, Zig) answers the same question by attaching cleanup at the
*acquisition* site rather than to a region. The ergonomic pitch is real —
acquire and release become adjacent lines, so neither can be forgotten and a
reader sees both at once — and it is why Go chose it: Go has no region worth
naming, so a region form would wrap nearly every function body.

Two known costs, and one that is mesa's alone.

**Scope granularity.** Go's `defer` fires at function return, so registering one
inside a loop accumulates until the function exits. Every Go style guide warns
about it. Swift scopes `defer` to the enclosing block and does not have the bug.
A region form cannot have it, because the extent is written out.

**Dynamic registration.** N cleanups per frame, run LIFO, is a growable per-frame
structure, and frame teardown stops being free.

**And the disqualifier: §30.2.** If `defer`'s operand were a proc literal, capture
by value means the deferred body observes a *snapshot*, with no way to spell the
other behaviour. Go offers both — `defer f(x)` evaluates arguments at
registration, `defer func(){ … }()` observes final state — precisely because Go's
closures capture by reference. Mesa would inherit Go's confusing rule minus the
escape hatch that makes it survivable.

**The repair generalises, which is why it is recorded here even though the form
is declined.** Make the operand a *block* rather than a proc: evaluated later in
the enclosing frame, reading live locals, no capture involved. Under the arena
that is cheaper than Go's version, not dearer — a registered cleanup is a
`BlockId`, so the frame's list is a `Vec<BlockId>` holding no `Val`s, keeping
nothing alive and leaving §30.3's "no local is ever boxed" untouched.

This is the third instance of one pattern: **when mesa needs a delayed body, it
wants a block in the current frame and never a closure.** §20.8 reached it for
iteration, making `each n in xs do … end` surface syntax over a pull protocol so
the body stays in the enclosing frame. §41.4 depends on it, since a body that
never crossed a proc boundary keeps `break`. And it is what a `with` body would
be. Stating it as a rule makes several future decisions automatic.

`defer` is declined on one-mechanism grounds rather than on cost. Mesa is
building `rescue`, so a region exists whether or not cleanup uses it; the
registration form would be a second, parallel cleanup mechanism with its own
scoping rules for the same job.

## 48.4 What is decided

1. **Cleanup is a region form**, spelled `ensure`, and it is a clause on
   whatever region `F2` establishes rather than a construct of its own.
2. **`return` and `break` inside an `ensure` body are a `sem::Error`.** See
   §48.5.
3. **It runs on all four exit paths**, including the top-level early exit
   (`Interpreter::eval`, where a `Signal::Return` is an exit rather than an
   error) and including the fatal path, where nothing catches the error at all.
   The fatal path is the one that matters most for descriptors.
4. **The body's value is discarded**; the construct evaluates to the protected
   region's value. Worth stating outright in a language where everything else is
   a value.
5. **Rescue arms run first, `ensure` last**, on every path — including the path
   where an arm matched and the path where none did.
6. **It does not force reification.** §19.5 keeps `Signal::Error` propagating a
   Rust-side representation until a rescue arm matches; an `ensure` never
   inspects the error, so the zero-allocation fatal path survives. This is a
   property of the design, not an accident of the first implementation, and it
   is what §48.7's second open question must not spend.

One implementation trap, recorded because it is in no other section. The trace
`Signal::Error` now carries is built by pushing a frame at each
`eval_proc_call`. An `ensure` body running mid-unwind makes its own calls, which
must not push onto the in-flight trace — save and restore, or run the body
against a fresh one.

## 48.5 `return` in cleanup is the settled part, and everyone else regrets it

Cleanup runs *during* an unwind. If the cleanup itself exits non-linearly there
are two control-flow actions in flight, and the later one wins by construction —
silently discarding the first.

```java
int f() {
    try { throw new RuntimeException("boom"); }
    finally { return 42; }   // the exception ceases to exist
}
```

Python and Ruby behave the same way with their own spellings. **The tell that
this is a design regret rather than a gotcha is that it is correct by
specification and universally linted against.** JLS 14.20.2 states the rule
deliberately — the finally block's abrupt completion replaces the try block's —
and then `javac -Xlint:finally`, SpotBugs, ErrorProne and Checkstyle all exist
in part to stop you relying on it. Python added a `SyntaxWarning` for
`return`/`break`/`continue` leaving a `finally` (PEP 765, 3.14) with the stated
intent of making it an error; Ruby warns on `return` inside `ensure`.

**Both languages that revisited the design banned it, one in each form.** C#
makes leaving a `finally` by `return`, `break`, `continue` or `goto` a compile
error (CS0157), and has since 1.0 without anyone finding it restrictive. Swift
makes transferring control out of a `defer` body a compile error. Mesa is in the
second camp.

The failure is also the worst available class. It is silent, it destroys the
diagnostic rather than producing one, it is almost always a refactor artifact
rather than intent, and it lives on the error path, which is the least-tested
code in any program.

**In mesa the check is nearly free and closes a hole besides.** `sem` already
walks bodies for `L10`'s loop-depth counter (§41.5), so this is a flag on an
existing traversal. And it disposes of an interaction `L10` would otherwise
leave undefined: the depth counter resets at every *proc* body, and an `ensure`
body is not one, so a `break` inside an `ensure` would pass the counter while
naming an enclosing loop that may already be unwinding from a different `break`.
Two breaks, one loop, no defined answer. Banning is the rule that keeps the
question from arising.

## 48.6 What this does not close

**§4.15 cl. 7 stays open.** An `ensure` does not give a resource-holding
iterator a cleanup hook on `break`, because under pull the loop belongs to the
interpreter and the iterator's state object has nowhere to hang a region.
Closing cl. 7 needs either a `with`, or `each` itself calling a release verb on
every exit path. Recorded so the row is not assumed closed by this section.

**No destructors remains the position.** Nothing here adds finalisation, and
refcount-driven release stays what §47's native types get by default. `ensure`
gives a *lexically scoped* release for code that asks for one; it does not make
release automatic.

**`with` is not foreclosed and would be additive.** A new construct plus a
protocol changes no existing program's validity. If it is ever built, §48.7
records what it would have to decide.

## 48.7 Open

Two of these block building; the rest are spelling and scope.

1. **What region does a `rescue`, and therefore an `ensure`, protect?**
   *Blocking, and it is `F2`'s question rather than this section's.* Three
   candidates: a `def` body only, as Ruby's method-level form; a new block
   expression, which makes sub-region protection free; or the remainder of the
   enclosing block. The first spends no construct but makes per-iteration
   cleanup impossible without extracting a proc — which, since a literal's
   `return` is literal-scoped (§16.4), changes the meaning of any `return`
   moved into it. Worth deciding with resources in view and not only errors.

   A spelling problem rides along. §6's opener rule — `then` introduces a
   consequence, `do` introduces repeated action — fits neither a protected
   region nor a `with` body, so both forms hit the same wall and neither has an
   obvious word.

2. **What happens when the `ensure` body itself raises?** *Blocking, and the
   thing to settle first, because it cannot be rejected statically the way
   clause 2 of §48.4 rejects the others.* The in-flight error is either dropped
   or chained.

   **This is the same shape as §4.17 cl. 12** — a caught error stored and
   re-raised gets the re-raise site, recorded there as deferred residue. Two
   instances of one problem: *an error with more than one interesting site,
   where the naive representation keeps one.* They should be decided together
   rather than separately, and solving the shape once is likely cheaper than
   two ad-hoc answers.

   Java is the cautionary case and the reason to decide now rather than later.
   It dropped the information; try-with-resources then made the collision
   routine; and Java 7 had to add `Throwable.getSuppressed()` to record what the
   original design had discarded. Mesa would be *choosing*, with both traces
   already in hand at the moment of collision. Whatever is chosen must not spend
   §48.4 clause 6 — the fatal path costs no allocation today, and chaining must
   not change that for errors nothing catches.

3. **Is a bare `ensure` legal with no rescue arms?** The resource case wants it
   — cleanup with no handler is the common shape — but it depends on clause 1's
   answer, since a region form with no arms may or may not have anything to
   attach to.

4. **The keyword budget** (§9). Twenty-six words today, twenty-eight with
   `raise` and `rescue`, twenty-nine with this. §16.6's reasoning for reusing
   `def` applies: if an existing word can carry it, that is worth more than the
   clarity of a new one.

   *Cheaper than it looks, per §53.* A cleanup marker sits where `rescue` sits —
   interior to a construct, never leading an expression — so it costs the bare
   form only, and `x.ensure` and `type T(ensure)` are recoverable. That weakens
   the argument for reusing an existing word here rather than strengthening it.

And if `with` is ever revisited, six questions this section did not have to
answer:

| | |
| --- | --- |
| One protocol member or two | Python's `__enter__`/`__exit__` exists because `with` rebinds its result; a construct binding what the expression already produced needs only the exit, as C#'s `IDisposable` does |
| May the exit see the error, and suppress it | Suppression is a second error-handling mechanism competing with `rescue`, and passing the error forces §19.5's reification — so probably not, on both grounds |
| How the interpreter reaches the protocol by identity | A language construct dispatching on a user-declared protocol needs it at a fixed location — the same seam `D7`'s iteration protocol needs, so `with` is not as dependency-free as it looks |
| Whether `each` participates | Determines whether §4.15 cl. 7 closes. `with` alone closes only the explicit case |
| Multiple resources per construct | §6.1's pyramid objection applies with full force — every nested level adds an `end` |
| The binding form | `with f := File.open(p)` reuses `:=` and matches `each n in xs`, spending no keyword, against Python's `as` |

One structural note in `with`'s favour, for whoever picks it up: it **cannot
have §48.5's bug**. There is no user-written cleanup block, only a cleanup
method, and a method's `return` returns from itself. The question does not arise
rather than being banned — which is the same reason a block-bodied construct
wins on the other side, seen from the opposite direction.

# 49. Containers compare, and do not hash

*The protocol is `Equal`, per §39; §24.6 and §24.7 predate the naming and say
`Eq`, as `D4`'s roadmap row still does.*

§24.6 scopes structural `Equal` to *user types* and §24.7's examples are all user
types, which reads as though built-in containers are a separate question `D4`
can leave alone. They are not. §26.4 cl. 7 has body fields participating in
structural `Equal`, and a field holds whatever it holds — so the first user type
carrying a list forces an answer about lists. `D4` cannot avoid deciding this;
it can only decide it implicitly.

**Decided: `List` and `Dict` compare structurally, and are unhashable.**
`[1, 2] == [1, 2]` is true; `d[[1, 2]] := 1` raises. This is Python's
arrangement, which §24.7 already cites from the other side — "Python's
containers avoid the hashing question entirely by being unhashable, and pay for
it on the comparison side instead." Mesa pays the same way, and §24.7's
coinductive equality is the payment already designed.

## 49.1 The bound must be a function of the equivalence class

§24.7 cl. 11 says the hash bound "may not read identity or reentry," derived
from the case in front of it: coinductive equality makes the self-loop equal to
the two-cycle, so a traversal noticing reentry can split them. That is correct
and it is not the general rule. **The general rule is that the bound may read
only what equality can see** — the value's equivalence class — and identity is
one thing outside it. Ruby obeys this rule for its own containers, by a
mechanism worth seeing in full (§49.7).

Insertion order is another, and it is the one that bites. Suppose dict equality
is order-insensitive, as Ruby's and Python's are. Then:

```
d1 := {"a": 1, "b": 2, ... "z": 26}    # inserted a..z
d2 := {"z": 26, ... "b": 2, "a": 1}    # inserted z..a
```

are equal, and a traversal bounded at ten nodes consumes `a..e` from one and
`z..v` from the other. A commutative fold does not save this. Commutativity
handles entries arriving in a different order; it does not handle **different
entries surviving**. The two hashes differ, equal values hash unequally, and a
`Dict` keyed on either silently misses.

So cl. 11 needs restating as the principle rather than the instance. Nothing
about the clause is wrong — it is narrower than the thing it is an instance of,
and the dict case is not covered by the sentence as written.

## 49.2 Canonical truncation of an unordered collection is impossible

This is what closes the question, and it is not a matter of finding a better
mixer. Any truncation of a dict's entries must choose a subset by a rule reading
only the entries themselves, since anything else reads insertion order. To apply
such a rule — smallest entry-hash, first *k* in key order, any rule at all — you
must first compute it for every entry. **Selecting a canonical subset costs a
full traversal, which is the thing truncation existed to avoid.**

The options are therefore all-or-nothing, with no middle:

| | Cost | Collisions |
| --- | --- | --- |
| Every entry, folded commutatively | O(n) in entries | ordinary |
| No entries — hash the type tag and `len()` alone | O(1) | every same-size dict |

There is no third row. Note the consequence for §24.7 cl. 10, which justifies
bounding partly because it "keeps hashing a large collection from being O(n)":
that claim is true of ordered structure and false of unordered structure, and
no hashing scheme makes it true of both.

## 49.3 Why unhashable, rather than paying the O(n)

The commutative fold works. It was declined for a reason that is not cost.

§4.12 cl. 3 makes `Hash` opt-in **so the mutable-key hazard is visible at the
declaration** — a type usable as a key said so in its own source. Built-in
containers have no declaration site. Making them structurally comparable forces
them to be structurally hashed (equality and hashing cannot disagree in either
direction), and structural hashing of a mutable container reintroduces exactly
the hazard cl. 3 exists to surface, in the one place no `impl` line can warn
about it. Today `val.rs:180` hashes every `Obj` by pointer, so mutating a list
after using it as a key is safe; the fold would end that quietly.

Unhashable containers keep cl. 3's principle intact without an exception, and
they delete §49.1 and §49.2 from the implementation entirely — an unordered
collection never enters a hash traversal, so the hash stays the counter §24.7
describes, over ordered structure only. **§4.13's hashing half needs no
amendment.** It needed one under the fold.

*Declined for completeness:* freezing a container on use as a key. It buys the
same safety and costs a mutability check on every write to every list, forever,
to serve a case §49.4 shows is rare.

*What would reopen this.* Not a cheaper hash. A hypothetical O(1) structural
container hash leaves the argument above exactly where it is, because the ground
is visibility and not cost — and §49.7 finds the mechanism is available already,
correctness and all, at a collision class that could be fixed. Three things
would reopen it. Changing §4.12 cl. 3, so that key-ability is no longer promised
visible at the declaration, removes the ground entirely. Carving an exception for
built-ins keeps the promise for user types and abandons it for containers, which
is the exception this section exists to avoid having to make. Or the freeze above
stops being too expensive — the only one of the three that makes the hazard
impossible rather than merely visible, and the one whose arithmetic flips if the
case §49.4 calls rare ever stops being rare.

## 49.4 What it costs, and where the check lands

A user type declaring `impl Hash` while holding a list or dict field cannot be
hashed. **This raises at hash time, not at declaration** — mesa is dynamically
typed, so nothing at the declaration knows what a field will hold.

That is an asymmetry with §24.6, whose `Equal`/`Hash` coupling is explicitly
"checked at declaration, with no runtime cost," and §13.4's habit of putting
protocol checks where the declaration is. It is not a departure from either so
much as a case neither anticipated: the check is not about the *declaration*
being wrong, which is statically decidable, but about a *value* being
unhashable, which is not. `D8`'s `ProtocolError.NotImplemented` is the carrier.
Python raises `TypeError: unhashable type: 'list'` at the same point for
`hash((1, [2]))`, so the shape is well-trodden.

The residual question — whether a dict should be usable as a dict key at all —
is answered no, and cheaply: it was never a use anyone wanted badly enough to
buy §49.2's O(n) with.

## 49.5 What this makes reachable earlier

§24.7 closes with "none of this bites today… `rt.rs` makes only `Str`
structural," and §0.2's `D5` row turns that into a hard edge — `D5` unreachable
before `D4`. **Container equality retires that edge.** A self-referential list
needs no user type:

```
a := []   a << a
b := []   b << b
a == b
```

so `D5`'s pair-set must cover `Obj::List` and `Obj::Dict` from the moment
containers compare, which is a change touching neither protocols nor `impl` nor
any declaration-time check. Cl. 4's "only `Obj` pairs ever enter" already
describes this correctly; what moves is the trigger, not the algorithm. The
coinductive machinery is now exercisable against the simplest values in the
language rather than waiting on the largest feature that needs it.

Two smaller consequences. `D6`'s cycle marker acquires the same earlier trigger,
since a self-referential list becomes printable before any user type is
structural. And **the dict order-sensitivity question decouples**: with hashing
out of the picture it is a pure equality decision, carrying none of §49.1's
constraint. `ordermap`'s own `PartialEq` is order-sensitive — that is the
crate's distinction from `indexmap` — so order-insensitivity costs a
hand-written comparison. Ruby and Python both treat insertion order as an
iteration guarantee and not an equality distinction, which is the better
precedent, but the choice is now free either way and `D4` can defer it.

## 49.6 The cost that survives regardless

Independent of all of the above, and worth stating because it is `D4`'s real
implementation work rather than its rules: once any user type can be a dict key,
`impl PartialEq for Val` (`val.rs:157`) and `impl Hash for Val` (`val.rs:173`)
stop being expressible as trait impls. Coinductive equality needs the pair-set
threaded through, an overridden `Equal` needs to call a user proc, and calling a
user proc needs `&mut Interpreter` and can raise. Both become interpreter
methods returning `Result<_, Signal>`, and `Dict`'s `OrderMap<Val, Val>` can no
longer use the derived traits for its own lookups — likely a precomputed `u64`
stored per key, with collisions resolved interpreter-side. Unhashable containers
shrink the surface this applies to; they do not remove it.

*One adjacent gap, on the same invariant and belonging to no row.* `val.rs:160`
compares `Num` with `f64` equality, so `NaN != NaN`, while `val.rs:176` hashes
`to_bits()` consistently. A `NaN` key can be stored and never found, and
assigning to it twice grows the dict. Pre-existing, orthogonal to `D4`, and
unrecorded until now.

## 49.7 Ruby's detection is correct, and its collisions disqualify it

§24.7 left a suspicion on the record and marked it unverified: that Ruby's
`Array#hash` might disagree with `Array#==` on cyclic arrays. **It is false.**
Read against `ruby/ruby` master and probed against a live interpreter, Ruby
satisfies the contract — by a mechanism assembled from three pieces, none of
which lives in `Array#hash`.

**`Array#hash` has no cycle handling at all.** `rb_ary_hash` (`array.c`) folds
every element through `rb_hash`: unbounded, order-sensitive, no seen-set and no
counter. `Hash#hash` is the same shape with `*hval ^= st_hash(...)` per entry,
commutative and so order-insensitive, matching its `==`. Neither truncates, so
Ruby sits in §49.2's top row and pays the full O(n) that §49.3 declines.

**The guard is generic, one level down.** `rb_hash` falls through to
`obj_any_hash`, which routes any non-default `hash` through
`rb_exec_recursive_outer_mid`, whose sentinel is a bare constant —
`if (recurse) return INT2FIX(0);`. Nothing about the re-entered object, not its
length and not its type, reaches the result.

**One flag decides whether the rule is obeyed.** In `exec_recursive`
(`thread.c`) the first guarded call marks itself outermost, and a reentry
beneath it does `rb_throw_obj`, longjmping past every intermediate frame to that
outermost one, which then yields the constant. Under `outer = 0` — the variant
Ruby uses for `inspect` — the sentinel would land *in place* and the hash would
read the depth at which the back-edge closed, which is precisely the violation
§24.7 predicted. The same helper serves equality as `rb_exec_recursive_paired`,
whose `if (recur) return Qtrue;` is this section's pair-set in one line.

So the hash reads the acyclic frontier above the first guarded object, plus
whether a cycle exists below it — and cyclicity is class-invariant, a finite
unrolling never equalling an infinite one. Both are things equality can see.
**§49.1's rule is confirmed by a language obeying it**, not contradicted, and
the back-edge depth that §24.7 feared is erased by the throw rather than by a
bound. Probed directly: the self-loop and the two-cycle hash alike, and so do
`[1, x]`-shaped values whose back-edge closes one level apart.

**What disqualifies it is the collision class.** Because the throw unwinds to
the outermost guarded frame rather than to the cycle's own root, a cycle
anywhere annihilates every acyclic sibling folded above it. Verified:

```ruby
u = []
u << u
[[5, u]].hash == [[6, u]].hash    # => true
```

The `5` and the `6` are hashed, folded in, and then discarded by the longjmp.
Any value carrying a cycle beneath its first non-immediate element collapses to
a hash of that element's position and the enclosing lengths. This is not a bound
trading collisions for cost in the way §24.7's OCaml citation describes — it is
O(n) **and** coarse, which is the worst corner of that trade and not a scheme
mesa would want even if containers were hashable.

**Where a better version would collapse.** The collision class is a property of
the collapse *point* rather than of detection as such, and Ruby picks the wrong
one. Collapsing at the entry to the value's strongly-connected component, instead
of at the outermost guarded frame, keeps the depth-erasure that makes the scheme
correct while confining the damage to the cycle itself: the self-loop and the
two-cycle still agree, each being its own SCC entry and both yielding
`f(len, const)`, but `u` alone is the SCC in `[[5, u]]`, so the enclosing pair
folds normally and the `5` survives. SCC membership is a property of the
unrolling, so §49.1's rule should still hold. This is recorded as the shape a
re-evaluation would start from (§49.3), not as a result — it has been neither
implemented nor proved invariant.

**The correction is therefore narrower than it looks.** The suspicion is struck
and §24.7's "reason to take OCaml's bound rather than Ruby's detection" is
replaced: detection is not incorrect, it is expensive and imprecise. Nothing in
§49's decision moves — §49.3 declines the fold on the visibility grounds of
§4.12 cl. 3, which never depended on cycles or on cost.

*Verified against `ruby/ruby` master, so line numbers are deliberately omitted;
the older arrangement carried a `recursive_hash` inside `array.c` and reached
the same place.*

---

# 50. Re-exports (open)

§46.9 opened cross-package import of *types* rather than modules — `import
Core.Order` yields the module, and the protocol inside it is
`Core.Order.Order` — and called the shorter spelling "a convenience question
the architecture does not constrain." It is the first concrete thing §38.5's
refusal costs, and the refusal deserves a fuller hearing than the one clause it
has. **This section decides nothing.** It records what a re-export would look
like, what each spelling costs, and what test would settle it.

## 50.1 The refusal is on record twice

§38.5, as `§4.18` cl. 27, declines deep paths in an export list: `export
Codec.decode` from the root "is a re-export — a *binding* form rather than a
list — and would let `JsonRpc.decode` name something declared nowhere near the
root, with two mechanisms jointly deciding its meaning."

§42.4 then uses the same word for a hazard rather than a feature. If
`Obj::Module` were a handle to the module's `Scope`, an `import Utils` at the
head of `codec.ms` would make `Codec.Utils` readable from outside — "an
accidental re-export, which cl. 27 declines by name when it is written
deliberately." So the member map is separate from the scope partly to keep
re-exports from happening by accident, and any deliberate form has to be
written *into* that map rather than falling out of it.

**The refusal is cheap in mesa for a structural reason.** Cl. 23's member
namespace is flat and shared — a module's own top-level declarations plus its
child modules — so an export list can always be checked against a set the
declaring file itself defines. A re-export is the first construct that would put
a name in that set which the declaring file does not declare.

## 50.2 What the refusal costs: the public surface is the file tree

§38.5's motivation for having export lists at all is that a distribution
boundary where everything is public means "a library ships its internals as API
and any refactor of any file is a breaking change to consumers."

**Without re-exports that is half-fixed.** Internals can be hidden and then
refactored freely; anything deliberately *exposed* is pinned to its on-disk
location forever, because the public path is the layout. Moving `decode` from
`codec.ms` to `codec/frame.ms` is a breaking change to every consumer, though
the package intended `decode` to be public and never intended `Codec.Frame` to
be. Re-exports are what decouple a package's public surface from its file tree,
and §46.9's stutter is the small end of that; the refactor case is the large
one.

This is the argument to weigh against everything below. It is not an argument
about convenience.

## 50.3 Five forms

**1. The deep path** — the spelling cl. 27 declines by name.

```
module JsonRpc
export call, Client, Codec.decode
```

No new keyword, and the export list keeps its position and its punctuation. Its
problem is cl. 27's: nothing at the *definition* site records that `decode` has
been lifted, and `JsonRpc.decode`'s meaning is decided jointly by the root's
export list and by `codec.ms`'s declaration.

**2. The prepositional form.**

```
module JsonRpc
export call, Client
export decode, encode from Codec
```

**This form and form 1 name the same two things** — a member, and the path that
owns it — and both put that evidence in the same file, on the same line, at the
same distance from the definition site. The difference is word order, plus one
marginal ergonomic: `export decode, encode from Codec` distributes over a list
where the dotted form repeats the prefix on each name. §50.4 prices the keyword;
the point to carry there is that form 1 obtains the same information for free,
so whatever `from` is bought with is bought for word order and list
distribution, and not for expressiveness.

Three costs belong to this shape specifically rather than to re-exports.
*A new resolution context*: `from Codec` must resolve `Codec` somewhere, and it
cannot be scope, since cl. 22 gives the root only the prelude, its own
declarations, and its imports — `Codec` is not among them. Resolving it against
the file's *members* is coherent with cl. 23 and needs no import, but it makes
this the first construct in the language that resolves a name in the member
namespace of the file it is written in; requiring the full `from JsonRpc.Codec`
avoids that and stutters the module header back at itself on every line.
*Cl. 24 stops being true as written* — "the form is a list at the head of a
declaring file" becomes two statement forms, and the currently-unasked question
of whether a file may carry several `export` lines has to be answered.
*It is where aliasing pressure lands*: the prepositional shape reads as an
invitation to `export parse from Codec.decode`, and §38.6 keeps aliasing out of
the language deliberately — `as` lives in the manifest, "the only aliasing
mechanism in the language, and it is not in the language." Form 1's dotted path
has no slot to put an alias in.

**3. Exporting an imported name** — the smallest change to the existing rules.

```
module JsonRpc
import JsonRpc.Codec.decode
export call, Client, decode
```

The export list keeps its shape entirely; what changes is cl. 23's definition of
what may appear in it — *own declarations plus child modules*, plus *names you
imported*. A reader of `package.ms` sees the full path of everything the file
re-exports without leaving the file, and the manifest's `as` composes with it
untouched.

Two costs. It makes §42.4's separation policy rather than structure: members
become declarations plus children plus a chosen subset of imports, so the loader
can no longer build the member map by ignoring the import list. And it
re-exports *and* binds — the name enters the root module's own scope, where it
is indistinguishable from a local declaration in the root's body. That side
effect is the one thing a dedicated form (1 or 2) avoids, and it is the strongest
argument for having one; note that the win belongs to *any* dedicated form, so
it is not evidence for form 2 over form 1.

**4. The value alias, which the design already permits.**

```
module JsonRpc
import JsonRpc.Codec

decode := &Codec.decode
export call, Client, decode
```

§44.3 records a member key for "the left-hand side of `x := …` as much as a
`type`, `proto`, or `def` name," so `JsonRpc.decode` is already a legal member
reached by a legal path, with no amendment to anything. **The question is
therefore narrower than whether mesa should have re-exports: it has one, and the
question is whether to promote it from an evaluated binding to a declaration.**

Three things make it unsatisfying rather than a solution. It is a top-level
*evaluated* binding, so it is order-dependent and it makes the root's top level
name its child — which converts §38.2's "containment is not a load-time edge"
into an edge, and arms precisely the cycle §38.2 flags as the first thing an
author reaches for. It needs `&` to avoid invoking (§14.8), which is a papercut
on a line that reads as declarative. And it is unclear that it reaches the case
that reopened the question: `Order := Core.Order.Order` binds the protocol as a
value, and whether `impl Order` then resolves through that binding is a §36
identity question rather than a naming one — if it does not, the alias form
handles everything except §46.9's own example.

**5. The implicit rule** — §46.9's other option, that a single-member module
exposes its member.

No syntax at all, and it dissolves the stutter exactly. It is also the only form
here whose behaviour is triggered by a *count*: adding a second member to a
stdlib module would silently change what `Core.Order` means at every use site in
every consumer. That is the action-at-a-distance §28.4 exists to refuse, applied
to a package's published surface rather than to one file's names.

## 50.4 The keyword budget, applied to `from`

All of mesa's keywords are fixed `Sym` constants assigned in `Interner::new` and
lexed unconditionally; there is no contextual keyword anywhere in the language,
so a word that is a keyword in one position and an identifier elsewhere would be
a new mechanism, and the lexer would stop being what decides what a word is.
`from` would therefore be a real keyword, and §20.5's rule applies at full
force: every keyword is a member name spent permanently.

*The premise is wrong in its second half, and §53 works it out.* The mechanism
is a widening in the parser, not a context-sensitive lexer: `lex_ident` sets
`sym: Some(sym)` on keyword tokens too, so the identifier is already carried and
the lexer keeps deciding exactly what it decides now. The consequence for this
section is direct — **the middle row below is the one that is answered**.
`type Edge(from, to)` is a parameter list, which is a closed position, so `from`
is declarable under the weak version of §53.3; and `from` in `export decode from
Codec` appears in no expression-position construct, so none of §53.4's
structural objections reach it. The comparison against `in` stands on frequency,
but the "worse casualty than any word currently spent" verdict does not.

§20.5's casualty table is the thing to apply, and it recovers less than it
looks:

| | `from` as a keyword |
| --- | --- |
| `msg.from` after a `.` | recoverable in principle; §20.5 declined the fix |
| `type Edge(from, to)` — declaring the field | **not declarable at all** |
| bare `from` in a method body | not reachable, per §3.2's implicit `self` |

**`from` is a worse casualty than any word currently spent**, and plausibly worse
than `next`, which §20.5 named as a recurring loss and §24.5 later refunded.
`from`/`to` is the canonical field pair for ranges, edges, transfers, messages,
date spans, and diffs — the record-shaped code mesa is aimed at. And §20.5's
refund history is pointed: both refunds came from *deleting a feature*, never
from finding a cheaper spelling. A re-export feature would not be deleted later,
so this cost is permanent in the way §20.5 means.

**Mesa already has a preposition**, so the objection is not grammatical taste:
`in` is spent, for `each x in xs`. The asymmetry is what each buys. `in` is paid
for by a construct written in nearly every program; `from` would be paid for by
a line written a handful of times per *package* — about as infrequent as a
construct gets, which is the measure §31.6 uses for length and §20.5 uses for
existence.

**One escape costs nothing**: reuse the word already paid for.

```
export decode, encode in Codec
```

It reads worse. Whether a spent word may be reused in an unrelated grammatical
role is a question this section does not answer, but it is the only version of
form 2 that leaves the budget where it is.

## 50.5 What a re-export costs the mechanisms

Independent of spelling, three things move, and they are the implementation
weight rather than the rules:

- **Reachability stops being a path property.** Cl. 26 is clean: a name is
  reachable from outside iff every segment on its path is exported by its
  parent, up to `src/package.ms`. A re-export creates a second path whose
  intermediate segments may be unexported deliberately — that is the point of
  lifting `decode` while hiding `Codec` — so reachability becomes graph
  reachability, and cl. 31's fourth check ("an import fails if any segment on
  its path is unexported") is rewritten rather than adjusted.
- **The member map gains a reference kind, and with it a second ordering.**
  §44.2 already made members `MemberDesc` rather than `Val`. A re-export is a
  description that points at *another module's* member, so resolving the map
  becomes a small fixpoint over names, with its own acyclicity check, distinct
  from §42.3's state DAG. §42.3's "two jobs and no third" for the import graph
  acquires a third job one grain over — over names rather than over state.
- **Duplicate-member detection widens from tidiness to a real check.** Cl. 23
  says the duplicate check "fires essentially never, since procs are lowercase
  and modules capitalized." Re-exports are exactly how two names arrive at one
  module from two elsewheres, so the check starts firing in practice and its
  message has to name both origins rather than one site.

## 50.6 What it does not cost

Worth retiring early, so they take no room in the decision. **Identity is
unaffected**: §38.1's once-per-package evaluation means both paths yield the
same value, and §28.1's identity equality answers correctly, exactly as it does
for a diamond. **The type-member rule is untouched**: cl. 24's principle is that
visibility governs the reachability of *names* and never what you can see of a
value you can already name, and a re-export is a name question throughout.
**The manifest is untouched**: `as` renames a dependency root, a re-export
renames nothing, and the two do not meet — provided no form acquires an alias
slot, which §50.3's form 2 is where that pressure appears.

## 50.7 The test that would settle it

§28.4's own test is the one that discriminates: *can a reader tell where a name
came from?* All five forms are static, so none of them fails it the way a
computed module name does. What separates them is **which file must contain the
evidence** — the definition site, the re-exporting file, or neither:

| | evidence lives in |
| --- | --- |
| 1, 2 — dedicated export forms | the re-exporting file, on the export line |
| 3 — export an imported name | the re-exporting file, on the import line |
| 4 — value alias | the re-exporting file, as evaluated state |
| 5 — implicit single-member rule | nowhere; it is a fact about a count |

None of them puts it at the definition site, which is cl. 27's actual objection
and is not answered by any spelling here. So the question is probably not
"re-exports, yes or no" but whether cl. 27's objection survives the §50.2 cost —
and if it does, whether form 5's absent evidence or form 4's load-time edge is
the cheaper thing to live with.

## 50.8 What this leaves open

Everything. Nothing here is decided, and no row moves.

The specific open questions, in the order they would have to be answered: whether
§50.2's refactor cost is worth a mechanism at all; if it is, whether the evidence
belongs at the definition site, which no form currently supplies; whether a
dedicated form's membership-without-scope is worth the amendment to cl. 23 that
form 3 avoids; whether `from` clears §20.5's bar, or `in`'s reuse does; and
whether §36's protocol identity travels through a value binding, which decides
whether form 4 reaches §46.9's example at all and is the one question here that
is answerable by experiment rather than by argument.

**This has no row.** It has lived in §46.9's closing paragraph as a convenience
question; §50.2 is the argument that it is not one.

# 51. Protocol composition, reconsidered

§34.5 declined composition — one protocol requiring another, Rust's supertraits —
"for now, and explicitly revisitable," and named the condition for revisiting:
*re-examine if the protocol set grows deep.* This is that re-examination, taken
before the set grows rather than after, because the asymmetry §34.5 priced runs
the other way in time.

§34.5's reversibility argument is about **program validity**: adding composition
later is additive, removing it is a reverse ratchet, so waiting is free. That is
still true, and it is still the right frame for the decision. What it does not
cover is **work**. The stdlib refactor §34.5 acknowledges as real grows with
every restated signature `H2` writes, and the mechanism costs the same to build
before that as after. Waiting preserves the option and accrues the bill.

## 51.1 The form

`impl` at the head of a `proto` body, in the position a `type` body already
carries it:

```
proto Hash
    impl Equal
    def hash end
end
```

Position is the whole rule, as it is for `type` (`B14`): the grammar admits one
`impl` line at the head of the body, and a later one does not parse. The clause
takes the same comma-separated dotted paths and resolves them the same way, by
§45's relative resolution with no descent into types.

The reading survives §39.1. `impl X` means "supports being X'd"; inside a
protocol it says that anything supporting being `Hash`'d supports being
`Equal`'d — a statement about a protocol rather than about a type, but the same
shape, with the same object.

## 51.2 Implication, not conjunction

§34.5's second preservation is promoted here from a migration convenience to the
definition of the feature: **`impl Hash` implies `impl Equal`.** A type naming
the derived protocol conforms to the prerequisite without naming it, and `x is
Equal` — when `L7` supplies it — answers yes.

The weaker reading, that `proto Hash impl Equal` merely *requires* an
implementing type to also write `impl Equal` and errors if it does not, is
rejected. It costs the same machinery and buys none of the collapse, since every
implementing type still restates every prerequisite. It also inverts the
preservation's purpose: the restated signatures move from one protocol
declaration to every implementor, which is worse than the status quo rather than
better than it.

## 51.3 The closure is the mechanism

Everything below reduces to one artifact: **each protocol's transitive
prerequisite set, computed once at its declaration and stored flattened.**

Three obligations become lookups against it.

1. §34.10's required-member rule reads against the **effective** required set —
   §34.5's third preservation — which asks whether *anything in the closure*
   requires a member, not whether this protocol does.
2. Conformance is decided against the effective member set, so `impl Hash`
   checks `Equal`'s required verbs too.
3. §31.3's conformance test is transitive, because the closure is flattened into
   the implementing type's own conformance record rather than consulted at query
   time. That is what keeps the interpreter out of it entirely (§51.8).

## 51.4 Cycles are the one genuinely new error

`proto Hash impl Equal` beside `proto Equal impl Hash` is newly writable and must
be rejected — not because it is ambiguous but because the closure does not exist.
A declaration-time error, in the family of §28.4's acyclic module rule and for
the same reason: a cycle is not a resolution problem to be settled by a tie-break,
it is a statement with no fixed point.

The check is confined to a package. Package dependencies are already acyclic
(§46), so a prerequisite edge crossing a package boundary cannot participate in a
cycle — every such edge points into a package fully described before this one was
begun.

This is the only item on the list that is new machinery rather than a widened
lookup, and it is the cheapest kind: a walk over a small graph with no data
flowing through it.

## 51.5 Diamonds deduplicate; they do not linearise

`type T impl Hash, Order` where both prerequisite `Equal` reaches `Equal` twice.
The answer is a visited set, and it is worth saying why that is sufficient rather
than the first step toward a method resolution order.

Linearisation exists to decide *which* of several bodies runs, and it is needed
when the answer is not already determined — when there is state to initialise in
order, or a `super` walking the chain, or a diamond whose two paths carry
different implementations. §34.3 gives a protocol no state, §34.2 declines
`super`, and a diamond here reaches the *same* protocol by two routes, so both
paths carry the identical body. There is nothing to order. Seeing `Equal` twice
and acting on it once is the whole of it.

Without the visited set the diamond is not merely redundant work: §34.6's
conflict rule fires and `T` is rejected for acquiring `Equal`'s members twice.
The deduplication is load-bearing, not an optimisation.

## 51.6 Precedence: nearer wins, unrelated still conflicts

§34.6 was worded against two **unrelated** protocols providing the same name,
with "related" left undefined because it was impossible. §34.5's first
preservation is now spent: relatedness is prerequisite reachability, and the rule
splits in two.

- **Related** — one protocol lies in the other's closure. The nearer body wins.
  This is §34.6's override rule applied one link outward: a derived protocol
  supplying a body for a verb its prerequisite also provides is refining it,
  exactly as a type supplying its own member refines what it acquires.
- **Unrelated** — neither reaches the other. Unchanged: a declaration-time error,
  still resolvable by the type supplying its own member.

Nearer-wins is not a linearisation, on §51.5's grounds and one more: it is decided
**per verb, between two protocols standing in a reachability relation**, never by
position in a list. Reordering a type's `impl` clause cannot change which body it
acquires. That is the property §34.2 actually wanted, and it survives intact.

The implementation falls out of the traversal rather than needing a rule of its
own — a prerequisite-first walk emits farther protocols before nearer ones, so
the nearer body is simply the later write.

## 51.7 The fourth preservation, unrecorded until now

§34.5 lists three preservations. There is a fourth, and it is the only one not
already paid for: **§34.4's reach check must widen to the effective member set.**

A provided body may touch the protocol's own members through `self` and nothing
else. That rule is what makes a protocol checkable at its own declaration site
independent of any implementor, and it is the whole of §34.4's value. But the
motivating case for composition is a derived protocol's provided body calling a
prerequisite's required verb — which under the check as written is a reach past
the protocol's own declaration, and is rejected.

Widening it does not weaken the guarantee. The check exists so that a protocol
never reaches state or members it has no contract for; a prerequisite's required
verbs are members it has a contract for, transitively and statically. The
self-containment argument is unchanged — the body is still checkable at its own
declaration site, against a set still closed and still computed from declarations
alone.

Recorded here rather than folded into §34.5 because it is the one that would
surface during implementation rather than at design time: the mechanism would
build, the tests would pass, and the first derived body to use a prerequisite's
verb would be rejected.

## 51.8 The interpreter learns nothing

§31.3's conformance test is a membership check against a flat set of protocol
identities recorded on the type. If the closure is flattened into that set when
the type is described, `impl Hash` records both `Hash` and `Equal`, and the test
answers transitively with no knowledge that protocols have prerequisites at all.

This holds for `extern` types too, since §47.7 routes a native type's conformance
through the same description — so a native `impl Display` picks up `Inspect` by
the same flattening, which is the motivating case for the built-ins.

That is the strongest argument for this shape. Composition is entirely a fact
about descriptions, and §44.1's split — the phase produces descriptions, the
interpreter constructs values — puts it wholly on one side of the line. No `Val`,
no change to `Obj::Proto`, no dispatch path, and no change to §36.1's boot-order
identity table.

## 51.9 What this closes, and what it opens

**Closes**, if taken: §34.5's declining. The restated required verbs in the
stdlib's protocol declarations collapse, and the duplicated default bodies §34.5
named as the real cost — a derived protocol reimplementing a prerequisite's
*provided* member over its own required one — stop being necessary. §34.6's
"unrelated" acquires a definition. §34.10's required-member rule acquires its
effective reading.

**Opens**: two errors, and both are small. The cycle of §51.4 is a shape a user
can now write and could not before, so it needs a message that names the cycle
rather than one edge of it — the same obligation `ImportCycle` already carries.
The duplicate of §51.11 is a shape a user could always write and that was always
rejected, badly; it acquires an honest message rather than a new prohibition.

**What it does not open is coherence.** §52 argues that properly; the summary is
that composition adds edges to the protocol graph and no new conformance sites,
and conformance sites are where coherence lives.

## 51.10 Why not requirement

§51.2 rejects the weaker reading in a paragraph, on the grounds that it costs
the same machinery and buys none of the collapse. That is true and it is not the
whole argument. Recorded at length here, after the mechanism rather than inside
the decision, for the same reason as §51.7: this is the question a reader is
most likely to reopen, and one line of the argument runs *against* §51.2 and
belongs on record rather than in a rediscovery.

**The two readings differ in one line of user source.** Under implication a type
writes `impl Hash` and conforms to `Equal` as a consequence. Under requirement
it writes `impl Equal, Hash`, and omitting `Equal` is a declaration-time error.
Everything else is common to both:

- The closure of §51.3 is needed either way. Requirement needs it *to check the
  requirement* — "you forgot `Equal`" is unsayable without knowing that `Hash`
  reaches `Equal`.
- §51.4's cycles are still an error, for the same reason: no closure, no check.
- §51.5's deduplication is still load-bearing. A type dutifully writing
  `impl Equal, Hash, Order` still reaches `Equal` by three routes, and §34.6's
  conflict rule still fires without a visited set.
- §51.7's widening is still required, or a derived provided body still cannot
  call a prerequisite's verb.
- §31.3 still answers `x is Equal` with yes, on every program valid under
  requirement.

So requirement is not a smaller feature. It is **implication plus a mandatory
redundancy check** — the same machinery less the splice, plus one error whose
only fix is to write down what the phase has already computed.

**The language split is about where bodies live, not about explicitness.** Rust
and Haskell require restatement: `trait Hash: Eq` does not supply
`impl Eq for T`, and `class Eq a => Ord a` does not supply `instance Eq T`. It
is tempting to read that as a considered stance against implicit conformance. It
is not. In both languages the impl block is *where the method bodies go*, so the
restated impl is a container holding real code and there is nowhere else to put
it. Two things confirm the reading: Rust applies implication freely at the use
site, where a `T: Hash` bound calls `Eq`'s methods with nothing restated; and
the residual redundancy is felt as a wart rather than a feature, which is what
the implied-bounds proposals are about.

Swift, Java, C#, Ruby and Go take implication. `struct S: Hashable` conforms to
`Equatable` and casts to it; a class implementing a Java interface satisfies
`instanceof` for everything that interface extends; Go, being structural, has no
declaration site at which requirement could even be spelled.

The line between the two camps is **whether a conformance has a body attached to
it or is only a name in a list**, and mesa is unambiguously the second. `impl`
is a comma-separated list of dotted paths on a declaration head; members are
written once in the type body, and conformance is per-signature (§34.5), so one
`size` satisfies every protocol that asks for `size`. There is no per-protocol
block to fill in. In Rust `impl Eq for T` is a place to put code; in mesa
`impl Equal` would be a name with nothing behind it. Swift is the closest
analogue on every axis — nominal conformance lists, protocol inheritance,
default bodies in extensions — and it chose implication.

**Reversibility runs the other way, and is spent knowingly.** §34.5 decided the
original question on a ratchet: additive later is free, subtractive later is
not. By that test **requirement is the conservative choice.** Everything written
under requirement stays valid if implication arrives afterward — the explicit
prerequisite merely becomes redundant — while the reverse breaks every type that
named only the derived protocol. Choosing implication therefore spends a ratchet
rather than preserving one, which is the opposite posture to §34.5's own.

This is the strongest thing sayable for requirement and §51.2 does not say it.
The cost is taken deliberately: the ratchet is worth less than what it buys,
because the intermediate state it preserves is the one §34.5 already identified
as the cost to be removed.

**The manifest property is real and already partial.** Requirement's other
argument is locality — a type's head lists every protocol it conforms to, with
no transitive lookup, which fits §45's refusal to descend and the general
preference for declaration-time answers. But a type already carries members that
its own source does not show: an acquired provided body (§34.6) puts `contains?`
on a type whose body never mentions it. The head has never been a member
manifest; it has been a list of protocols to go and read. Implication extends an
indirection that exists rather than introducing one.

**What requirement would cost.** §34.5 priced the status quo as restated
required verbs in protocol declarations — stdlib source, written once.
Requirement does not remove that price, it **relocates it from one protocol
declaration to every implementing type**, which is worse than the status quo
rather than better, and is §51.2's inversion stated concretely. The cost also
acquires a multiplier: under implication depth is free at the use site, and
under requirement it is depth × implementors, so every edge added to the
protocol graph edits every type downstream of it. §34.5 rested its judgment on
the set staying shallow, and §51 exists because that condition is expiring.

**What requirement would not change is bounds.** It is tempting to argue that
implication is what lets a `Seq` bound admit `Iter`'s verbs under §52. It is
not: the closure exists under both readings, so a checker propagates through it
either way. Requirement constrains what an author must write, never what the
phase knows. §34.5's "worth less in mesa than the Rust analogy suggests" applies
to both readings equally and settles nothing between them.

**One argument for requirement is filed rather than weighted.** Under
implication a protocol author adding a prerequisite silently grows every
implementor's conformance set, so a type with an existing `show` of the wrong
shape acquires a `SignatureMismatch` from a change it did not make. Requirement
surfaces that as the more legible "you must now write `impl Show`". This is a
real hazard in a language with a package ecosystem and third-party protocols;
mesa has neither yet, and §46's acyclic dependencies mean the blast radius is
bounded and statically known. Worth revisiting if protocols become something
packages routinely publish for other packages to implement.

**The hedge, taken.** Implication does not have to forbid writing a prerequisite
explicitly. `impl Equal, Hash` stays legal — §51.5's visited set already makes it
harmless — leaving authors who want the manifest a way to write it. Silently, with
no lint: a warning about a redundant name is a poor use of the first lint mesa
would own, and §51.11 finds the line worth drawing somewhere else entirely. Taking
the hedge keeps the collapse and recovers most of what requirement offered, which
is why the implication decision is not as narrow as it looks.

## 51.11 The written list is checked before it is flattened

§51.10's hedge leaves `impl Equal, Hash` legal and silent. It does not follow
that `impl Equal, Equal` should be, and the two languages nearest mesa's shape
draw the line in exactly that place.

Swift permits `struct S: Equatable, Hashable` with no diagnostic, though
`Hashable` refines `Equatable`; what it rejects is stating the *same*
conformance twice, once on the type and again in an extension. Java compiles
`class C implements List, Collection` clean, and the JDK is full of the shape;
what the JLS forbids is one interface appearing twice as a direct superinterface,
`implements List, List`, or twice after erasure. C# behaves the same way and
Ruby's ancestor chain simply deduplicates.

So the convergent rule is not the loose "permit redundancy" that the hedge on its
own suggests. It is sharper: **repetition of a name is an error, implication of a
name is not.** That line is worth having because the two sides are different
kinds of thing. Naming a prerequisite is a choice about how much a declaration
head should say; naming one twice is a typo, and there is no reading of it that
the author meant.

mesa already half-enforces it, by accident. `impl Equal, Equal` today walks the
conformance job twice over one protocol; the second pass finds no declaring
member, reaches the provided branch, and collides with its own earlier entry in
the map that names both sides of a conflict — so it is rejected, with the message
*"protocols 'Equal' and 'Equal' both provide 'x'"*, and only when the protocol
happens to provide something. The error is right and everything about how it
arrives is wrong.

Which settles where the check goes. It reads the **written** `impl` list, before
§51.3's flattening, because that is the last point at which "you typed it twice"
and "you reached it twice" are still distinguishable. §51.5's visited set applied
first erases the difference and makes the duplicate silently legal — the
deduplication that keeps the diamond working would swallow the typo with it. The
rule attaches to the `impl` line rather than to types, so it covers a protocol's
own prerequisites too: `proto Hash impl Equal, Equal` fails the same way, which
is what position-is-the-whole-rule (§51.1) should mean.

This does not disturb §51.4's claim on the only genuinely new error. A cycle is a
shape no program could express before composition; a duplicate is one every
program could always express and that was always refused. What §51.11 adds is not
a prohibition but a message, moved from an accident of the conflict rule to a
statement of its own — and it is the one place where the flattening must be told
to look at the source rather than the closure.


# 52. Types, bounds, and generics (open)

Nothing in this document has said what mesa's type annotations would be. One
aside in §34.5 — "mesa has no parameter types and no bounds" — is the whole of
the record. This section writes the stance down and works out what it would cost,
because §51 raises the question directly: composition is worth more under bounds
than without them, so the argument for building it depends on what bounds would
eventually look like.

Nothing here is decided.

## 52.1 The stance: disprove incorrectness

**The type system warns when something is provably wrong, and stays silent when
it is uncertain.** Not: reject unless provably right.

This is the inverse of the Rust and Haskell posture rather than a weaker version
of it. The value of static types, on this reading, is comprehension and the
catching of obvious mistakes. The cost lands at system boundaries — parsing,
arguments, validation — where a proving checker forces a program to either fight
the type system or escape it, and where the escape hatch is the part that
actually gets used.

Three clauses follow, and they constrain everything below.

1. **Annotations are optional**, now and permanently. An unannotated program is a
   valid program. This is not a migration path to full annotation.
2. **Annotations are `sem`-only.** Nothing an annotation says reaches the
   interpreter. There is no runtime check derived from a signature — which
   distinguishes this from gradual typing in the Typed Racket sense: no
   contracts, no wrappers, no blame.
3. **The checker may get smarter without invalidating anything.** Because it only
   ever refutes, a more precise pass produces more warnings on wrong programs and
   never rejects a program it previously accepted. That property is what makes
   this buildable incrementally, and it is the reason to fix the stance before
   the mechanism.

## 52.2 Coherence is a property of conformance sites

The intuition that generics bring trait coherence with them is worth dismantling,
since it is the objection that would otherwise block all of this.

Coherence is needed when a (type, protocol) pair can have more than one
implementation. Rust admits that because `impl Display for T` is a free-standing
item: two crates can write one for the same type, so without orphan and overlap
rules a program's meaning depends on what is linked. Generics make the problem
*visible* — a bound must select an implementation — but they do not create it.

Mesa cannot express the question. Conformance is a clause on the type's own
declaration (§20.1), resolved once, in the type's own file; §47.7 puts a native
type's conformance on its `extern` declaration by the same rule. There is exactly
one conformance site per pair. §51 does not change this: prerequisite edges are
facts about protocols, and no protocol edge introduces a site.

So the candidate set for any query has size one, before generics and after.
Coherence is zero and stays zero. What generics would introduce is **entailment**
— `T: Hash` must discharge `T: Equal` — and entailment is what §51's closure is.

## 52.3 What a proving system needs, and what refuting drops

| | proving | mesa, refuting |
| --- | --- | --- |
| Collect constraints from bodies | yes | **yes — the real cost** |
| Discharge obligations | recursive search | **set membership** (§52.4) |
| Coherence and overlap | orphan rules | not applicable (§52.2) |
| Substitute and propagate | unification | **yes — the real cost** |
| Dictionaries or monomorphisation | yes | none |

Two rows vanish and one collapses. The last vanishes because mesa dispatches
through the type registry at call time already: there is no witness to pass and
nothing to specialise, so annotations are erased by construction rather than by a
decision anyone has to take.

The two remaining rows are the genuine cost, and they are the same size with or
without §51. That is the sequencing argument in one line: **composition does not
make generics dearer, it makes them worth more.** §34.5's own reasoning runs
backwards here — it declined composition on the grounds that supertraits earn
their keep through bounds and mesa has none, which is an argument that the two
belong together, not that either should be declined alone.

## 52.4 Discharge is a lookup, because conformance is ground

Rust's solver is hard because an obligation can spawn obligations. `impl<T: Foo>
Bar for Vec<T>` means discharging `Vec<U>: Bar` requires discharging `U: Foo`,
recursively, with termination and overlap both live concerns.

Mesa's `impl` names ground protocols on a concrete declaration. No conformance is
conditional on anything, so an obligation is discharged by asking whether the
protocol lies in the closure of one the type is declared to implement — a
membership test against an artifact computed once at declaration. Constant time,
no search, no backtracking, and nothing whose termination has to be argued.

## 52.5 The line to hold

One rule keeps §52.4 true permanently.

**Conformance stays ground and unconditional: a type's `impl` clause never
carries a condition, and no conformance is parameterised.**

No `impl Order for List<T> where T: Order`. Conditional conformance is the single
feature that converts a membership test into a recursive solver, and it is
independent of both generics and composition — a full annotation-and-bounds
system can exist without ever admitting it.

This is the same shape of rule as §20.1's placing conformance on the type
declaration, and the two are a pair: one guarantees a single site, the other
guarantees the site is unconditional. Together they are what keep the checker a
table lookup.

## 52.6 Ambiguity moves from declarations to signatures

§34.6 resolves two unrelated protocols providing the same name as a
declaration-time error "resolvable by the type supplying its own member." That
works because a type declaration knows every fact and offers somewhere to put the
fix.

A bound offers neither. `def f(x: Hash & Order)` where both provide `key`, with
`x.key` in the body, names no type and has no site — this is Rust's ambiguity
requiring `<T as Hash>::key`.

Under §52.1 the answer is to say nothing: the call is unrefuted, dispatch decides
at runtime, and no warning is owed because nothing is provably wrong. Worth
noting that §51 *reduces* this rather than aggravating it — where the two
protocols are related, §51.6 has already chosen, and only genuinely unrelated
bounds reach the ambiguous case at all.

## 52.7 Variance is refused rather than annotated

`List<Circle>` where `List<Shape>` is expected is the question that forces
variance annotations into proving systems, and it has no cheap answer there.

Under §52.1 it has one: do not refute on the assignment. Refute at a use site
where an element is provably wrong, and stay silent about the container relation.
This is not an approximation of a variance rule; it is the absence of one. There
is no soundness obligation to discharge, so no annotation is needed to discharge
it.

The general form is what makes the whole stance cheap, and is worth stating
plainly: **the expensive machinery of a type system exists to protect soundness,
and soundness is what this stance gives up on purpose.** A refuting checker never
licenses anything, so its failure mode is a false warning rather than a program
that goes wrong. That is a difference in the severity of its bugs, and it is what
lets questions like variance be settled by convenience.

## 52.8 A protocol in type position

`proto P` binds `P` as a value — `B14` made `Proto` a core type so that `impl P`
could resolve through the ordinary scope chain. Under bounds, `P` also appears in
a signature, where it is not a value being referenced but a constraint being
named.

Whether those are the same `P`, and whether one spelling serves both, is not
answered here. It is a grammar and naming question rather than a semantic one —
nothing above depends on which way it goes — but it is the first place
annotations would touch machinery that already exists.

## 52.9 §34.4's reach check has a twin

The check that a provided body touches only the protocol's own members, widened
by §51.7 to the effective set, is structurally the check a generic body needs: a
body with `x: T` and `T: Order` reaching `x.compare` asks exactly the question
§34.4 asks about `self`, against a member set derived exactly the same way.

That the two coincide is some evidence the pieces are cut correctly. It also means
§51.7's widening is not a one-off — it is the first instance of a lookup that
bounds would want a second instance of.

## 52.10 What this leaves open

Everything. §52.1's stance is the only thing here proposed as settled, and it is
proposed rather than taken.

The specific open questions, in the order they would have to be answered: whether
annotations are worth building at all before the language is otherwise finished;
the spelling, including §52.8's collision; whether a bound may name more than one
protocol, and with what syntax; how much provenance the pass traces before the
warnings become useful, since a checker that knows only literal types refutes
almost nothing; and whether "provably wrong" admits nil, which under §18's policy
is the case deciding how much of the refutation is reachable at all.

**This has no row.** §34.5's aside is its only prior mention.

---

# 53. Contextual keywords (open)

§50.4 priced `from` against §20.5's budget and opened with a structural claim:
"there is no contextual keyword anywhere in the language, so a word that is a
keyword in one position and an identifier elsewhere would be a new mechanism,
and the lexer would stop being what decides what a word is." The second half is
worth checking, because §20.5's conclusion — every keyword is a member name
spent permanently — is what follows from it.

Nothing here is decided. What this section establishes is that the cost is far
less uniform than "spent permanently" suggests, that two thirds of it is
recoverable for the *entire* keyword set at negligible cost, and that the
remaining third is expensive for a reason neither §9 nor §20.5 named.

## 53.1 The lexer already keeps what the parser throws away

`lex_ident` (`src/syn/lex.rs:566`) maps a `Sym` to a keyword `TokenTag`, and
then sets `sym: Some(sym)` on the token whatever the tag turned out to be.
Every keyword token in the stream still carries the identifier it was lexed
from. What makes `x.type` a syntax error is not that the information is gone —
it is that `parse_member_expr` (`src/syn/parse.rs:565`) calls
`take(TokenTag::Ident)` and the tag no longer matches.

So the mechanism §50.4 describes is a *widening in the parser*, not a
context-sensitive lexer. The lexer goes on deciding exactly what it decides
today, and no token, span, or `Sym` changes shape. That does not make the
feature free; §53.6 is where the cost actually lands. It does move the question
off the lexer, where §50.4 put it.

## 53.2 The constraint is per-position, not per-keyword

The rule a contextual keyword has to satisfy is local: **a word can be soft at a
position iff, at every position where the parser expects it as a keyword, an
identifier cannot also occur** — or one token of lookahead separates the two
readings.

Applied to mesa, that sorts the nine positions currently requiring a bare
`Ident` into two groups, and the split is lopsided. Six of them are *closed* —
the parser is already inside a construct, the head keyword has already been
consumed, and nothing but a name may appear:

| position | site | why nothing else can appear |
| --- | --- | --- |
| member after `.` | `parse_member_expr:565` | `.` admits one thing |
| parameter name | `parse_params:484` | flat `Ident [: default]` list, `,`/`)` delimited |
| `each` loop variable | `parse_each_expr:716` | fixed slot; `in` must follow |
| keyword-argument label | `parse_call_expr:592` | already gated on an `Ident Colon` lookahead |
| `export` name | `parse_export_decl:199` | comma list of names |
| method name after `def` | `parse_method:337` | fixed slot; params or body follow |

At all six, accepting *any* keyword token is unambiguous — for every word in the
language, `end` included. Three remain open: bare expression start
(`parse_expr_unit:679`), type-item start (`parse_type_item:304`), and
module-item start (`parse_module_item:161`). Those three are §53.4.

**§9 assumed the parameter list was in the second group and it is in the
first.** That section restricted its own fix to dotted access on the grounds
that "field declarations in `type T(...)` and parameter lists are a separate
question with real ambiguity." Read against `parse_params`, there is no
ambiguity there to be had: the list is delimited, the elements are separated,
and no keyword introduces anything inside it. The claim was a guess about the
grammar, made in the same section that verified the dotted case by probe, and
it was never re-checked. §20.5 then inherited it as row 2 of the casualty
table.

## 53.3 The weak version, and what it recovers

*Widen the six closed positions to accept any keyword token; change nothing
else.* Concretely one `take_name()` helper returning `tok.sym.unwrap()` for
`Ident` or any keyword tag, swapped in at those six sites. Nothing downstream
moves: `sem` and `rt` receive a `Sym`, and a `Sym` carries no memory of which
tag delivered it.

Against §20.5's casualty table, verified against the current binary:

| | today | under the weak version |
| --- | --- | --- |
| `$print(e.type)` | `4,10: syntax error: unexpected token TYPE` | parses |
| `type R(start, end)` | `1,15: syntax error: unexpected token END` | parses |
| `type P(when)` | `2,8: syntax error: unexpected token WHEN` | parses |
| `self.def` in a method | `4,21: syntax error: unexpected token DEF` | parses |
| `def each(x)` | `2,5: syntax error: unexpected token EACH` | parses |
| bare `return type` in a method | `3,16: syntax error: unexpected token TYPE` | unchanged — still an error |

That is rows 1 and 2 of §20.5's table, recovered for the whole keyword set at
once rather than word by word, and including the two words that section named
as its recurring casualties. `range.end` works; `type R(start, end)` works.

**It is also the whole of §50.4's objection to `from`.** That section's table
gives `from` three rows and rests on the middle one — "`type Edge(from, to)` —
**not declarable at all**", called out as the reason `from` would be a worse
casualty than any word yet spent. The weak version answers that row. `from` in
`export decode from Codec` occupies no expression position anywhere, so it is
unaffected by everything in §53.4 onward. (Verified separately that
`type Edge(from, to)` and `.from` parse *today*, since `from` is not yet a
keyword — the spend §50.4 prices is real and still unpaid.)

## 53.4 The three open positions, and three classes of word

Bare expression start is §20.5's third row and the one it treated as
structural: mesa has implicit `self` field access (§3.2, extended by §14.4), so
"a bare keyword-named member lexes as its keyword whatever `parse_member_expr`
accepts." True, and it sorts the keywords into three classes rather than
leaving them uniform.

**Class A — words appearing in no expression-position construct.** `module`
`import` `export` `type` `extern` `case` `proto` `impl`. These are free at bare
expression start *by construction*, and the reason is §3.1: `parse_body_block`
(`parse.rs:453`) is a loop over `parse_expr`, and declarations are top-level
only. A method body cannot contain a `type` declaration, so a bare `type` there
can only be a field read. No lookahead at all.

Their two open declaration positions need lookahead(1), and it is one test in
both places: *a declaration is the keyword followed by a name token; an
expression beginning with that name is followed by an operator, `(`, `[`, `.`,
`=`, or a line end.* `type Foo` can only be a declaration; `type(x)`,
`type.id`, `type[0]` can only be expressions. In a type body the same test
reads off `Eq`, since `parse_field_decl:314` requires an initialiser: `type =
5` is a field, `type Foo … end` is a nested type.

**Class B — expression heads.** `when` `each` `loop` `do` `return` `break`
`raise`, and `def` (§53.5). Hard at bare expression start, and lookahead does
not rescue them: `when(x)` is a call to a proc named `when` or a conditional
over a parenthesised condition, and separating them needs unbounded lookahead —
the shape §17.1 already refused.

**Class C — interior markers and operators.** `in` `then` `else` `rescue` `end`
`and` `or` `not`. `end`, `then`, `else`, `rescue`, and `case`-as-arm-head are
block *terminators*: the body loops in `parse_body_block`, `parse_when_expr`,
and `parse_case_arms` run until they see one, and what they would otherwise
parse is an expression. A bare soft `end` at the head of a body line is
ambiguous with `end`-the-terminator at any lookahead depth. The infix words are
technically decidable at expression start, since an operator cannot lead an
expression, but nothing recommends it.

So the third row is recovered for class A only:

| | `x.w` | `T(w)` | bare `w` |
| --- | --- | --- | --- |
| A: `type`, `import`, `case`, … | yes | yes | yes |
| B: `when`, `each`, `return`, `def` | yes | yes | no |
| C: `end`, `then`, `in`, `else`, … | yes | yes | no |

## 53.5 §16.6 moves `def` out of class A, and that is the general lesson

The obvious reading of class A is "the declaration keywords." That reading is
wrong, and `def` is the counterexample.

§16.6 settles proc literals as `def (a, b) … end` and `def … end` in
**expression position**, which makes `def` an expression head. Bare `def` in a
method body no longer reads as a field access; it opens a literal. §16.6 also
rules out precisely the lookahead class A depends on — this section's test is
"keyword + name token ⇒ declaration," and §16.6 rejects "after `def`, an
`Ident` means named," on the ground that it "would spend the entire class of
literals whose body begins with a name — `def ident end`." At module-item
position the two rules contradict each other outright: `def foo … end` is a
declaration under one and a literal returning `foo` under the other.

`def` keeps the six closed positions and lands in class B. Nobody wants a
member named `def`, so the practical loss is nil. The criterion is what has to
change:

> **Class A is not "the declaration keywords." It is the keywords that appear
> in no expression-position construct anywhere in the grammar** — a whole-grammar
> property, checked per word against every construct, not a property of the
> word's usual role.

**This is §20.5's own thesis reproduced in miniature**, and it happened the same
way §24.5's `next` did: two sections decided independently, neither mentioning
the other, and the interaction was invisible from both. §16.6 closes with
"reusing `def` costs nothing against §9's keyword-namespace pressure, which a
new word would." Under the current hard-keyword regime that is true. Under a
contextual one it is false — reusing `def` for the literal is exactly what
costs `def` its third row.

**It gives `B7` a column it did not have.** The proc-literal spelling now has a
second consequence: `def` demotes `def` from class A to class B; `do||` demotes
`do`, which is free because `do` is already a class C interior marker of `each`
and `loop`; a brace-and-pipe form demotes nothing. That does not decide `B7` —
the dict collision is still the blocker — but the comparison was being made
without it.

## 53.6 The strong version's cost is anchors, not error messages

The obvious objection to soft keywords is that diagnostics degrade: a widened
`take` turns a precise "expected an identifier, found TYPE" into a failure
reported later and further away. That is true and it is not the real cost.

The real cost is that **the keyword set is the resynchronisation anchor set**. A
forgiving parser — the kind an LSP needs, and the kind mesa does not yet have —
contains damage from a missing `end` by scanning forward for a token that
reliably means *a new construct starts here* and restarting there. Class A
softness removes `type`, `import`, `proto`, and the rest from that set, and
removes them exactly when the parser is already lost, so the lookahead that
would disambiguate them is running over tokens it has already misgrouped.

**§17.1 has ruled on this shape before.** It gave up "`end` optional on a
bodiless variant" — not because that was ambiguous, since counting `end`s to
end of input resolves it uniquely, but because it "needs unbounded lookahead,
so it does not fit recursive descent, and it destroys error locality," with a
missing `end` "reported at EOF with everything between parsed under the wrong
reading." Soft class-A keywords buy a smaller benefit in the same currency,
against a parser that would be doing this on every keystroke rather than on
every compile.

Two more, in the same direction:

- **Mid-keystroke parses are the common input, not the rare one.** With `type`
  hard, a line holding only `type` yields a declaration node with a missing
  name — the parser knows what is being built and completion can attach to it.
  With `type` soft, it yields an expression reading a variable named `type`,
  and there is nothing to attach to. This is every declaration, at the moment
  it is typed.
- **No non-parsing highlighter can ever be correct.** Colouring `type` requires
  the lookahead, so it requires the parser. Minor where an LSP exists, total
  where one does not.

**All three costs belong to the strong version alone.** The six closed
positions of §53.2 sit inside constructs whose head keyword has already
anchored the parse, so none of those tokens was ever an anchor and widening
them moves no restart point. The weak version is anchor-preserving; the strong
version is anchor-destroying. That is a sharper line than diagnostics quality
and it falls in the same place.

## 53.7 What it does not cost

Three things worth retiring so they take no room in the decision.

**The postfix forms cannot interfere.** Class A's lookahead needs that no
construct places two name tokens adjacent. Both of mesa's postfix forms are
introduced by punctuation — `Expr::Member` by `.` and `Expr::Access` by `[` —
so neither can produce the adjacency. Worth stating as a property of the
postfix set rather than an accident: `Expr::Access` is the renamed subscript
syntax, never a call form, so the language has never had juxtaposition. **The
forward constraint is therefore narrow**: not "add no script syntax," but *add
no punctuation-free call form*, which mesa's `(`-required calls already imply.

**The `Sym` layer is untouched.** Keyword `Sym`s stay fixed constants assigned
in `Interner::new`; §20.6's unshadowable-prelude check, §33's resolver, and
everything downstream see names, not tags.

**The lookahead is not capitalisation-sensitive.** `take_type_name:440` rejects
only `!` and `?`; the uppercase-module/lowercase-proc split §50.5 relies on is a
convention, not a rule, so a sharper "next token is capitalised" test is not
available without promoting it to one.

## 53.8 Where this leaves the budget

The weak version is cheap, complete for two of three rows, costs no anchors, no
lookahead, and no interaction with `starts_expr`/`terminates_expr`, and would
retire §50.4's central objection to `from`. It does not reduce the keyword
count — every word is still reserved somewhere — so the honest claim is that it
shrinks the collision surface, not that it solves the budget.

The strong version looks worse after §53.5 than before it. It recovers the
third row for one word anyone actually wants (`type`), it has already lost
`def` to a decision made elsewhere, and it pays in exactly the coin §17.1
refused to spend.

**What changes about §20.5 either way is the direction of the ratchet.** That
section's permanence argument is that a spent keyword can never be reclaimed,
and its evidence is that both refunds on record came from deleting a feature
rather than from finding a cheaper spelling. Under a contextual regime,
softening a hard word later is backward-compatible and hardening a soft word
later is the breaking change. So the safe default for a new declaration keyword
inverts: introduce it soft, harden if it turns out to matter. §20.5's "adding a
keyword now has an unrecoverable cost" would stop being true of class A, which
is the class future keywords are most likely to join.

**What is open**: whether to take the weak version at all, which is a small
change with no known argument against it and therefore mostly a question of
whether the budget is worth spending any mechanism on; whether the strong
version is declined outright or held for whenever the forgiving-parser question
is actually live, since §53.6 is the only argument against it and it is an
argument about a parser that does not exist yet; and whether §16.6's
`def` should be re-examined given §53.5, which is really `B7`'s question with
one more column in the table.

**This has no row.** It arises from §50.4, which is itself open.
