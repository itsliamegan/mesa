import Core.Protos.Hash

type Task(priority)
	impl Hash
end

type Group(task)
	impl Hash

	status := "queued"
end

dict := {}
dict[Task(1)] := "first"
#> first
$print(dict[Task(1)])

dict[Group(Task(2))] := "group"
#> group
$print(dict[Group(Task(2))])

type Node(next)
	impl Hash
end

self_loop := Node(nil)
self_loop.next := self_loop

two_cycle := Node(nil)
two_cycle_next := Node(two_cycle)
two_cycle.next := two_cycle_next

#> true
$print(self_loop == two_cycle)
dict[self_loop] := "cycle"
#> cycle
$print(dict[two_cycle])

#---

#! runtime error: type Task is not hashable

type Task(priority)
end

{}[Task(1)]

#---

#! runtime error: type List is not hashable

import Core.Protos.Hash

type Group(tasks)
	impl Hash
end

{Group([Task(1)]): "group"}

type Task(priority)
end

#---

import Core.Protos.Hash

type Task(priority)
	impl Hash

	def hash
		$print("custom hash")
		1
	end
end

dict := {}
#> custom hash
dict[Task(1)] := "task"
#> custom hash
#> task
$print(dict[Task(1)])

#---

import Core.Protos.Hash

type Task(priority)
	impl Hash

	def equal(other)
		true
	end

	def hash
		0
	end
end

dict := {Task(1): "task"}
#> task
$print(dict[Task(2)])

#---

#! runtime error: type Task is not hashable

import Core.Protos.Hash

type Task(priority)
	impl Hash

	def hash
		"not a number"
	end
end

{Task(1): "task"}

#---

import Core.Protos.Hash

type Task
	case Ready(priority)
		impl Hash

		def equal(other)
			true
		end
	end

	def hash
		0
	end
end

dict := {Task.Ready(1): "ready"}
#> ready
$print(dict[Task.Ready(2)])

#---

#! semantic error: type 'Task' must declare 'hash' when overriding 'equal' and implementing 'Hash'

import Core.Protos.Hash

type Task(priority)
	impl Hash

	def equal(other)
		true
	end
end

#---

import Core.Protos.Hash
import Test.Left

def run
	"run"
end

type Task
	def run
		"task"
	end
end

proc := &run
method := &Task().run

dict := {}
dict[proc] := "proc"
dict[method] := "method"
dict[Task] := "type"
dict[Left] := "module"

#> proc
$print(dict[proc])
#> method
$print(dict[method])
#> type
$print(dict[Task])
#> module
$print(dict[Left])

#:left.ms
module Test.Left
