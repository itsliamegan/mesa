type Task(name, priority)
	def copy
		self
	end
end

#> Task(name: "laundry", priority: 2)
$print(Task("laundry", 2))
#> laundry
$print(Task("laundry", 2).name)
#> Task(name: "laundry", priority: 2)
$print(Task("laundry", 2).copy)

copy := &Task("laundry", 2).copy
#> Task(name: "laundry", priority: 2)
$print(copy())

type Amount(cents)
	def self.of_dollars(n)
		Amount(n * 100)
	end

	def self.zero
		self(0)
	end

	def display
		self.cents
	end
end

#> Amount(cents: 500)
$print(Amount.of_dollars(5))
#> Amount(cents: 0)
$print(Amount.zero)
#> 0
$print(Amount.zero.display)

type Rect(w, h)
	area := w * h
	half_area := area / 2
end

r := Rect(4, 6)
#> Rect(w: 4, h: 6, area: 24, half_area: 12)
$print(r)
#> 24
$print(r.area)
#> 12
$print(r.half_area)

type Counter(start)
	count := start
end

c := Counter(5)
c.count := c.count + 1
#> 6
$print(c.count)

type LinkedList
	type Node(data, next)
		def display
			self.data
		end
	end

	def self.new_node
		LinkedList.Node(0, nil)
	end
end

node := LinkedList.Node(3, nil)
#> 3
$print(node.data)
#> 3
$print(node.display)
#> 0
$print(LinkedList.new_node.data)

type Group
	type Task(priority)
		type Tag(label)
		end
	end
end

tag := Group.Task.Tag("x")
#> x
$print(tag.label)

#> LinkedList.Node(data: 0, next: nil)
$print(LinkedList.Node(0, nil))
#> Group.Task.Tag(label: "x")
$print(tag)

#---

#! runtime error: type Stack is not hashable

type Stack(top)
	items := [1, 2]
	count := 0
end

{}[Stack(3)]

#---

#! runtime error: type Task has no such member 'unknown'

type Task(priority)
end
$print(Task(1).unknown)

#---

#! runtime error: member 'status' on type Task is read-only

type Task(priority)
	def status
		"queued"
	end
end

t := Task(1)
t.status := "done"

#---

#! runtime error: member 'status' on type Task is read-only

type Task(priority)
	def status
		"queued"
	end

	def close
		status := "done"
	end
end

Task(1).close

#---

#! runtime error: type Char cannot be constructed

Char()

#---

#! semantic error: type 'Task' declares member 'priority' more than once

type Task
	priority := 0
	priority := 1
end

#---

#! semantic error: type 'Task' declares member 'priority' more than once

type Task
	priority := 0

	def priority
		1
	end
end

#---

#! semantic error: type 'Task' declares member 'priority' more than once

type Task(priority)
	priority := 0
end

#---

#! semantic error: type 'Task' declares member 'priority' more than once

type Task(priority)
	def priority
		1
	end
end

#---

#! semantic error: type 'Task' declares member 'close' more than once

type Task
	def close
		1
	end

	def close
		2
	end
end

#---

#! semantic error: type 'Task' declares member 'urgent' more than once

type Task
	def self.urgent
		1
	end

	def self.urgent
		2
	end
end

#---

#! semantic error: type 'Transaction' declares member 'Void' more than once

type Transaction
	case Void
	end

	case Void
	end
end

#---

#! semantic error: type 'Transaction' declares member 'Void' more than once

type Transaction
	case Void
	end

	type Void
	end
end

#---

#! semantic error: duplicate param 'priority'

type Task(priority, priority)
end

#---

#! semantic error: duplicate param 'priority'

type Task
	def close(priority, priority)
	end
end

#---

#! semantic error: duplicate param 'priority'

type Task
	def self.at(priority, priority)
	end
end

#---

#> 1
#> 2

# An instance member and a static are separate namespaces, so one name can
# serve both.
type Task
	def priority
		1
	end

	def self.priority
		2
	end
end

$print(Task().priority)
$print(Task.priority)

#---

#> Task(name: "laundry", priority: 2)
#> a group
#> a group

type Task(name, priority)
end

type Group(items)
	def inspect
		"a group"
	end
end

$print(Task("laundry", 2).inspect)
$print(Group([1, 2]))
$print(Group([1, 2]).inspect)

#---

import Core.Protos.Display

type Group(items)
	impl Display

	def display
		"a group"
	end
end

#> a group
$print(Group([1, 2]))
#> Group(items: [1, 2])
$print(Group([1, 2]).inspect)

#---

type Task(name)
	def inspect
		"task"
	end
end

type Group(tasks)
end

#> task
$print(Task("a"))
#> [task, task]
$print([Task("a"), Task("b")])
#> {"task": task}
$print({"task": Task("b")})
#> Group(tasks: [task])
$print(Group([Task("a")]))

#---

type Node(n)
	def inspect
		[Node(self.n + 1)].inspect
		"node"
	end
end

#> node
$print(Node(0))
