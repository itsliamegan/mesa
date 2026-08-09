#> {"id": 105}
#> Alice
#> 2
#> 105

dict := {"id": 105}
$print(dict)

dict["name"] := "Alice"
$print(dict["name"])

$print(dict.size)

each key in dict do
	when key == "id" then
		 $print(dict[key])
	end
end
