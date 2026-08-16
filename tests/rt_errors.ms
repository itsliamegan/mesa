#! runtime error: name 'unknown' is not defined

$print(unknown())

#---

#! runtime error: name 'inner' is not defined

def outer()
	inner()
end
outer()

#---

#! runtime error: type Num is not callable

num := 1
num()

#---

#! runtime error: missing arg 'b'

def either(a, b)
	b
end
either(2)

#---

#! runtime error: type T has no such member 'unknown'

type T(a)
end
$print(T(1).unknown)

#---

#! runtime error: type Str cannot be used in arithmetic

$print(5.1 + "str")

#---

#! runtime error: type Num is not iterable

each item in 1 do
	$print(item)
end

#---

#! syntax error: unexpected token NUM

$print(1 2)

#---

#! syntax error: unexpected character '@'

x := @

#---

#! syntax error: unknown builtin '$unknown'

$unknown(1)

#---

#! runtime error: missing arg 'a'

def one(a)
	a
end
one

#---

#! runtime error: member 'greet' on type Greeter is read-only

type Greeter
	def greet()
		"hi"
	end
end

g := Greeter()
g.greet := "nope"

#---

#! runtime error: member 'greet' on type Greeter is read-only

type Greeter
	def greet()
		"hi"
	end

	def rename()
		greet := "nope"
	end
end

Greeter().rename()

#---

#! runtime error: too many args; have 3, want at most 2

def pair(a, b)
	b
end
pair(1, 2, 3)

#---

#! runtime error: missing arg 'unit'

type Distance(amount, unit)
end
Distance(45.7)

#---

#! runtime error: no param named 'x'

def named(a, b: 1)
	a
end
named(1, x: 2)

#---

#! runtime error: arg 'a' given twice

def twice(a, b: 1)
	a
end
twice(1, a: 2)

#---

#! runtime error: missing args 'a', 'b'

def needs(a, b, c: 1)
	a
end
needs()

#---

#! runtime error: name 'unknown' is not defined

type Ctor(a, b: unknown)
end
Ctor(1)

#---

#! runtime error: index -1 is out of bounds

$print([1, 2, 3][-1])

#---

#! runtime error: index 1.7 is not a whole number

$print([1, 2, 3][1.7])

#---

#! runtime error: index 3 is out of bounds

$print([1, 2, 3][3])

#---

#! runtime error: index -1 is out of bounds

xs := [1, 2, 3]
xs[-1] := 0

#---

#! runtime error: index a is not a number

$print([1, 2, 3]["a"])

#---

#! runtime error: key "b" not found

$print({"a": 1}["b"])

#---

#! runtime error: type Dict is not appendable

{"id": 105} << "name"

#---

#! runtime error: type Num is not invokable

$print(&1)

#---

#! runtime error: type Num cannot be concatenated

$print("str" + 5.1)

#---

#! runtime error: type Char cannot be constructed

Char()
