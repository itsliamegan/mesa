module Core.Prelude

import Core.Protos.Access
import Core.Protos.Append
import Core.Protos.Display
import Core.Protos.Hash
import Core.Protos.Order

extern type Nil impl Hash
	extern def hash end
	extern def equal(other) end
	extern def inspect end
end

extern type Num impl Hash, Order
	extern def hash end
	extern def equal(other) end
	extern def order(other) end
	extern def inspect end
end

extern type Bool impl Hash
	extern def hash end
	extern def equal(other) end
	extern def inspect end
end

extern type Char impl Hash, Order, Display
	extern def hash end
	extern def equal(other) end
	extern def order(other) end
	extern def display end
	extern def inspect end
end

extern type Str impl Hash, Order, Display
	extern def hash end
	extern def equal(other) end
	extern def size end
	extern def chars end
	extern def order(other) end
	extern def display end
	extern def inspect end
	extern def self.empty end

	def empty?
		size == 0
	end
end

extern type List impl Access, Append
	extern def equal(other) end
	extern def size end
	extern def inspect end
	extern def append(item) end
	extern def access(key) end
	extern def store(key, item) end

	def first
		self[0]
	end
end

extern type Dict impl Access
	extern def equal(other) end
	extern def size end
	extern def inspect end
	extern def access(key) end
	extern def store(key, item) end
end

extern type Proc impl Hash
	extern def hash end
	extern def equal(other) end
	extern def inspect end
end

extern type Type impl Hash
	extern def hash end
	extern def equal(other) end
	extern def inspect end
end

extern type Proto impl Hash
	extern def hash end
	extern def equal(other) end
	extern def inspect end
end

extern type Module impl Hash
	extern def hash end
	extern def equal(other) end
	extern def inspect end
end

extern type Digest impl Hash
	extern def hash end
	extern def equal(other) end
	extern def inspect end
end
