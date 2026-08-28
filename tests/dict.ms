dict := {"id": 105}
#> {"id": 105}
$print(dict)

dict["name"] := "Alice"
#> Alice
$print(dict["name"])

#> 2
$print(dict.size)

dict["note"] := nil
#> nil
$print(dict["note"])
#> 3
$print(dict.size)

#> 105
each key in dict do
	when key == "id" then
		 $print(dict[key])
	end
end

#---

#! runtime error: key "b" not found

$print({"a": 1}["b"])

#---

#! runtime error: type Dict is not appendable

{"id": 105} << "name"
