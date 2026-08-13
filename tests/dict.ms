#> {"id": 105}
#> Alice
#> 2
#> nil
#> 3
#> 105

dict := {"id": 105}
$print(dict)

dict["name"] := "Alice"
$print(dict["name"])

$print(dict.size)

dict["note"] := nil
$print(dict["note"])
$print(dict.size)

each key in dict do
	when key == "id" then
		 $print(dict[key])
	end
end
