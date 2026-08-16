def encode()
end
def encode()
end
#! src/package.ms:4,1: semantic error: duplicate member 'encode'
#---
type Request
end
def Request()
end
#! src/package.ms:10,1: semantic error: duplicate member 'Request'
#---
proto Order
	def before(other) end
end
type Order
end
#! src/package.ms:17,1: semantic error: duplicate member 'Order'
#---
x := 1
x := 2
def f()
end
#---
count := 1
def count()
end
#! src/package.ms:27,1: semantic error: duplicate member 'count'
