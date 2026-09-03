import Core.Behaviors.Hash

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

import Core.Behaviors.Hash

type Task(priority)
	impl Hash
end

#> true
$print(nil.hash != (0).hash)
#> true
$print((0).hash != false.hash)
#> true
$print(false.hash != true.hash)
#> true
$print(Task.hash != Task(1).hash)
#> true
$print(Task(1).hash != Task(2).hash)
#> true
$print((0).hash == (0 * (0 - 1)).hash)
#> 1
zero := {}
zero[0] := "pos"
zero[0 * (0 - 1)] := "neg"
$print(zero.size)

#---

#! runtime error: type Task is not hashable

type Task(priority)
end

{}[Task(1)]

#---

#! runtime error: type List is not hashable

import Core.Behaviors.Hash

type Group(tasks)
	impl Hash
end

{Group([Task(1)]): "group"}

type Task(priority)
end

#---

import Core.Behaviors.Hash

type Task(priority)
	impl Hash

	def hash
		$print("custom hash")
		(1).hash
	end
end

dict := {}
#> custom hash
dict[Task(1)] := "task"
#> custom hash
#> task
$print(dict[Task(1)])

#---

import Core.Behaviors.Hash

type Task(priority)
	impl Hash

	def equal(other)
		true
	end

	def hash
		(0).hash
	end
end

dict := {Task(1): "task"}
#> task
$print(dict[Task(2)])

#---

import Core.Behaviors.Hash

type Meddler(id)
	impl Hash

	dict := nil

	def hash
		(0).hash
	end

	def equal(other)
		when self.dict != nil then
			meddled := self.dict
			self.dict := nil
			meddled[Meddler(other.id)] := "meddled"
		end
		self.id == other.id
	end
end

dict := {}
first := Meddler(1)
dict[first] := "first"
first.dict := dict
dict[Meddler(2)] := "second"

#> 2
$print(dict.size)
#> second
$print(dict[Meddler(2)])

#---

#! runtime error: type Task is not hashable

import Core.Behaviors.Hash

type Task(priority)
	impl Hash

	def hash
		"not a number"
	end
end

{Task(1): "task"}

#---

import Core.Behaviors.Hash

type Account(id, name)
	impl Hash

	def hash
		self.id.hash + self.name.hash
	end
end

#> true
$print(Account(1, "a").hash == Account(1, "a").hash)
#> true
$print(Account(1, "a").hash != Account(2, "a").hash)
#> true
$print(Account(1, "a").hash != Account(1, "b").hash)
#> true
$print(Account(1, 2).hash != Account(2, 1).hash)

dict := {}
dict[Account(1, "a")] := "first"
dict[Account(2, "a")] := "second"
dict[Account(1, "b")] := "third"
#> 3
$print(dict.size)
#> second
$print(dict[Account(2, "a")])

#---

#! runtime error: type Num cannot be mixed into a digest

import Core.Behaviors.Hash

(1).hash + 5


#---

import Core.Behaviors.Hash

type Task
	case Ready(priority)
		impl Hash

		def equal(other)
			true
		end
	end

	def hash
		(0).hash
	end
end

dict := {Task.Ready(1): "ready"}
#> ready
$print(dict[Task.Ready(2)])

#---

#! semantic error: type 'Task' must declare 'hash' when overriding 'equal' and implementing 'Hash'

import Core.Behaviors.Hash

type Task(priority)
	impl Hash

	def equal(other)
		true
	end
end

#---

import Core.Behaviors
import Core.Behaviors.Hash
import Test.Left
import Test.Left.Ready

def run
	"run"
end

type Task
	def run
		"task"
	end
end

proto Waiting
	def wait end
end

proc := &run
method := &Task().run

dict := {}
dict[proc] := "proc"
dict[method] := "method"
dict[Task] := "type"
dict[Left] := "module"
dict[Waiting] := "proto"
dict[Ready] := "imported proto"
dict[Hash] := "stdlib proto"

#> proc
$print(dict[proc])
#> method
$print(dict[method])
#> type
$print(dict[Task])
#> module
$print(dict[Left])
#> proto
$print(dict[Waiting])
#> imported proto
$print(dict[Left.Ready])
#> stdlib proto
$print(dict[Behaviors.Hash])
#> 7
$print(dict.size)

#:left.ms
module Test.Left

proto Ready
	def ready end
end
