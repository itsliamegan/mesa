#! sem_error_proto.ms:3,1: semantic error: protocol 'Mixin' provides members but requires none

proto Mixin
	def helper()
		return 1
	end
end

#---

#! sem_error_proto.ms:13,1: semantic error: unknown protocol 'Ordering'

type Task(priority)
	impl Ordering
end

#---

#! sem_error_proto.ms:25,1: semantic error: type 'Task' does not implement 'compare', required by protocol 'Order'

proto Order
	def compare(other) end
end

type Task(priority)
	impl Order
end

#---

#! sem_error_proto.ms:40,2: semantic error: 'compare' on type 'Task' does not match the signature declared by protocol 'Order'

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

#! sem_error_proto.ms:56,2: semantic error: 'at' on type 'Row' does not match the signature declared by protocol 'Sliced'

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

#! sem_error_proto.ms:81,1: semantic error: protocols 'Order' and 'Named' both provide 'label'

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

#! sem_error_proto.ms:112,2: semantic error: member 'label' provided by protocol 'Order' collides with a member of type 'Task'

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

#! sem_error_proto.ms:124,14: semantic error: provided body cannot reference 'self.priority', which protocol 'Order' does not declare

proto Order
	def compare(other) end

	def min(other)
		return self.priority
	end
end
