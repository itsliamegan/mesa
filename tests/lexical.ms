#> Hello, world!
$print("Hello, world!")
#> 5.23
$print(5.23)
#> true
$print(true)
#> [1, 7]
$print([1, 7])
#> {"id": 774}
$print({"id": 774})
#> nil
$print(nil)
#> a
$print('a')
#> é
$print('é')

#> Multi-
#> line
$print("Multi-\nline")
#> Tabu	lated
$print("Tabu\tlated")
#> "Quoted"
$print("\"Quoted\"")
#> Back\slash
$print("Back\\slash")
#> 
#> 
$print('\n')
#> 	
$print('\t')
#> \
$print('\\')
#> '
$print('\'')
#> "
$print('"')

# A comment.
#> nil
$print(nil) # A trailing comment.

# Another comment with space.

# A # nested comment.

#---

y := [10, 20, 30]
x := y
[1]
#> [10, 20, 30]
$print(x)

#---

total := 1 +
	2
#> 3
$print(total)

#---

def wrapped
	return
	5.23
end
#> nil
$print(wrapped)

#---

total := 1
- 2
#> 1
$print(total)

#---

total := 1
	+ 2 * 4
#> 9
$print(total)

#---

s := "ab"
y := s
	.size
#> 2
$print(y)

#---

ready := true
count := 5
result := ready
	and count > 0
#> true
$print(result)

#---

def compute
	(1 + 2) * 3
end
#> 9
$print(compute)

#---

def add(a, b)
	a + b
end
#> 7
$print(add(3, 4))

#---

type Task(priority)
	def compute
		(1 + 2) * 3
	end
end
#> 9
$print(Task(1).compute)
