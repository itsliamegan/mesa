#> [1, 2, 3]
#> 3
#> 1
#> 2
#> 3

list := [1, 0, 3]
list[1] := 2

$print(list)
$print(list.size)

each item in list do
	$print(item)
end
