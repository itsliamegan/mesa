proto Order
	def compare(other) end

	def min(other)
		when self.compare(other) <= 0 then return self else return other end
	end

	def max(other)
		when self.compare(other) <= 0 then return other else return self end
	end
end

type Task(priority, name)
	impl Order

	def compare(other)
		return self.priority - other.priority
	end
end

#> proto Order
$print(Order)

low := Task(1, "low")
high := Task(5, "high")
#> low
$print(low.min(high).name)
#> high
$print(low.max(high).name)
#> low
$print(high.min(low).name)
#> high
$print(high.max(low).name)

#---

proto Sized
	def size end

	def empty
		return self.size == 0
	end

	def full
		return not self.empty
	end
end

type Group(tasks: [])
	impl Sized

	def size
		return self.tasks.size
	end
end

type Task(priority)
	impl Sized

	def size
		return self.priority
	end

	def full
		return "always full"
	end
end

#> true
$print(Group().empty)
#> false
$print(Group(tasks: [1, 2, 3]).empty)
#> false
$print(Group().full)
#> true
$print(Group(tasks: [1, 2, 3]).full)
#> always full
$print(Task(0).full)

#---

type Task(priority)
	impl Power

	def of
		return self.priority
	end
end

proto Power
	def of end

	def pow(n)
		when n == 0 then return 1 end
		return self.of * self.pow(n - 1)
	end
end

#> 32
$print(Task(2).pow(5))
#> 1
$print(Task(7).pow(0))

#---

proto Order
	def compare(other) end

	def min(other)
		when self.compare(other) <= 0 then return self else return other end
	end

	def max(other)
		when self.compare(other) <= 0 then return other else return self end
	end
end

type Transaction
	impl Order

	case Credit(amount)
	end

	case Debit(amount)
		impl Order

		def compare(other)
			return self.amount - other.amount
		end
	end

	def compare(other)
		return 0
	end
end

#> 2
$print(Transaction.Credit(2).min(Transaction.Credit(9)).amount)
#> 9
$print(Transaction.Debit(9).max(Transaction.Debit(4)).amount)
#> 4
$print(Transaction.Debit(9).min(Transaction.Debit(4)).amount)

#---

proto Named
	def name end

	def label
		return "provided"
	end
end

type Transaction
	case Credit(note)
		impl Named
		def name
			return self.note
		end
	end

	def label
		return "parent"
	end
end

entry := Transaction.Credit("x")
#> parent
$print(entry.label)
#> x
$print(entry.name)

#---

x := 5
y := 3
#> 3
$print(x.min(y))
#> 5
$print(x.max(y))
#> b
$print("a".max("b"))
#> a
$print("a".min("b"))
#> a
$print('a'.min('z'))
#> z
$print('a'.max('z'))

#---

import Core.Protos.Order

type Task(priority)
	impl Order

	def order(other)
		self.priority - other.priority
	end
end

#> true
$print(Task(1) < Task(5))
#> false
$print(Task(1) > Task(5))
#> true
$print(Task(5) >= Task(1))
#> false
$print(Task(1) >= Task(5))

#---

import Core.Protos.Display

type Task(priority)
	impl Display

	def inspect
		return "inspected"
	end

	def display
		return self.inspect
	end
end

#> inspected
$print(Task(1).inspect)
#> inspected
$print(Task(1).display)

#---

import Core.Protos.Display

type Task(priority)
	impl Display

	def display
		return self.inspect
	end
end

#> Task(priority: 1)
$print(Task(1).inspect)
#> Task(priority: 1)
$print(Task(1).display)

#---

#! semantic error: 'inspect' on type 'Task' does not match the signature declared by protocol 'Inspect'

import Core.Protos.Display

type Task(priority)
	impl Display

	def inspect(other)
		return "inspected"
	end

	def display
		return "task"
	end
end

#---

proto Equal
	def equal(other) end

	def label
		return "equal"
	end
end

proto Hash impl Equal
	def hash end

	def label
		return "hash"
	end

	def same_priority(other)
		return self.equal(other)
	end
end

type Task(priority)
	impl Hash

	def equal(other)
		return self.priority == other.priority
	end

	def hash
		return self.priority
	end
end

#> true
$print(Task(1).equal(Task(1)))
#> hash
$print(Task(1).label)
#> true
$print(Task(1).same_priority(Task(1)))

#---

proto Equal
	def equal(other) end
end

proto Hash impl Equal
	def hash end
end

proto Order impl Equal
	def compare(other) end
end

type Task(priority)
	impl Hash, Order

	def equal(other)
		return self.priority == other.priority
	end

	def hash
		return self.priority
	end

	def compare(other)
		return self.priority - other.priority
	end
end

type Group(tasks: [])
	impl Order, Hash

	def equal(other)
		return self.tasks.size == other.tasks.size
	end

	def hash
		return self.tasks.size
	end

	def compare(other)
		return self.tasks.size - other.tasks.size
	end
end

