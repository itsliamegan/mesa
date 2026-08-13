#> [1, 2, 3]
#> 3
#> 1
#> 3
#> 1
#> 2
#> 3

list := [1, 0, 3]
list[1] := 2

$print(list)
$print(list.size)
$print(list[0])
$print(list[2])

each item in list do
	$print(item)
end
