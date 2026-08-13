#! rt_errors.ms:4,8: runtime error: name 'unknown' is not defined
#! 	at <main> (rt_errors.ms:4,8)

$print(unknown())

#---

#! rt_errors.ms:13,2: runtime error: name 'inner' is not defined
#! 	at outer (rt_errors.ms:13,2)
#! 	at <main> (rt_errors.ms:15,6)

def outer()
	inner()
end
outer()

#---

#! rt_errors.ms:23,4: runtime error: type Num is not callable
#! 	at <main> (rt_errors.ms:23,4)

num := 1
num()

#---

#! rt_errors.ms:33,7: runtime error: missing arg 'b'
#! 	at <main> (rt_errors.ms:33,7)

def either(a, b)
	b
end
either(2)

#---

#! rt_errors.ms:42,12: runtime error: type T has no such member 'unknown'
#! 	at <main> (rt_errors.ms:42,12)

type T(a)
end
$print(T(1).unknown)

#---

#! rt_errors.ms:49,12: runtime error: type Str cannot be used in arithmetic
#! 	at <main> (rt_errors.ms:49,12)

$print(5.1 + "str")

#---

#! rt_errors.ms:56,1: runtime error: type Num is not iterable
#! 	at <main> (rt_errors.ms:56,1)

each item in 1 do
	$print(item)
end

#---

#! rt_errors.ms:64,10: syntax error: unexpected token NUM

$print(1 2)

#---

#! rt_errors.ms:70,6: syntax error: unexpected character '@'

x := @

#---

#! rt_errors.ms:76,1: syntax error: unknown builtin '$unknown'

$unknown(1)

#---

#! rt_errors.ms:86,1: runtime error: missing arg 'a'
#! 	at <main> (rt_errors.ms:86,1)

def one(a)
	a
end
one

#---

#! rt_errors.ms:100,9: runtime error: member 'greet' on type Greeter is read-only
#! 	at <main> (rt_errors.ms:100,9)

type Greeter
	def greet()
		"hi"
	end
end

g := Greeter()
g.greet := "nope"

#---

#! rt_errors.ms:114,9: runtime error: member 'greet' on type Greeter is read-only
#! 	at rename (rt_errors.ms:114,9)
#! 	at <main> (rt_errors.ms:118,17)

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

#! rt_errors.ms:128,5: runtime error: too many args; have 3, want at most 2
#! 	at <main> (rt_errors.ms:128,5)

def pair(a, b)
	b
end
pair(1, 2, 3)

#---

#! rt_errors.ms:137,9: runtime error: missing arg 'unit'
#! 	at <main> (rt_errors.ms:137,9)

type Distance(amount, unit)
end
Distance(45.7)

#---

#! rt_errors.ms:147,6: runtime error: no param named 'x'
#! 	at <main> (rt_errors.ms:147,6)

def named(a, b: 1)
	a
end
named(1, x: 2)

#---

#! rt_errors.ms:157,6: runtime error: arg 'a' given twice
#! 	at <main> (rt_errors.ms:157,6)

def twice(a, b: 1)
	a
end
twice(1, a: 2)

#---

#! rt_errors.ms:167,6: runtime error: missing args 'a', 'b'
#! 	at <main> (rt_errors.ms:167,6)

def needs(a, b, c: 1)
	a
end
needs()

#---

#! rt_errors.ms:174,17: runtime error: name 'nope' is not defined
#! 	at <main> (rt_errors.ms:174,17)

type Ctor(a, b: nope)
end
Ctor(1)

#---

#! rt_errors.ms:183,17: runtime error: index -1 is out of bounds
#! 	at <main> (rt_errors.ms:183,17)

$print([1, 2, 3][-1])

#---

#! rt_errors.ms:190,17: runtime error: index 1.7 is not a whole number
#! 	at <main> (rt_errors.ms:190,17)

$print([1, 2, 3][1.7])

#---

#! rt_errors.ms:197,17: runtime error: index 3 is out of bounds
#! 	at <main> (rt_errors.ms:197,17)

$print([1, 2, 3][3])

#---

#! rt_errors.ms:205,8: runtime error: index -1 is out of bounds
#! 	at <main> (rt_errors.ms:205,8)

xs := [1, 2, 3]
xs[-1] := 0

#---

#! rt_errors.ms:212,17: runtime error: index a is not a number
#! 	at <main> (rt_errors.ms:212,17)

$print([1, 2, 3]["a"])

#---

#! rt_errors.ms:219,16: runtime error: key "b" not found
#! 	at <main> (rt_errors.ms:219,16)

$print({"a": 1}["b"])
