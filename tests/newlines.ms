#> [10, 20, 30]

y := [10, 20, 30]
x := y
[1]
$print(x)

#---

#> 3

total := 1 +
	2
$print(total)

#---

#> nil

def wrapped()
	return
	5.23
end
$print(wrapped())

#---

#> 1

total := 1
- 2
$print(total)

#---

#> 9

total := 1
	+ 2 * 4
$print(total)

#---

#> 2

s := "ab"
y := s
	.size
$print(y)

#---

#> true

ready := true
count := 5
result := ready
	and count > 0
$print(result)

#---

#! syntax error: unexpected token EQ

x
:= 5

#---

#> 9

def compute
	(1 + 2) * 3
end
$print(compute)

#---

#> 7

def add(a, b)
	a + b
end
$print(add(3, 4))

#---

#> 9

type Box
	def compute
		(1 + 2) * 3
	end
end
$print(Box().compute)
