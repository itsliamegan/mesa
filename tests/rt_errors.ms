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

#! rt_errors.ms:33,7: runtime error: wrong number of args; have 1, want 2
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

#! rt_errors.ms:70,6: syntax error: unexpected char '@'

x := @

#---

#! rt_errors.ms:76,1: syntax error: unknown builtin '$unknown'

$unknown(1)
