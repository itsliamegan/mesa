dict := {"id": 105}
#> {"id": 105}
$print(dict)

dict["name"] := "Alice"
#> Alice
$print(dict["name"])

#> 2
$print(dict.size)

#> Alice
$print(dict.access("name"))
dict.store("status", "active")
#> active
$print(dict.access("status"))

dict["note"] := nil
#> nil
$print(dict["note"])
#> 4
$print(dict.size)

#> 105
each key in dict do
	when key == "id" then
		 $print(dict[key])
	end
end

dict := {}
dict["self"] := dict
#> {"self": ...}
$print(dict)

#---

#! runtime error: key "b" not found

$print({"a": 1}["b"])

#---

#! runtime error: type Dict is not appendable

{"id": 105} << "name"
