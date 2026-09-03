import Core.Protos.Hash

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

type Task(priority)
	impl Hash

	def hash
		(0).hash
	end
end

collisions := {}
collisions[Task(1)] := "first"
collisions[Task(2)] := "second"
collisions[Task(1)] := "updated"
#> updated
$print(collisions[Task(1)])
#> second
$print(collisions[Task(2)])
#> 1
#> 2
each task in collisions do
	$print(task.priority)
end

literal := {Task(1): "first", Task(2): "second", Task(1): "updated"}
#> {Task(priority: 1): "updated", Task(priority: 2): "second"}
$print(literal)

#---

#! runtime error: key "b" not found

$print({"a": 1}["b"])

#---

#! runtime error: type Dict is not appendable

{"id": 105} << "name"

#---

#! runtime error: type List is not hashable

dict := {}
dict[[1]] := 1

#---

#! runtime error: type Dict is not hashable

key := {}
{key: 1}

#---

#! runtime error: type Task is not hashable

type Task(priority) end

{}[Task(1)]

#---

type Task(priority) end

dict := {}
dict[Task] := 1
#> 1
$print(dict[Task])