#> true
$print(Task(1).equal(Task(1)))
#> true
$print(Group(tasks: [1]).equal(Group(tasks: [2])))

#---

proto Equal
	def equal(other) end
end

proto Hash impl Equal
	def hash end
end

type Task(priority)
	impl Equal, Hash

	def equal(other)
		return self.priority == other.priority
	end

	def hash
		return self.priority
	end
end

#> true
$print(Task(1).equal(Task(1)))

#---

proto Equal
	def equal(other) end
end

proto Hash impl Equal
	def hash()
		return 0
	end
end

#---

#! semantic error: protocol 'Mixin' provides methods but requires none

proto Mixin
	def helper
		return 1
	end
end

#---

#! semantic error: unknown protocol 'Ordering'

type Task(priority)
	impl Ordering
end

#---

#! semantic error: type 'Task' does not implement 'compare', required by protocol 'Order'

proto Order
	def compare(other) end
end

type Task(priority)
	impl Order
end

#---

#! semantic error: 'compare' on type 'Task' does not match the signature declared by protocol 'Order'

proto Order
	def compare(other) end
end

type Task(priority)
	impl Order

	def compare(thing)
		return 0
	end
end

#---

#! semantic error: 'at' on type 'Task' does not match the signature declared by protocol 'Sliced'

proto Sliced
	def at(i, step: 1) end
end

type Task(priority)
	impl Sliced

	def at(i, step: 2)
		return i
	end
end

#---

#! semantic error: protocols 'Order' and 'Named' both provide 'label'

proto Order
	def compare(other) end

	def label
		return "order"
	end
end

proto Named
	def name end

	def label
		return "named"
	end
end

type Task(priority)
	impl Order, Named

	def compare(other)
		return 0
	end

	def name
		return "task"
	end
end

#---

#! semantic error: method 'label' provided by protocol 'Order' collides with a member of type 'Task'

proto Order
	def compare(other) end

	def label
		return "order"
	end
end

type Task(priority)
	impl Order

	def compare(other)
		return 0
	end

	type label
	end
end

#---

#! semantic error: provided body cannot reference 'self.priority', which protocol 'Order' does not declare

proto Order
	def compare(other) end

	def min(other)
		return self.priority
	end
end

#---

#! semantic error: 'Sub' is not a protocol

import Test.Sub

type Task(priority)
	impl Sub
end

#:sub.ms
module Test.Sub

#---

#! semantic error: 'Sub.Tag.Extra' is not a protocol

import Test.Sub

type Task(priority)
	impl Sub.Tag.Extra
end

#:sub.ms
module Test.Sub

type Tag(label)
end

#---

#! semantic error: type 'Task' does not implement 'equal', required by protocol 'Equal'

proto Equal
	def equal(other) end
end

proto Hash impl Equal
	def hash end
end

type Task(priority)
	impl Hash

	def hash
		return self.priority
	end
end

#---

#! semantic error: protocol cycle: Draw → Erase → Draw

proto Draw impl Erase
	def draw end
end

proto Erase impl Draw
	def erase end
end

#---

#! semantic error: protocol 'Order' is named twice

proto Order
	def compare(other) end
end

type Task(priority)
	impl Order, Order
end

#---

#! semantic error: protocol 'Equal' is named twice

proto Equal
	def equal(other) end
end

proto Hash impl Equal, Equal
	def hash end
end

#---

#! semantic error: protocol 'Sub.Equal' is named twice

import Test.Sub.Equal
import Test.Sub

type Task(priority)
	impl Equal, Sub.Equal
end

#:sub.ms
module Test.Sub

proto Equal
	def equal(other) end
end

#---

#! runtime error: type Task is not orderable

proto Order
	def order(other) end
end

type Task(priority)
	impl Order

	def order(other)
		self.priority - other.priority
	end
end

$print(Task(1) < Task(2))

#---

#! runtime error: type Task is not orderable

import Core.Protos.Order

type Task(priority)
	impl Order

	def order(other)
		self.priority - other.priority
	end
end

$print(Task(1) < 5)

#---

#! runtime error: type Task is not orderable

type Task(priority)
	def order(other)
		self.priority - other.priority
	end
end

$print(Task(1) < Task(2))

#---

#> x
$print("x".display)
#> "x"
$print("x".inspect)
#> a
$print('a'.display)
#> 'a'
$print('a'.inspect)

#> Multi-
#> line
$print("Multi-\nline".display)
#> "Multi-\nline"
$print("Multi-\nline".inspect)

#---

#! semantic error: type 'Task' does not implement 'display', required by protocol 'Display'

import Core.Protos.Display

type Task(priority)
	impl Display
end

$print(Task(1))

#---

#! runtime error: type Task is not inspectable

type Task(name)
	def inspect
		0
	end
end

$print(Task("a"))

#---

#! runtime error: type Task is not displayable

import Core.Protos.Display

type Task(name)
	impl Display

	def display
		0
	end
end

$print(Task("a"))
