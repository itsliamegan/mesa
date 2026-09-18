# Mesa Language Manual

## Table of Contents

1. [Introduction](#introduction)
2. [Principles](#principles)
3. [Lexical Structure](#lexical-structure)
4. [Values](#values)
5. [Procs](#procs)
6. [Types](#types)
7. [Protocols](#protocols)
8. [Expressions and Operators](#expressions-and-operators)
9. [Bindings and Scope](#bindings-and-scope)
10. [Control Flow](#control-flow)
11. [Error Handling](#error-handling)
12. [Modules](#modules)
13. [Packages](#packages)

- [Appendix A - Grammar](#appendix-a---grammar)
- [Appendix B - Built-in Behaviors](#appendix-b---built-in-behaviors)
- [Appendix C - Built-in Errors](#appendix-c---built-in-errors)
- [Appendix D - Operator Precedence](#appendix-d---operator-precedence)

## 1. Introduction

This document describes the design & behavior of the Mesa programming language.
Mesa is a hobby programming language designed primarily for writing personal
software including web applications and command-line utilities. It is inspired
by Ruby, JavaScript, and Swift (among others). It is dynamically & strongly
typed but features a subset of compile-time semantic analysis which can achieve
some of the correctness assurances of more sophisticated static type systems. It
features a pragmatic data model taking cues from object-oriented as well as
functional approaches.

## 2. Principles

Mesa's design is guided by a set of principles. No one principle dominates over
any other, and they are traded off against one another where appropriate. More
importantly, they are not to be understood as a set of *axioms* from which the
entire design follows; rather, they are guidance to be used for evaluating
points of tension or directions of development. A language which is axiomatic
may possess a satisfying simplicity, but this satisfaction comes at the cost of
its expressiveness. These principles are furthermore not to be taken as ultimate
or universal, but particularly suited to the design of personal software
systems.

**Systems are built from the coherent organization of concepts and
relationships.** Programs represent discrete understandings of problems and
solutions in a particular knowledge space. Thus, a programming language must
provide the tools for constructing a vocabulary and grammar appropriate to the
problem domain. In Mesa, systems are composed of a foundation of nouns and verbs
which are combined in distinct forms of relationships, making the task of
describing a particular problem the same task as converting it to an appropriate
meta-language.

**Prioritizing local reasoning results in clarity and encourages
composability.** Whereas some systems value flexibility above all else, Mesa
prioritizes the ability to examine a program in relative isolation and, with a
set of rules & restrictions in mind, understand what it means. Every name
binding is typically locatable within the current source file. Standard control
flow constructs and bounded points of extensibility provide specific ways to
describe a problem space. The result is that reading and understanding a
particular program consists of converting a limited set of language constructs
into a defined set of domain constructs and exploring the resulting space.

**Modeling complexity necessitates navigating it rather than refusing it.** Mesa
does not claim to be a 'simple' language but nevertheless attempts to be an
understandable one. Programming languages, like natural languages, are tools for
expressing thought and denoting reasoning, and for this reason must bend to the
facts of the material world. While there is a satisfying reassurance in the
belief that a minimal system can emphasize only the relevant aspects of a
particular problem—while the language used to denote the problem regresses into
the background—the reality is that the problem itself ultimately resolves into
language. The world is not, despite our efforts to make it so, a coherent
system, and so our tools for describing it must not resign to this assumption.

**Optimizing for small scale yields less generality**: A concept which provides
affordances at a small scale may not necessarily provide the same affordances at
a larger scale, and vice-versa. In particular, industrial systems tend to
require robustness or universality that is not applicable at the scale of
personal systems. By optimizing for one end of this tradeoff as opposed to
attempting to satisfy both, programs written in Mesa can satisfy the hobbyist
without burdening them with constraints which may be legitimate in an industrial
context.

**Visual cohesion is not separate from conceptual cohesion.** Programs are both
text files on disk and the reasoning which those files denote. Just as coherent
concepts establish relations to one another with specific meanings and patterns,
so must the text representing those concepts possess harmonious relationships. A
resonant Mesa program strives to possess a distinctive style, rhythm, and rhyme
just as a resonant piece of writing does.

**Understanding and trust are the foundations for learning and empowerment.** A
programming language designed for building personal software ultimately serves
both those who use it and those in their community with whom the software is
shared. Mesa provides features for introspection and limits its available forms
of access control so as to make the software explainable and understandable to
anyone who can access it. Additionally, Mesa itself is free software and its
implementation is documented so as to make building it and extending it as easy
as possible.

## 3. Lexical Structure

All Mesa source is UTF-8 encoded text. This source text is broken up into
primitive elements, called tokens.

### Identifiers

An identifier begins with an ASCII letter or `_` and is followed by any number
of ASCII letters, digits, and `_`s. It may end with a single `?` or `!`.

```mesa
byte
read_all
_init
Account
JSON
empty?
sort!
```

Capitalization carries no requirements to the language. An identifier with a
capitalized first letter conventionally names a type or protocol.

A trailing `?` or `!` in an identifier carries no requirements to the language.
A trailing `?` is conventionally used to name a boolean predicate. A trailing
`!` is conventionally used to distinguish a mutating, raising, or otherwise
distinct variant of a non-suffixed name.

A trailing `!` is treated as part of the preceding identifier unless it is
immediately followed by an `=`. In this latter case, it is considered part of
the `!=` operator. Mesa has no bare `=` operator, so this is unambiguous.

### Keywords

The following symbols are reserved words and cannot be used as identifiers:

```mesa
and     break   case    def     do      each    else
end     export  extern  false   impl    import  in
loop    module  nil     not     or      proto   raise
rescue  return  self    then    true    type    when
```

### Literals

A **number literal** is a sequence of digits with an optional fractional part
separated by a `.`. The fractional part must be followed by at least one digit.

A number literal is never negative; `-5` is unary `-` applied to `5`. There is
no digit separator, exponent notation, or alternate radix.

```mesa
17
64.2
3.0
```

A **string literal** is a sequence of characters delimited by double quotes. It
may span multiple lines.

A **character literal** is a single character delimited by single quotes.

Both string and character literals permit the escapes `\n`, `\t`, `\"`, `\'`,
and `\\`. There is no string interpolation.

```mesa
"Hello, world!"
"Multi
line"
"Multi\nline"

'a'
'é'
'\t'
```

A **boolean literal** is either `true` or `false`.

The **nil literal** is `nil`.

### Comments and Whitespace

Comments begin with a `#` and run until the end of the line. There is no
multi-line comment form.

```mesa
# Add the numbers.
5 + 2.3 # These are numbers.
```

Almost all whitespace in a Mesa program is insignificant. Newlines are an
exception; they are used to delimit expressions. There is no explicit statement
separator or terminator.

### Operators and Punctuation

The following tokens are operators or otherwise meaningful punctuation:

```mesa
:=  ==  !=  <   >   <=  >=  +   -   *  /
<<  &   :   ,   .   (   )   [   ]   {  }
```

The assignment operator is spelled `:=`. A bare `=` is a syntax error.

## 4. Values

**Values** are the fundamental unit of composition in Mesa. Every name in a Mesa
program is bound to an inspectable value with some set of behaviors.

Like other dynamically typed languages, values have **types**, and any name can
be bound to a value of any type. A value's type functions as the primary
descriptor for that value's behavior. A type provides a structure for otherwise
arbitrary data, and from this structure emerges a coherent concept which affords
composing abstractions. Types have members which are properties about values of
that type or behaviors which values of that type exhibit.

One can say that values are 'instances of' a particular type, and this will be
familiar from object-oriented vocabulary. Instances of a particular type behave
in similar ways.

### Built-in Types

The following types are built in to the language:

- `Num`: A 64-bit floating-point number, e.g. `64.2`, `17`.
- `Bool`: A boolean, `true` or `false`.
- `Char`: A single Unicode scalar, e.g. `'a'`, `'é'`.
- `Str`: An immutable sequence of Unicode scalars, e.g. `"Hello, world!"`.
- `Digest`: An immutable, opaque blob of bytes, used for hashing.
- `List`: A mutable, heterogeneous, insertion-ordered sequence of values, e.g.
  `[1, "apple", nil]`.
- `Dict`: A mutable, heterogeneous, insertion-ordered mapping of keys to values,
  e.g. `{"id": 1024}`.
- `Proc`: An invocable unit of behavior. A method bound to its receiver is also
  a `Proc`.
- `Type`: A meta-unit of related data and functionality.
- `Proto`: A unit of behaviors common to multiple types.
- `Module`: A unit of related functionality.
- `Nil`: An absent value, `nil`.

### Value and Reference Semantics

Mesa supports value semantics for some built-in types. The types which follow
value semantics are: `Num`, `Bool`, `Char`, `Str`, `Digest`, and `Nil`. All
other types, built-in and user-defined, follow reference semantics. The
consequences of this design for *equality* are mitigated by defaulting to
structural equality for all types, overridable in cases where it is appropriate.
The consequences of this design for *mutability* are not mitigated; Mesa instead
encourages mutable algorithms and data sharing as the natural paradigm for
expressing domain relationships and the change which occurs over a program's
lifetime.

### Truthiness, `nil`, and Absence

Every value other than `nil` and `false` is **truthy**. When a truthy value is
used in a conditional or with a logical operator, it is equivalent to `true`.

An empty list `[]` and an empty string `""` are both truthy. Truthiness denotes
*presence*, not *fullness*.

`nil`, on the other hand, is used to denote absence.

## 5. Procs

A **proc** is a subroutine which computes a value and hands it back to the
caller.

```mesa
def balance(transactions)
	total := 0
	each transaction in transactions do
		...
	end
	total
end
```

As with all blocks, the value of calling a proc is the value of its last
expression. If there is an early `return`, the value is instead the value
provided to the `return`, or `nil` if the `return` has no value.

```mesa
def find(id, query)
	record := find_by_id(id)
	when record == nil then
		return nil
	end
	...
end
```

### Parameters and Arguments

Procs can be declared with or without **parameters**, values which must be
provided by the caller to specify the behavior of the proc. The caller provides
**arguments** which bind to these parameters.

Without parameters, the proc's parentheses are optional. An incorrect number of
provided arguments will raise an error.

Parameters can either be *required* or *optional*. Required parameters must be
provided by a caller while optional parameters can be omitted. Placing optional
parameters before required ones is an error at declaration time. An optional
parameter can reference any parameter declared before it, and the expression is
evaluated every time the proc is called.

```mesa
def new
	...
end

def credit(account, amount: 0)
	...
end

new
new()
credit(account)
credit(account, 127.25)
```

When provided by a caller, arguments can either be *positional* or *keyword*.
Positional arguments are not accompanied by a label while keyword parameters
are. The labels provided by the caller are the *names of the parameters*, so
these names are part of the proc's interface. Placing keyword arguments before
positional ones is an error.

```mesa
def debit(account, amount)
end

debit(account, amount: 50.13)
```

### Invocation and Mentioning

A declared proc **invokes** when it is referenced rather than resolving to the
value of the proc itself.

```mesa
def amount
	100
end

amount # => 100
```

Invocation is a property of the *declaration*, not of the value. A name
introduced by an assignment or by a parameter never invokes; it reads its value
as it is, even when that value is a proc.

```mesa
def notify
	...
end

def apply(action)
	action # => def notify; a parameter never invokes
end

handler := &notify
handler   # => def notify; a local never invokes
handler() # calls notify
```

If invocation would leave a required parameter unfilled, it raises an error.

```mesa
def credit(account)
	...
end

credit # => runtime error: missing arg 'account'
```

When used with explicit call syntax, automatic invocation is suppressed. This
supports using parentheses in all cases without knowing the type of the value
that the proc returns.

```mesa
def make
	...
end

make     # invokes with no arguments
make()   # calls with no arguments
make()() # calls the result of make() with no arguments
```

To take an invocable as a value, the **mention** operator `&` asserts that what
it names is invocable and yields it without invoking. Anything that is not
invocable raises an error.

```mesa
def make
	...
end

make  # invokes make
&make # takes make as a value
&100  # => runtime error: type Num is not invocable
```

On a name that would not have invoked, `&` yields the value unchanged, though
the assertion still holds. It can therefore be written wherever the author wants
to be explicit, without needing to know whether the name would have invoked.

```mesa
handler := &notify
&handler # => def notify; the local would not have invoked either way
```

Mentioning can be used to pass a proc as an argument to another proc.

```mesa
listeners := []

def add_listener(listener)
	listeners << listener
end

def on_credit
	...
end

add_listener(&on_credit)
```

Types *are not invocable* and must instead always use explicit call syntax. This
syntactic necessity contains semantic information: creating an instance of a
type and exercising a behavior of that instance are emphasized as two different
operations.

### Proc Literals

A proc defined in an expression context is a **proc literal** and evaluates to
the callable proc. Literals can be used to store procs in variables or pass
procs to other procs. Though such composition is typically accomplished via
types, instances, and protocols, passing procs as values can achieve a form of
composition and delegation useful for simple situations.

```mesa
add := def (a, b) a + b end
add(4, 2) # => 6
```

A proc literal can be used to pass a proc as an argument to another proc.

```mesa
amounts := transactions.collect(def (txn) txn.amount end)
```

Unlike a regular proc, a proc literal *does not* automatically invoke. This
follows the rule that only declared procs invoke, while locals do not.

```mesa
make := def ... end
make # => def ...
```

#### Capturing

Proc literals can **capture** variables bound in the scope in which they are
defined. Locals referenced in the body of a proc literal are captured by value,
including `self`. Module-level and prelude names *are not* captured and resolve
at call time.

```mesa
def adder(lhs)
	def (rhs) lhs + rhs end
end

add := adder(5)
add(2) # => 7
add(3) # => 8
```

Locals capture the *value of the binding*, not the *value itself*. Assigning to
a captured name inside a proc literal only changes the literal's own copy of the
value.

This means that for mutable types whose value is a reference, mutating them
inside a literal works as expected. This is the case for `List`, `Dict`, and all
user-defined types. `Num`, `Bool`, `Char`, and `Str` are not mutable, however,
so assigning to a name bound to them will not mutate the captured value.

```mesa
def stream(items: [])
	next := def
		when items.empty? then
			nil
		else
			items.shift()
		end
	end
	next
end

next := stream([1, 2])
next() # => 1
next() # => 2
next() # => nil

def broken
	count := 0
	counter := def
		count := count + 1
		count
	end
	counter
end

counter := broken
counter() # => 0
counter() # => 0
```

## 6. Types

**Types** are a foundational tool for abstraction in Mesa along with procs. If a
proc defines a verb reusable across contexts, a type defines a *noun* reusable
across contexts and more easily transferrable as a value.

### Constructors

All types feature a single constructor invoked via call syntax. Because
constructors follow normal call syntax rules, they can declare defaults for
their parameters and callers can use keyword syntax. There are no additional
construction or object initialization concepts. Any alternative construction
behavior should go in a type-level method.

```mesa
Str() # => ""
Num() # => 0.0

type Amount(cents)
end

Amount(105) # => Amount(cents: 105)
```

As opposed to procs and methods, types *are not* invocable. Referencing the name
of a type does not automatically invoke its constructor. One always needs to
distinguish a type from its instances, whereas one only rarely needs to
distinguish an invocable from the result of its invocation.

### User-defined Types

While the built-in types provide a foundation for describing computational
use-cases, most applications and libraries will want to define their own types
specific to their problem domain. **User-defined types** provide a mechanism for
declaring types, their behaviors, and their invariants in a coherent, consistent
manner.

#### Fields

Every type has a statically known set of **fields**, pieces of data that compose
the type. Fields can be defined either by the constructor or in the type body.
Constructor fields are supplied by the caller while body fields are not.

```mesa
type Task(label, priority: 0)
end

Task("Update documentation")
```

Fields defined in the body can reference constructor fields or other body fields
declared before themselves. They are evaluated every time a value of the type is
constructed.

```mesa
type Group(label)
	labels := [label]
end
```

Only constructor fields are printed by the output of `Display`, while body
fields are not. All fields are printed by the output of `Inspect`.

```mesa
type Group(label)
	items := []
end

Group("Urgent").display # => "Group(label: \"Urgent\")"
Group("Urgent").inspect # => "Group(label: \"Urgent\", items: [])"
```

#### Methods

Procs attached to a type are its **methods**. Methods are procs bound to an
instance of a type, and they follow all the same rules: they can define
parameters, are automatically invoked when referenced, and can be mentioned to
suppress invocation.

```mesa
type Account(transactions)
	def balance
		each transaction in transactions do
			...
		end
	end
end

balance := &account.balance
```

In the body of a method, `self` is bound to the instance on which the method was
called.

```mesa
type Task(label)
	def clone
		self
	end
end

Task("Refactor").clone # => Task(label: "Refactor")
```

##### Type-Level Methods

A type can define methods which are bound to the type object itself rather than
instances of it. This can be done by defining a method with the prefix `self.`
in the type body. In the body of a type-level method, `self` is bound to the
type object itself. There is *no equivalent* for type-level fields.

```mesa
type Amount(cents)
	def self.zero
		self(0)
	end
end
```

#### Members and Access

Fields, methods, local types, and variants (discussed below) are all uniformly
**members** of a type. Accessing a member uses syntax dependent on context:
inside the type body, the *bare name* of a member accesses the member; outside
the type body, the member operator spelled `.` accesses the member of the
instance on the left hand side of the dot.

```mesa
type Task(label, priority: 0)
	def description
		label
	end
end

Task("Documentation").label # => "Documentation"
Task("Documentation").description # => "Documentation"
```

Whether or not a member is automatically invoked when referenced follows the
normal rules of invocation, which read the declaration and not the value. A
method is declared, and invokes. A field is not, and is read as it is even when
it holds a proc.

```mesa
type Task(label, on_done)
	def description
		label
	end
end

def archive
	...
end

task := Task("Documentation", &archive)
task.description  # => "Documentation"; a method invokes
task.on_done      # => def archive; a field never invokes
task.on_done()    # calls archive
&task.description # takes the bound method as a value
&task.on_done     # => def archive; the field would not have invoked either way
```

Fields are therefore uniform with one another: a field is read the same way
whatever it holds. Uniform access between a field and a method taking no
arguments is preserved, since both still yield a value.

Mesa has *no access control* mechanism within types. Every member of a
type—field, method, local type, variant—is public and introspectable, even if it
is not providable by the caller or has no meaning outside of the type's internal
implementation.

Mesa also has *no mutability control* mechanism. Every field is readable and
writable from every part of the program. Methods are the one exception to this
rule because they cannot be reassigned.

### Local Types

Some types are more related than shared module membership or source code
adjacency would imply. In the case of certain domain types, several internal
types may be related implementations of similar behavior. For these cases,
**local types** provide a form of namespacing within a particular module.

```mesa
type LinkedList
	head := nil

	type Node(data)
	end
end
```

Local types are not related to the enclosing type in any way other than their
name. They cannot automatically see into fields of instances of the enclosing
type and cannot call methods on it without explicitly holding an instance. In
all ways except for name lookup they function exactly as a sibling type at the
module level with a name prefix.

### Case Types

**Case types** are Mesa's form of sum types. They are used to represent values
which can come in one of a limited number of forms, called **variants**. A
variant is declared inside the body of the case type and can declare fields and
methods of its own.

A variant *cannot* declare its own variants; there is a single level of nesting.
In situations where you need multiple levels of nesting, you can nest instances
of further case types as fields of the variants.

```mesa
type Transaction
	case Credit(amount) end
	case Debit(amount) end
end
```

References to variants always use the variant's fully-qualified name. This
applies in every context: constructing a variant, matching one in `when...case`,
and naming one in the case type's own body.

```mesa
def absolute
	when transaction
	case Transaction.Credit then
		transaction.amount
	case Transaction.Debit then
		-1 * transaction.amount
	end
end
```

Case types support a limited form of behavioral sharing. When a member name
cannot be resolved against a case type's variant, the member is looked up
against the case type itself. This is suited for situations in which a common
behavior can be implemented in terms of a shared behavior.

```mesa
type File
	case Virtual(contents)
		def read ... end
	end
	case Disk(fd)
		def read ... end
	end

	def read_all
		each byte in read do
			...
		end
	end
end
```

Fallback applies only to *methods, not fields*; a case type cannot have fields.

## 7. Protocols

**Protocols** define behaviors shared across different types. Protocols provide
a mechanism for types to explicitly declare that they share a common interface
across all implementers. A type's conformance to a protocol is statically
checked at declaration time.

Protocols are named after the primary verb which they provide, rather than with
a capability suffix like '-able.'

```mesa
proto Draw
	def draw end
end

type Square
	impl Draw

	def draw
		...
	end
end

type Circle
	impl Draw

	def draw
		...
	end
end
```

A protocol can declare both *required* and *provided* methods. A required method
must be implemented by the implementing type. A provided method may be
implemented by the implementing type, but otherwise falls back to the
implementation provided by the protocol.

A protocol may not declare, but may possess, *derived* methods. A derived method
has the same fallback mechanism as a provided method; however, instead of the
default implementation being provided in the protocol declaration, it is
provided by the language itself. The protocols which have derived methods are
`Equal`, `Hash`, `Access`, and `Inspect` (see below).

A protocol must either be empty, contain only derived methods, or contain at
least one required method. A protocol with only provided methods is not allowed.

```mesa
proto Handle
end

proto Read
	def read end
	def read_all
		each byte in read do
			...
		end
	end
end

type File
	impl Handle, Read

	def read
		...
	end
end

type Socket
	impl Handle, Read

	def read
		...
	end
end
```

A protocol's provided methods can only access members the protocol *explicitly
declares*. This is statically checked at declaration time.

### Built-in Behaviors

Some protocols are provided which offer a limited mechanism for user types to
hook into built-in behaviors, including some operators. The protocols and the
behaviors they govern are as follows:

| Protocol  | Governs                   | Status                 |
|-----------|---------------------------|------------------------|
| `Equal`   | `== !=`                   | Automatic, overridable |
| `Hash`    | Use as a `Dict` key       | Opt-in                 |
| `Order`   | `< <= > >=`               | Opt-in                 |
| `Access`  | `a[k]` and `a[k] := v`    | Opt-in                 |
| `Append`  | `<<`                      | Opt-in                 |
| `Iterate` | Use in `each`<sup>1</sup> | Opt-in                 |
| `Display` | User-facing output        | Opt-in                 |
| `Inspect` | Developer-facing output   | Automatic, overridable |

<sup>1.</sup> `Iterate` pairs with the `Advance` protocol, not listed here.

The details of these protocols, including what they require, how they behave,
and what they guarantee can be found in Appendix B.

## 8. Expressions and Operators

Mesa is an expression-oriented language. Everything in an expression position
evaluates to a value. Not every piece of syntax can be used in expression
position, however; declarations like `module` and `type` are not expressions and
do not evaluate to a value. Additionally, not every expression evaluates to a
*useful* value. For example, `each` evaluates to `nil` unless it `break`s with a
value, and `when` evaluates to `nil` if there is no matching branch and no
`else` clause.

### Expression Termination

Expressions are terminated at a newline when two things are true at once. First,
the token *preceding* the newline must be able to end an expression. Second, the
token *following* the newline must not be able to continue one.

The tokens that can end an expression are: an identifier; the keywords `self` and
`end`; a string, character, number, boolean, or nil literal; and the characters
`)`, `]`, and `}`.

The consequence of this rule is that almost any expression can be wrapped across
lines by breaking either before or after an operator.

```mesa
transactions.
	collect(def (transaction) transaction.amount end).
	select(def (amount) amount.cents > 0 end)

# or

transactions
	.collect(def (transaction) transaction.amount end)
	.select(def (amount) amount.cents > 0 end)
```

```mesa
when
	shape.visible?
	and not shape.color.transparent?
then
	...
end

# or

when
	shape.visible? and
	not shape.color.transparent?
then
	...
end
```

The only time one must specifically break before an operator is with `-`, `[`,
and `(`.

```mesa
5 -
4   # evaluates to 1

5   # evaluates to 5
- 4 # evaluates to 4; ignored
```

```mesa
items[0] # evaluates to items[0]

items # evaluates to items
[0]   # evaluates to [0]; ignored
```

```mesa
make(1) # evaluates to the result of calling make(1)

make # evaluates to the result of invoking make
(1)  # evaluates to 1; ignored
```

`return` and `break` are the exceptions to this rule. These constructs always
terminate at a newline despite the fact that they cannot end an expression. This
means a `return` or a `break` is always bare unless its operand starts on the
same line.

```mesa
return 5.2 # returns 5.2

return # returns nil
5.2    # ignored
```

Additionally, because procs have optional parameter lists, a newline immediately
after the name of the proc always indicates an empty parameter list, even if
that line begins with a `(`. This is to distinguish parameter lists from
grouping expressions with `(`.

```mesa
# draw takes 1 parameter, 'shape'
def draw(
	shape
)
	...
end

# draw takes no parameters
def draw
	(shape)
end
```

### Block Results

A block of expressions always evaluates to the value of its last expression. The
only exception to this is if the block exits early.

### Operators

Mesa supports the following operators, grouped by function:

**Arithmetic**: `+ - * /` and unary `-` on `Num`. Not overloadable by user
types.

**Concatenation**: `+` on `Str`. Not overloadable by user types.

**Mixing**: `+` on `Digest`. Combines two hashes in an *order-sensitive* manner,
i.e. `a.hash + b.hash` and `b.hash + a.hash` are not necessarily equivalent. Not
overloadable by user types.

**Ordering**: `< <= > >=` on any type implementing `Order`. `Num` compares
numerically, `Str` and `Char` compare lexicographically by scalar value. User
types can overload by implementing `Order`.

**Equality**: `==` and `!=` on any type implementing `Equal`. Compares by
*structural equality* over all fields; there is *no reference equality*
operator. Automatic for every user type, but can be overloaded by explicitly
implementing `Equal`. Implemented on every built-in type.

**Logical**: `and` and `or` on any type short-circuit and return the selected
operand. `not` on any type always returns a `Bool`. Every value is truthy except
for `nil` and `false`, including e.g. an empty `List`. Not overloadable by user
types.

**Append**: `<<` on a type implementing `Append` appends the operand to the end
and returns the receiver so it can be chained. User types can overload by
implementing `Append`. Implemented on built-in `List`.

**Mention**: `&` on an *invocable* takes its value instead of automatically
invoking it. Only applicable to `Proc`s, which includes a method bound to its
receiver.

### Call, Member, and Access

**Call** syntax uses `(` and `)` to denote arguments passed to a `Proc`, bound
method, or type constructor. Calls can pass both positional and keyword
arguments, and parameters can be declared required or optional.

**Member** access syntax uses `.` to denote accessing a field, method, or local
type / variant of the left-hand value.

**Access** syntax uses `[` and `]` to denote keyed or indexed access into a
collection implementing `Access`.

### Precedence and Grouping

Expressions may be wrapped in `( )` to form explicit groups. Otherwise,
expressions are parsed according to the following precedence table ordered from
highest to lowest precedence:

| Operator        | Description              | Form                 | Associativity |
|-----------------|--------------------------|----------------------|---------------|
| () [] .         | Call, access, member     | Postfix              | Left-to-right |
| &               | Invocable mention        | Prefix               | Right-to-left |
| -               | Arithmetic negation      | Prefix               | Right-to-left |
| * /             | Multiplication, division | Infix                | Left-to-right |
| + -             | Addition, subtraction    | Infix                | Left-to-right |
| <<              | Append                   | Infix                | Left-to-right |
| == != < <= > >= | Comparison               | Infix                | Left-to-right |
| not             | Logical negation         | Prefix               | right-to-left |
| or and          | Logical or, and          | Infix                | Left-to-right |
| :=              | Assignment               | Infix, place on left | Right-to-left |

This table can also be found in Appendix D.

## 9. Bindings and Scope

Every name is introduced by a **binding** which associates that name with a
value. The constructs which introduce bindings are type declarations, proc
declarations, imports, proc and constructor parameters, `each` loop variables,
`rescue` bindings, and assignment.

Every source location has a **scope** which consists of the bindings introduced
both at that point and by the constructs enclosing it. The constructs which
introduce a scope are `each`, `loop`, `when`, `do`, `rescue`, and proc bodies.

Two kinds of name are introduced by none of these constructs. `self` is a
keyword which denotes a value, but it is not a binding and cannot be reassigned.
The prelude's names are bound before any program runs.

A binding does not outlive the scope which introduced it. A name first bound
inside a block is unavailable after it, while an assignment to a name bound in
an enclosing scope rebinds that binding rather than introducing a new one.

```mesa
count := 0
do
	count := count + 1 # rebinds the outer binding
	extra := 5         # a new local binding
end
count # => 1
extra # => error: name 'extra' is not defined
```

### Resolution

A name is *always* resolved in the following order, falling back to a later tier
only if it is not found in an earlier one.

> `locals → self's members → module bindings → prelude`

**Locals** are the bindings of the current scope and of the scopes enclosing it.
Resolution walks outward, beginning from the innermost scope; if the name is not
found in any local scope, resolution continues to the next tier rather than
raising.

**`self`'s members** are reached through the `self` value which a method
implicitly possesses: in an instance method, this is the instance; in a
type-level method, this is the type. Each of these values has a fixed set of
members against which a name is looked up. This lookup happens even for bare
names, so e.g. `name` in an instance method on `type Account(name)` refers to
the field. Code outside a method has no `self` and skips this tier.

**Module bindings** are a module's top-level declarations, top-level
assignments, and the names its imports bind.

**Prelude** names are available in every module without being imported. The
prelude consists of the built-in type names and nothing else.

If a name is not found in any tier, an error is raised.

### Assignment

Mesa features a single assignment operator, `:=`. The first assignment to a name
binds it, and subsequent assignments rebind it.

```mesa
sum := 4 + 5
sum := sum + 1
sum # => 10
```

The `:=` operator always evaluates to the *assigned value*.

```mesa
old := 4
new := old := 2
old # => 2
new # => 2
```

#### Places

The left-hand side of the `:=` operator designates a **place**. A place is
either an *identifier*, a *member* expression, or an *access* expression; no
other syntax is valid as a place for assignment.

```mesa
shape := Square(5.2)
shape.color := "red"
shapes[0] := shape
```

A place is resolved in the same order as any other name. Assigning to a bare
name writes to `self`'s member before it creates a local, which is how a method
assigns to a field of its instance with no receiver.

```mesa
type Account(balance)
	def deposit(amount)
		balance := balance + amount
	end
end
```

A name which resolves in no tier is bound as a local in the current scope. There
is *no variable declaration syntax*. Reading an undefined name raises, while
assigning to one defines it.

```mesa
color # runtime error: name 'color' is not defined.
color := "red"
color # => "red"
```

Not every place accepts assignment. Methods and read-only members cannot be
reassigned.

```mesa
type Group(items: [])
	def size
		items.size
	end
end

Group().size := def 5 end # runtime error: member 'size' on type Group is read-only.
```

A module-level declaration claims its name for the whole of the module that
declares it, so an assignment cannot displace one. Re-declaring a member is an
error at declaration time.

```mesa
def make
	...
end

make := make # semantic error: duplicate member 'make'
```

### Shadowing

There is no explicit syntax for shadowing. Only the binding forms which bind a
name in a new scope shadow: proc and constructor parameters, `each` loop
variables, and `rescue` bindings.

```mesa
num := 5
each num in [1, 2, 3] do
	num # => 1, 2, 3
end
num # => 5
```

Assignment shadows only a prelude name. Where a name resolves to a local, to a
member of `self`, or to a module binding, assignment rebinds it in place; where
it resolves to a prelude name or to nothing at all, assignment introduces a new
binding in the current scope.

```mesa
Num # => type Num
Num := 5
Num # => 5
```

## 10. Control Flow

Mesa features two **looping** constructs (`each` and `loop`), a **conditional**
construct (`when`), a **grouping** construct (`do`), two **exit** constructs
(`return` and `break`), and **error handling** constructs (`raise` and
`rescue`).

Control enters at the top of a construct, transfers to subroutines, proceeds
sequentially through expressions, and leaves the construct either at its end or
through a construct that transfers out of it.

### Loops

#### `each`

`each` loops over each item in any type implementing `Iterate`, binding a single
name for each yielded item. The `iterate` method is called on the value
provided, and the resulting `Advance` type is called until it is exhausted.
`Iterate` is implemented for the built-in `List`, `Dict`, and `Str` types,
yielding items, keys, and characters respectively. An `each` expression
evaluates to `nil` unless exited with a `break` value.

```mesa
each item in [1, "apple", nil] do
	item # => 1, "apple", nil
end

each key in {"id": 100} do
	key # => "id"
end
```

#### `loop`

`loop` loops infinitely until explicitly exited with `break`. This can replace
any alternative loop construct, like `while`, `until`, etc. A `loop` expression
evaluates to `nil` unless exited with a `break` value.

```mesa
count := 0
loop do
	when count == 10 then break end
	count := count + 1
end
count # => 10
```

### Conditionals

#### `when...else`

`when...else` evaluates conditions in the order they are listed until one
matches. Alternative conditions are specified using `else when` branches, and a
catch-all branch using `else`. An `else` branch is not required, in which case
the entire construct evaluates to `nil`; otherwise, it evaluates to the value of
the branch which matched. The entire construct is closed by a single `end`.

A condition matches when the relevant expression evaluates to a truthy value.

```mesa
name := when 4 / 2 == 2 then
	"four"
else when 8 / 2 == 4 then
	"eight"
else
	"other"
end

name # => "four"
```

#### `when...case`

`when...case` matches against the type of a value. Each branch names a type, and
the branches are tried in order.

There is no destructuring, no binding form, and no type narrowing; once a branch
matches, member access in the body proceeds by normal rules.

```mesa
type Expr
	case Ident(name)
	case Lit(lit)
end

when expr
case Expr.Ident then
	expr.name
case Expr.Lit then
	expr.lit
end
```

The branches need not name variants of a case type. `when...case` performs
general type matching against any type.

```mesa
def encode(val)
	when val
	case Str then ...
	case Num then ...
	else ...
	end
end
```

The branches *cannot* match against a case type itself, given that it can never
be constructed. Such a branch will never match.

```mesa
type Transaction
	case Credit(amount) end
	case Debit(amount) end
end

result := when transaction
case Transaction then
	"transaction"
else
	"other"
end

result # => "other"
```

`when...case` is *not* a general pattern matching construct. For conditions more
complex than type dispatch, `when...else` should be used instead.

#### Exhaustiveness Checking

Exhaustive coverage of a `when...case` construct is checked at declaration time
against the construct's *branches*. If the branches all name variants of the
same case type, they are checked for exhaustiveness. If an `else` is provided in
this circumstance, the branches *must not exhaustively cover the variants*; this
would swallow later additions to the case type. If the branches do not cover the
variants, however, an `else` is permitted.

If the branches name unrelated types, whether they be case types or not, an
`else` is always permitted. If it is absent, the `when...case` evaluates to
`nil` if no branch matches.

### `do`

`do` groups a sequence of expressions and evaluates to the value of the last one
in its body. `do` introduces a new scope, so it can introduce variables that are
not visible after its accompanying `end`.

```mesa
sum := do
	total := 0
	each num in [1, 2, 3] do
		total := total + num
	end
	total
end

sum # => 6
```

`do` can be paired with `rescue` to guard a series of expressions from a raised
error.

```mesa
do
	window.draw(shape)
rescue
case DrawError.OutOfBounds then ...
end
```

### `return`

`return` exits a proc early and causes it to evaluate to the value provided to
the `return`. If `return` is not provided a value, the proc evaluates to `nil`.
In the absence of a `return`, a proc or block evaluates to the value of its last
expression.

There is no special-cased guard clause; guarding a particular section of a block
can be accomplished by a `return` inside a `when`.

```mesa
def draw(shape)
	when not shape.visible? then
		return
	end

	...

	return dimensions
end
```

`return` inside a proc literal returns from the literal, not the defining proc.

```mesa
def visible(shapes)
	shapes.select(def (shape) return shape.visible? end)
end
```

#### `break`

`break` terminates the containing `each` or `loop`. If provided a value, the
loop evaluates to the value, otherwise `nil`.

`break` is statically checked to never occur outside of a loop body; doing so is
an error at declaration time.

```mesa
first := each num in [1, 2, 3] do
	break num
end

first # => 1
```

`break` inside a proc literal breaks from the inner loop.

```mesa
each nums in [[1], [2], [3]] do
	first := def
		each num in nums do
			break num
		end
	end
	first # => 1, 2, 3
end
```

### `raise` and `rescue`

`raise` propagates a value through calls until a `rescue` handles it, or until
the program exits. A `rescue` is always paired with a `do`, and it guards the
expressions in that `do`'s body.

## 11. Error Handling

Mesa represents both programming and system faults using **errors**. Errors are
propagated with `raise` and handled with `rescue`. An unhandled error terminates
the program and reports the location it was raised from.

There are no result types, no error return values, and no error propagation
operator.

### Built-in Errors

Errors raised by the language itself or by the core library are variants of the
`Error` type provided by the core library. The variants of this type, as well as
their purposes, are listed in Appendix C. They are used throughout this section
whenever `Error` is referenced.

### `raise`

`raise` aborts the currently executing construct and propagates the raised value
up the call stack until it is handled.

```mesa
raise Error.KeyError("key \"id\" not found")
```

Although it is most useful to raise a value of a purpose-built error type, a
value of any type can be raised.

```mesa
raise "An error occurred."
```

### `rescue`

`rescue` stops an error from propagating if one of its branches matches the
error value. If a branch matches, its corresponding `do` evaluates to the last
expression in that branch.

```mesa
config := {}
port := do
	config["port"]
rescue
case Error.KeyError then
	4000
end

port # => 4000
```

#### Branches

`rescue` matches against the type of the raised value. Like `when...case`, each
branch names a type, and the branches are tried in order.

```mesa
type LedgerError
	case InsufficientFunds(account) end
	case InvalidRecipient(id) end
end

do
	account.transfer(amount, to: other)
rescue
case LedgerError.InsufficientFunds then ...
case LedgerError.InvalidRecipient then ...
end
```

An `else` branch matches any error that no preceding branch matches.

```mesa
do
	account.transfer(amount, to: other)
rescue
case LedgerError.InsufficientFunds then ...
else ...
end
```

#### Propagation

Unlike `when...case`, the branches of a `rescue` are *not* checked for
exhaustiveness; they need only name the types they explicitly handle. An
unhandled error propagates through calls until a `rescue` handles it, or until
the program exits.

```mesa
type DrawError
	case OutOfBounds(shape) end
	case InvalidDimensions(shape) end
end

do
	raise DrawError.OutOfBounds(shape)
rescue
case DrawError.InvalidDimensions then ...
end
```

#### Binding

If provided a name, `rescue` binds a variable to the raised error. This is
unlike `when...case`, which has no binding form.

```mesa
do
	account.transfer(amount, to: other)
rescue err
case LedgerError.InsufficientFunds then
	err.account # => Account(...)
case LedgerError.InvalidRecipient then
	err.id # => ID(...)
end
```

### Hierarchies

Mesa does not support subtyping. Instead, an error hierarchy is constructed by
composing case types.

```mesa
type DrawError
	case GeometryError(cause) end
end

type GeometryError
	case OutOfBounds(shape) end
	case InvalidDimensions(shape) end
end
```

To handle the composed error variant, `rescue` matches against the outer type
while `when...case` matches against the inner type. Whereas `rescue` *is not*
checked for exhaustiveness, `when...case` *is*.

```mesa
do
	window.draw(shape)
rescue err
case DrawError.GeometryError then
	when err.cause
	case GeometryError.OutOfBounds then ...
	else ...
	end
end
```

An example of a set of errors defined using this structure is the predefined set
of errors, described in Appendix C.

## 12. Modules

Every Mesa program is composed of **modules**. A module is declared in a single
file which contains its name, imports, declarations, and top-level expressions.
This file contains the entire contents of the module; it may never be reopened
or extended.

```mesa
module Draw

origin := 0

type Shape
	...
end

def draw(shape)
	...
end
```

A module's declarations are *order-independent*.

```mesa
module Draw

def draw(shape: Circle(radius: 5))
	...
end

type Circle(radius)
	...
end
```

A module may contain top-level expressions in addition to declarations. They are
evaluated when the module is loaded.

```mesa
module Draw

unit := scale(1)

def scale(n)
	...
end
```

Unlike declarations, top-level expressions are *order-dependent*.

```mesa
module Draw

double := unit * 2 # runtime error: name 'unit' is not defined

unit := scale(1)
```

### Children

A module may have **children** which are declared in files within a directory
that sits beside the module's own file and shares its name. Child modules use a
dot `.` to indicate their relationship, akin to member access. A child module
declares its name as the full dotted path.

```mesa
# draw/screen.ms
module Draw.Screen
```

A module's filename carries no requirements to the language; only the module's
prefix and its position are checked. A module is conventionally declared in a
file named for its leaf, lowercased with underscores between words.

```mesa
# draw.ms
module Draw

# draw/printer.ms
module Draw.Printer

# draw/screen.ms
module Screen # semantic error: module 'Screen' must be declared under 'Draw'
```

Module nesting is a fact about names rather than lookup. Parent and child
modules do not implicitly share names. A child module must import anything it
uses from its parent or from a sibling. A parent module must import anything it
uses from its children.

```mesa
# draw.ms
module Draw

type Shape
	...
end

# draw/screen.ms
module Draw.Screen

import Draw.Shape
```

### Imports

A module may **import** members of other modules, including other modules
themselves. An import names a member by a dotted path and binds its last segment
to a name in the module's scope.

There is no wildcard or renaming import form.

```mesa
module Draw

import Draw.Screen
import Draw.Screen.Window

Screen # => module Draw.Screen
Window # => type Window
```

An import of a module binds the module itself, so it may be passed to a proc,
stored, or otherwise used as a value.

```mesa
import Draw.Screen
import Draw.Printer

def render(shape, target: Screen)
	target.draw(shape)
end

render(shape)
render(shape, target: Printer)
```

#### Cycles

Modules may not form an import cycle. A cycle happens when a module imports
another module that imports it, directly or indirectly. An import cycle is an
error at declaration time.

```mesa
# draw/screen.ms
module Draw.Screen

import Draw.Printer # semantic error: import cycle: Draw.Printer → Draw.Screen → Draw.Printer

# draw/printer.ms
module Draw.Printer

import Draw.Screen
```

### Members

A module's declarations, its top-level assignments, and its child modules are
its **members**, in the same sense as a type's, and are accessed by the same
member operator `.`.

```mesa
# draw.ms
module Draw

origin := 0

type Shape
	...
end

def draw(shape)
	...
end

# draw/screen.ms
module Draw.Screen

import Draw

Draw.origin # => 0
Draw.Shape  # => type Shape
&Draw.draw  # => def draw(shape)
```

An imported module *is not* a member of the importing module. There is no
implicit re-export.

```mesa
# draw/screen.ms
module Draw.Screen

import Draw.Shape

Shape # => type Shape

# draw/printer.ms
module Draw.Printer

import Draw.Screen

Screen.Shape # runtime error: module Draw.Screen has no such member 'Shape'
```

A module's members share a single namespace. Two members may not share a name.

```mesa
# draw.ms
module Draw

type Screen
	...
end

# draw/screen.ms
module Draw.Screen # semantic error: duplicate member 'Screen'
```

A module's members are read-only from outside the module. To mutate a member of
another module, go through a proc call.

```mesa
# draw.ms
module Draw

origin := 0

# draw/screen.ms
module Draw.Screen

import Draw

Draw.origin := origin # runtime error: member 'origin' on module Draw is read-only
```

## 13. Packages

A **package** is a tree of modules. It is the unit of *compilation*,
*evaluation*, and *distribution* of a Mesa program. When building an executable,
it is evaluated as a package; when sharing a library, it is distributed as a
package.

A package has a **root module**, which is declared in `src/package.ms`. Every
other module in the package descends from the root.

`src/` is a build root rather than a module; it has no name in the language. The
root module is the one module whose declaring file sits inside the directory
holding its children rather than beside it.

```
tracker/
	package.toml
	src/
		package.ms      # module Tracker
		groups.ms       # module Tracker.Groups
		tags.ms         # module Tracker.Tags
		tags/
			label.ms    # module Tracker.Tags.Label
```

A directory within `src/` must have a declaring file beside it.

Like all other modules, children of the root module must have a prefix that
matches their parent.

```mesa
# src/package.ms
module Tracker

# src/groups.ms
module Groups # semantic error: module 'Groups' must be declared under 'Tracker'
```

### Exports

Within a single package there is *no access control* mechanism. Just as no type
hides its members, every module and its members are reachable from every other
module. Between packages, **exports** declare which names are reachable.

A module exports all of its members by default. An export list at the head of a
module narrows this to just the members it names.

```mesa
# src/package.ms
module Tracker

export Task, complete

type Task
	...
end

def complete(task)
	...
end
```

Every module has its own export list, and the lists *compose*. A name is
reachable from outside a package when every member on its path is exported, from
the root module's own members down to the member itself.

A module's children are among its members, so an export list may name them. A
module exports a child without importing it.

```mesa
# src/package.ms
module Tracker

export Tags

# src/tags.ms
module Tracker.Tags

export tag

def tag(task)
	...
end

# in another package
import Tracker.Tags.tag
```

Where any module on the path does not export the next, the name is unreachable,
whatever the modules below it export.

```mesa
# src/package.ms
module Tracker

export Task

type Task
	...
end

# src/tags.ms
module Tracker.Tags

export tag

def tag(task)
	...
end

# in another package
import Tracker.Tags.tag # semantic error: module 'Tracker' does not export 'Tags'
```

Exports govern the reachability of *names*. They never govern what may be seen
of a value which is already named. An export list may only name *module
members*, not members of the module's types themselves; an importer holding an
exported value has access to the whole of it.

```mesa
export Task.status # syntax error: unexpected token DOT
```

### Manifest

A package's **manifest** declares the package's name and its dependencies. The
manifest file is `package.toml` in the package's root directory beside `src/`.

```toml
name = "tracker"

[dependencies]
ledger = "https://github.com/acme/ledger"
```

A package's name identifies the package for distribution. It is never written in
Mesa source. A package's root module name is declared in `src/package.ms`, like
every other module's, and is the name source code uses. The two are
conventionally related but the correspondence is not checked.

### Dependencies

A package may **depend** on other packages, which it declares in its manifest by
package name. Every module in a package may import any of the package's
dependencies.

Packages may not form a dependency cycle. A package may not depend on itself,
directly or indirectly.

```toml
# tracker/package.toml
name = "tracker"

[dependencies]
ledger = "https://github.com/acme/ledger" # semantic error: dependency cycle: ledger → tracker → ledger

# ledger/package.toml
name = "ledger"

[dependencies]
tracker = "https://github.com/acme/tracker"
```

An import names a dependency by its root module name, which the dependency
declares in its own `src/package.ms`, not by the package name under which the
manifest declares it.

```mesa
# src/tags.ms
module Tracker.Tags

import Ledger.Account
```

An import does not distinguish a dependency's module from a module of the
importing package's own tree.

```mesa
# src/tags.ms
module Tracker.Tags

import Ledger.Account
import Tracker.Groups
```

Because imports do not distinguish them and there is no renaming import form,
the root module names of a package and its dependencies share one space. A
package depending on another package whose root module shares a name with its
own, or on two packages whose root modules share a name with each other, is an
error.

```toml
[dependencies]
ledger = "https://github.com/acme/ledger"
account-book = "https://github.com/acme/account-book" # semantic error: packages 'ledger' and 'account-book' both declare root module 'Ledger'
```

Only a dependency's exported names are reachable.

```mesa
import Ledger.Table # semantic error: module 'Ledger' does not export 'Table'
```

The **core library** is a dependency of every package and is not declared. Its
root module is `Core`. This module shares the one namespace with every other
dependency's root, so no package may declare a root module named `Core`.

```mesa
import Core.Behaviors.Order
```

## Appendix A - Grammar

```
program ::= { module-item }

module-item ::= module-decl
              | import-decl
              | export-decl
              | type-decl
              | extern-decl
              | proto-decl
              | proc-decl
              | expr

module-decl ::= "module" dotted-path
import-decl ::= "import" dotted-path
dotted-path ::= IDENT { "." IDENT }

export-decl ::= "export" IDENT { "," IDENT }

type-decl ::= "type" IDENT [ "(" params ")" ]
              [ impl-clause ]
              { case-decl }
              { field-decl }
              { type-decl | method-decl }
              "end"

case-decl ::= "case" IDENT [ "(" params ")" ]
              [ impl-clause ]
              { field-decl }
              { type-decl | method-decl }
              "end"

field-decl ::= IDENT ":=" expr

method-decl ::= "def" [ "self" "." ] IDENT [ "(" params ")" ] { expr } "end"

extern-decl ::= "extern" "type" IDENT
                [ impl-clause ]
                { method-decl }
                "end"

proto-decl ::= "proto" IDENT { proc-decl } "end"

impl-clause ::= "impl" dotted-path { "," dotted-path }

proc-decl ::= "def" IDENT [ "(" params ")" ] { expr } "end"

params ::= [ param { "," param } ]
param  ::= IDENT [ ":" expr ]

expr ::= each-expr
       | loop-expr
       | when-expr
       | do-expr
	   | proc-expr
       | return-expr
       | break-expr
       | raise-expr
       | assign-expr
       | binary-expr
       | unary-expr
       | mention-expr
       | call-expr
       | member-expr
       | access-expr
       | primary

each-expr ::= "each" IDENT "in" expr "do" { expr } "end"

loop-expr ::= "loop" "do" { expr } "end"

when-expr ::= "when" expr ( "then" { expr } ( "end" | else-clause )
                           | case-arms )
else-clause ::= "else" ( when-expr | { expr } "end" )

case-arms ::= { case-arm } [ "else" { expr } ] "end"
case-arm  ::= "case" dotted-path "then" { expr }

do-expr ::= "do" { expr } ( "end" | "rescue" [ IDENT ] case-arms )

proc-expr ::= "def" [ "(" params ")" ] { expr } "end"

return-expr ::= "return" [ expr ]

break-expr ::= "break" [ expr ]

raise-expr ::= "raise" expr

assign-expr ::= place ":=" expr
place       ::= IDENT | member-expr | access-expr

binary-expr ::= expr bin-op expr
bin-op ::= "or" | "and"
         | "==" | "!=" | "<" | ">" | "<=" | ">="
		 | "<<"
         | "+" | "-" | "*" | "/"

unary-expr   ::= ( "not" | "-" ) expr
mention-expr ::= "&" expr

call-expr ::= expr "(" args ")"
args      ::= [ arg { "," arg } ]
arg       ::= [ IDENT ":" ] expr

member-expr ::= expr "." IDENT
access-expr ::= expr "[" expr "]"

primary ::= "self"
          | IDENT
          | BUILTIN "(" expr ")"
          | STR | CHAR | NUM | BOOL | NIL
          | "[" [ expr { "," expr } ] "]"
          | "{" [ expr ":" expr { "," expr ":" expr } ] "}"
          | "(" expr ")"

IDENT   ::= ( letter | "_" ) { letter | digit | "_" } [ "?" | "!" ]
NUM     ::= digit { digit } [ "." digit { digit } ]
STR     ::= '"' { any | escape } '"'
CHAR    ::= "'" ( any | escape ) "'"
escape  ::= "\n" | "\t" | "\"" | "\'" | "\\"
BOOL    ::= "true" | "false"
NIL     ::= "nil"
BUILTIN ::= "$" IDENT
COMMENT ::= "#" { any except "\n" }
```

## Appendix B - Built-in Behaviors

Some protocols are provided which offer a limited mechanism for user types to
hook into built-in behaviors, including some operators. The protocols and the
behaviors they govern are as follows:

| Protocol  | Governs                   | Status                 |
|-----------|---------------------------|------------------------|
| `Equal`   | `== !=`                   | Automatic, overridable |
| `Hash`    | Use as a `Dict` key       | Opt-in                 |
| `Order`   | `< <= > >=`               | Opt-in                 |
| `Access`  | `a[k]` and `a[k] := v`    | Opt-in                 |
| `Append`  | `<<`                      | Opt-in                 |
| `Iterate` | Use in `each`<sup>1</sup> | Opt-in                 |
| `Display` | User-facing output        | Opt-in                 |
| `Inspect` | Developer-facing output   | Automatic, overridable |

<sup>1.</sup> `Iterate` pairs with the `Advance` protocol, detailed below.

These protocols are all defined in the `Core.Behaviors` module of the core
library.

### `Equal`

`Equal` indicates that values of a type can be tested against one another for
(in)equality. `equal` should return `true` when `self` is equivalent to `other`
and `false` otherwise. By implementing `Equal`, values of a type can be used
with the equality operators `== !=`.

`Equal` is implemented by default for all types, both built-in and user-defined.
It is *structural* over all fields, including body fields. For a value of a
given type, another value of that same type is equal to the first if all the
values of its fields are equal. For a value of a variant, another value of that
same variant is equal to the first both if the fields are equal and if the
values are of the same variant.

```mesa
"Hello, world!" == "Hello, world!" # => true

type Task(priority)
end

type Group(tasks: [])
end

Task(5) == Task(5) # => true
Task(5) == Task(2) # => false
Group([Task(2)]) == Group([Task(2)]) # => true
```

Equality is guaranteed to terminate in the case of referential cycles.
Structural equality is intended to determine *semantic equivalence* rather than
referential identity. Two graphs of cyclic values are equal if every path
traversal through their fields yields the same sequence of values.

```mesa
type Node(data)
end

a := Node(nil)
a.data := a

b := Node(nil)
c := Node(b)
b.data := c

a == b # => true
```

`Equal` is implemented for all user-defined types, so `impl Equal` on any type
is redundant. Consequently, any type implementing an `equal` method will
override the behavior of `==`.

```mesa
type Account(id, name)
	def equal(other)
		id == other.id
	end
end

Account(1, "Checking") == Account(1, "Savings") # => true
```

### `Hash`

`Hash` indicates that values of a type can deterministically provide a
representation of their identity. `hash` should return a `Digest` that
deterministically identifies the value. By implementing `Hash`, values of a type
can be used as a key in a `Dict`.

`Hash` is implemented for `Num`, `Bool`, `Char`, `Str`, `Digest`, `Proc`,
`Type`, `Module`, and `Nil`. `List` and `Dict` do *not* implement `Hash`.

`Hash` requires that `Equal` be implemented. Two values which are equal should
hash to the same value. A mutable value with an unstable identity *should not*
implement `Hash`, because a mutation after its insertion into a `Dict` would
cause future lookups to hash to a different value.

```mesa
import Core.Behaviors.Hash

type Account(id, name, transactions)
	impl Hash

	def equal(other)
		id == other.id and name == other.name
	end

	def hash
		id.hash + self.name.hash
	end
end
```

A type which implements `Hash` and overrides `equal` must also override `hash`.
Not doing so is an error at declaration time.

### `Order`

`Order` indicates that values of a type can be compared against one another to
produce a coherent ordering. `order` should return `-1` when `self < other`, `1`
when `self > other`, and `0` otherwise. By implementing `Order`, values of a
type can be used with the comparison operators `< <= > >=`.

`Order` is implemented for `Num`, `Str`, and `Char`. `Num` sorts by numeric
value. `Str` and `Char` sort by *scalar value*, not locale collation.

```mesa
import Core.Behaviors.Order

type Task(priority)
	impl Order

	def order(other)
		priority.order(other.priority)
	end
end

Task(0) < Task(1)  # => true
Task(1) >= Task(0) # => true
Task(3) < Task(1)  # => false
Task(2) <= Task(2) # => true
```

The ordering produced by a type implementing `Order` must be *total*,
*transitive*, and *reflexive*. It need only be antisymmetric with respect to the
sorted equivalence of the values.

`order` returning `0` operates independently of structural equality. For
example, a user-defined type might implement an ordering in terms of a single
field, resulting in two structurally distinct objects evaluating as equivalent
under `Order`.

The ordering produced by a type implementing `Order` can be thought of as one
over a *projection* of the values of the type rather than the values themselves.

```mesa
import Core.Behaviors.Order

type Task(name, priority)
	impl Order

	def order(other)
		priority.order(other.priority)
	end
end

Task("Documentation", 1).order(Task("Refactor", 1)) # => 0
Task("Documentation", 1) == Task("Refactor", 1) # => false
```

### `Access`

`Access` indicates that values of a type can have their elements read from or
written to using access syntax. `access` should return the item corresponding to
the given key. `store` should set the given key to the given item.

Implementing `access` is required, but implementing `store` is optional.
Implementing `access` without implementing `store` indicates that values of the
type cannot be mutated using access syntax. In this case, an error will be
raised when calling `store`.

`Access` is implemented for `List` and `Dict`. Accessing a list by an
out-of-bounds index will raise an error. Accessing a dict by a key which does
not implement `Hash` will raise an error.

```mesa
import Core.Behaviors.Access

type Collection(items: [])
	impl Access

	def access(key)
		items[key]
	end

	def store(key, item)
		items[key] := item
	end
end

collection := Collection(["Item 1", "Item 2"])
collection[0] # => "Item 1"
collection[1] := "Item 3" # => "Item 3"
```

### `Append`

`Append` indicates that values of a type can be appended to. `append` should
append the given item and return `self` to support chaining. By implementing
`Append`, values of a type can be used with the `<<` operator.

`Append` is implemented for `List`, and appends the item to the list.

```mesa
import Core.Behaviors.Append

type Collection(items: [])
	impl Append

	def append(item)
		items << item
		self
	end
end

Collection() << "Item" # => Collection(items: ["Item"])
```

### `Iterate`

`Iterate` indicates that values of a type can be iterated using the built-in
`each` construct. `iterate` should return an `Advance` which yields the
successive values until it is exhausted.

`Iterate` is implemented for `List`, `Dict`, and `Str`. `List` yields items,
`Dict` yields keys, and `Str` yields characters.

```mesa
import Core.Behaviors.Iterate

type Group(tasks: [])
	impl Iterate

	def iterate
		tasks.iterate
	end
end

group := Group([Task(...)])
each task in group do
	...
end
```

### `Advance`

`Advance` indicates that values of a type represent an exhaustable iterator.
`advance` should advance the iterator and return `true` if the iterator has more
items, or `false` otherwise. `item` should return the current item, or `nil` if
the iterator is exhausted.

`each` calls `advance` then `item` repeatedly until `advance` returns `false`.

```mesa
import Core.Behaviors.Advance
import Core.Behaviors.Iterate

type LinkedList
	impl Iterate

	head := nil

	type Node(data, next)
	end

	type Iter(node)
		impl Advance

		started? := false

		def advance
			when not started? then
				started? := true
			else when node != nil then
				node := node.next
			end
			node != nil
		end

		def item
			when node != nil then
				node.data
			end
		end
	end

	def iterate
		LinkedList.Iter(head)
	end
end
```

### `Display`

`Display` indicates that values of a type can be converted to user-facing
output. `display` should return a `Str` which is suitable to display to a user.

`Display` is implemented for `Str` and `Char`, and returns the contents of the
respective string or character without quotes and with escape sequences
translated to their literal equivalents.

`Display` requires that `Inspect` be implemented. A value which cannot be
printed in a user-facing format prints using a developer-facing format. No value
is unprintable.

```mesa
"\"Hello\nworld!\"".display # => "\"Hello\nworld!\""
```

### `Inspect`

`Inspect` indicates that values of a type can be inspected by a developer for
runtime debugging. `inspect` should return a `Str` which provides information
about the value. In general, `inspect` should return something which would be
valid syntax for creating that value, but this is not a requirement.

`Inspect` is implemented for all types, both built-in and user-defined. `Num`
returns the number's decimal representation using the least amount of precision
necessary. `Bool` returns `"true"` or `"false"`. `Nil` returns `"nil"`. `Char`
returns the single-quoted character, escapes included. `Str` returns the
double-quoted string, escapes included. `List` returns the comma-separated items
surrounded by square brackets. `Dict` returns the comma separated key-value
pairs separated by curly braces.

`Inspect` is implemented for all user-defined types and returns a constructor
invocation including both constructor and body fields and their values.

```mesa
type Account(name)
	transactions := []
end

Account("Checking").inspect # => "Account(name: \"Checking\", transactions: [])"
```

`Inspect` is implemented for all user-defined types, so `impl Inspect` on any
type is redundant.

## Appendix C - Built-in Errors

| Case Type     | Variant          | Description                                                            |
|---------------|------------------|------------------------------------------------------------------------|
| Error         | ProtocolError    | A value did not conform to a particular protocol.                      |
| Error         | ArgumentError    | The arguments provided to a proc or constructor were invalid.          |
| Error         | TypeError        | A value of an incorrect type was used.                                 |
| Error         | MemberError      | A member was incorrectly accessed.                                     |
| Error         | IndexError       | An invalid `List` index was provided.                                  |
| Error         | NameError        | An undefined name was read.                                            |
| Error         | KeyError         | An invalid `Dict` key was provided.                                    |
| ProtocolError | NotIterable      | A value that does not implement `Iterate` was used with `each`.        |
| ProtocolError | NotAccessible    | A value that does not implement `Access` was used with `[]`.           |
| ProtocolError | NotAppendable    | A value that does not implement `Append` was used with `<<`.           |
| ProtocolError | NotOrderable     | A value that does not implement `Order` was used with `< <= > >=`.     |
| ProtocolError | NotImplemented   | A method was called on a type that only implements part of a protocol. |
| ArgumentError | Missing          | A required argument was not provided.                                  |
| ArgumentError | TooMany          | Too many positional arguments were provided.                           |
| ArgumentError | Unknown          | An unknown keyword argument was provided.                              |
| ArgumentError | Duplicate        | A duplicate keyword argument was provided.                             |
| TypeError     | IndexNonNum      | A `List` index that is not a `Num` was provided.                       |
| TypeError     | ArithNonNum      | A value that is not a `Num` was used with `+ - * /`.                   |
| TypeError     | ConcatNonStr     | A value that is not a `Str` was used with `+`.                         |
| TypeError     | NotCallable      | A value that is not callable was used with `()`.                       |
| TypeError     | NotConstructible | A case type, rather than a variant, was constructed.                   |
| TypeError     | NotInvocable     | A value that is not invocable was used with `&`.                       |
| TypeError     | CaseNonType      | A value that is not a type was used in a `when...case` branch.         |
| MemberError   | Missing          | A member was read from that does not exist.                            |
| MemberError   | ReadOnly         | A member was written to that is read-only.                             |
| IndexError    | OutOfRange       | A `List` index that is outside the bounds of the list was provided.    |
| IndexError    | NonIntegral      | A `List` index that is a non-integral `Num` was provided.              |


```mesa
type Error
	case ProtocolError(cause) end
	case ArgumentError(cause) end
	case TypeError(cause) end
	case MemberError(cause) end
	case IndexError(cause) end
	case NameError(message) end
	case KeyError(message) end
end

type ProtocolError
	case NotIterable(message) end
	case NotAccessible(message) end
	case NotAppendable(message) end
	case NotOrderable(message) end
	case NotImplemented(message) end
end

type ArgumentError
	case Missing(message) end
	case TooMany(message) end
	case Unknown(message) end
	case Duplicate(message) end
end

type TypeError
	case IndexNonNum(message) end
	case ArithNonNum(message) end
	case ConcatNonStr(message) end
	case NotCallable(message) end
	case NotConstructible(message) end
	case NotInvocable(message) end
	case CaseNonType(message) end
end

type MemberError
	case Missing(message) end
	case ReadOnly(message) end
end

type IndexError
	case OutOfRange(message) end
	case NonIntegral(message) end
end
```

## Appendix D - Operator Precedence

| Operator        | Description              | Form                 | Associativity |
|-----------------|--------------------------|----------------------|---------------|
| () [] .         | Call, access, member     | Postfix              | Left-to-right |
| &               | Invocable mention        | Prefix               | Right-to-left |
| -               | Arithmetic negation      | Prefix               | Right-to-left |
| * /             | Multiplication, division | Infix                | Left-to-right |
| + -             | Addition, subtraction    | Infix                | Left-to-right |
| <<              | Append                   | Infix                | Left-to-right |
| == != < <= > >= | Comparison               | Infix                | Left-to-right |
| not             | Logical negation         | Prefix               | right-to-left |
| or and          | Logical or, and          | Infix                | Left-to-right |
| :=              | Assignment               | Infix, place on left | Right-to-left |
