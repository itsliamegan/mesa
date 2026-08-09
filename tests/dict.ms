#> {"id": 105}
#> Alice
#> 2
#> 2

dict := {"id": 105}
$print(dict)

dict["name"] := "Alice"
$print(dict["name"])

$print(dict.size)

size := 0
each key in dict do
	size := size + 1
end
$print(size)
