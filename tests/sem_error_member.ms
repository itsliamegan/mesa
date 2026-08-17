#! semantic error: duplicate member 'encode'

def encode()
end
def encode()
end

#---

#! semantic error: duplicate member 'Request'

type Request
end
def Request()
end

#---

#! semantic error: duplicate member 'Order'

proto Order
	def before(other) end
end
type Order
end

#---

x := 1
x := 2
def f()
end

#---

#! semantic error: duplicate member 'count'

count := 1
def count()
end
