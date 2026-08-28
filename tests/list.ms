list := [1, 0, 3]
list[1] := 2

#> [1, 2, 3]
$print(list)
#> 3
$print(list.size)
#> 1
$print(list[0])
#> 3
$print(list[2])

#> 1
#> 2
#> 3
each item in [1, 2, 3] do
	$print(item)
end

list := []
list << 1
list << 2 << 3

#> [1, 2, 3]
$print(list)

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

#! runtime error: index "a" is not a number

$print([1, 2, 3]["a"])
