#> [1, 2, 3]
#> 3
#> 1
#> 3

list := [1, 0, 3]
list[1] := 2

$print(list)
$print(list.size)
$print(list[0])
$print(list[2])

#---

#> 1
#> 2
#> 3

each item in [1, 2, 3] do
	$print(item)
end

#---

#> [1, 2, 3]

list := []
list << 1
list << 2 << 3

$print(list)
