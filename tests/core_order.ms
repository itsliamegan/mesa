#> 3
#> 5
#> b
#> a
#> a
#> z

x := 5
y := 3
$print(x.min(y))
$print(x.max(y))
$print("a".max("b"))
$print("a".min("b"))
$print('a'.min('z'))
$print('a'.max('z'))

#---

#> true
#> false
#> 10

$print("".empty?)
$print("x".empty?)
$print([10, 20].first)

#---

#> true
#> false
#> true
#> false

import Core.Order

type Task(priority)
	impl Order

	def order(other)
		self.priority - other.priority
	end
end

$print(Task(1) < Task(5))
$print(Task(1) > Task(5))
$print(Task(5) >= Task(1))
$print(Task(1) >= Task(5))

#---

#! runtime error: type Task is not orderable

import Core.Order

type Task(priority)
	impl Order

	def order(other)
		self.priority - other.priority
	end
end

$print(Task(1) < 5)

#---

#! runtime error: type Rogue is not orderable

# Defines 'order' but never declares 'impl Order' — a same-named member does
# not make a type conform.
type Rogue(n)
	def order(other)
		self.n - other.n
	end
end

$print(Rogue(1) < Rogue(2))
