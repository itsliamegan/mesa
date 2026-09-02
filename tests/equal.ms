import Core.Protos.Equal

$print((1).equal(1))
#> true
$print("a".equal("b"))
#> false
$print(nil.equal(nil))
#> true
$print((1).equal("1"))
#> false

#> true
$print(1 == 1)
#> true
$print(1 != 2)
#> true
$print(nil == nil)
#> true
$print(true != false)
#> true
$print('a' == 'a')
#> true
$print("a" == "a")
#> false
$print(1 == "1")

#> true
$print([1, 2] == [1, 2])
#> true
$print([[1], [2]] == [[1], [2]])
#> true
$print([1] != [1, 1])
#> true
$print({"a": 1, "b": 2} == {"b": 2, "a": 1})
#> true
$print({"items": [1, 2]} == {"items": [1, 2]})
#> true
$print([{"a": 1}] == [{"a": 1}])
#> false
$print({"a": 1} == {"b": 1})

type Task(priority)
end

type Group(tasks: [])
	status := "open"
end

type OtherTask(priority)
end

#> true
$print(Task(5) == Task(5))
#> true
$print(Task(5) != Task(2))
#> true
$print(Group([Task(5)]) == Group([Task(5)]))
#> true
left := Group([Task(5)])
right := Group([Task(5)])
right.status := "done"
$print(left != right)
#> false
$print(Task(5) == OtherTask(5))

type Transaction
	case Credit(amount) end
	case Debit(amount) end
end

#> false
$print(Transaction.Credit(1) == Transaction.Debit(1))
#> true
$print(Transaction.Credit(1) != Transaction)

type AlwaysEqual(value)
	def equal(other)
		"equal"
	end
end

#> true
$print(AlwaysEqual(1) == AlwaysEqual(2))
#> true
$print(AlwaysEqual(1) == Task(1))
#> true
$print(Group([AlwaysEqual(1)]) == Group([AlwaysEqual(2)]))
#> true
$print([AlwaysEqual(1)] == [AlwaysEqual(2)])

type NeverEqual(value)
	impl Equal

	def equal(other)
		false
	end
end

#> false
$print(NeverEqual(1) == NeverEqual(1))

type Derived(value)
	impl Equal
end

#> true
$print(Derived(1) == Derived(1))

self_list := []
self_list << self_list
other_self_list := []
other_self_list << other_self_list
#> true
$print(self_list == other_self_list)

two_cycle_a := []
two_cycle_b := []
two_cycle_a << two_cycle_b
two_cycle_b << two_cycle_a
#> true
$print(self_list == two_cycle_a)

marked_a := []
marked_a << marked_a
marked_b := []
marked_b << marked_b
#> true
$print([marked_a, marked_a] != [marked_b, 5])

left_dict := {}
left_dict["self"] := left_dict
right_dict := {}
right_dict["self"] := right_dict
#> true
$print(left_dict == right_dict)

type Node(next)
end

left_node := Node(nil)
left_node.next := left_node
right_node := Node(nil)
right_node.next := right_node
#> true
$print(left_node == right_node)

type Link(next)
	def equal(other)
		self.next == other.next
	end
end

left_link := Link(nil)
left_link.next := left_link
right_link := Link(nil)
right_link.next := right_link
#> true
$print(left_link == right_link)

type Flaky(raising)
	def equal(other)
		when self.raising then
			self.raising := false
			raise "boom"
		end
		false
	end
end

flaky := Flaky(true)
other_flaky := Flaky(false)
do
	flaky == other_flaky
rescue e case Str then
	nil
end
#> false
$print(flaky == other_flaky)

#---

type Node(data)
end

a := Node(nil)
a.data := a

b := Node(nil)
c := Node(b)
b.data := c

#> true
$print(a == b)

#---

import Test.Left
import Test.Right

type First
end

type Second
end

def first
end

def second
end

#> true
$print(Left == Left)
#> true
$print(Left != Right)
#> true
$print(First == First)
#> true
$print(First != Second)
#> true
$print(&first == &first)
#> true
$print(&first != &second)

#:left.ms
module Test.Left

#:right.ms
module Test.Right

#---

#! runtime error: "boom"

type Explosive(value)
	def equal(other)
		raise "boom"
	end
end

Explosive(1) == Explosive(1)
