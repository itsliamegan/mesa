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

#! newlines.ms:31,1: syntax error: unexpected token PLUS

total := 1
+ 2
