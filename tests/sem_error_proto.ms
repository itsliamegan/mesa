#! semantic error: protocol 'Mixin' provides members but requires none

proto Mixin
	def helper()
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

#! semantic error: 'at' on type 'Row' does not match the signature declared by protocol 'Sliced'

proto Sliced
	def at(i, step: 1) end
end

type Row(cells)
	impl Sliced

	def at(i, step: 2)
		return i
	end
end

#---

#! semantic error: protocols 'Order' and 'Named' both provide 'label'

proto Order
	def compare(other) end

	def label()
		return "order"
	end
end

proto Named
	def name() end

	def label()
		return "named"
	end
end

type Task(priority)
	impl Order, Named

	def compare(other)
		return 0
	end

	def name()
		return "task"
	end
end

#---

#! semantic error: member 'label' provided by protocol 'Order' collides with a member of type 'Task'

proto Order
	def compare(other) end

	def label()
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

#! semantic error: 'Sub.Thing.Extra' is not a protocol

import Test.Sub

type Task(priority)
	impl Sub.Thing.Extra
end

#:sub.ms
module Test.Sub

type Thing(x)
end
