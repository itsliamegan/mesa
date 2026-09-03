module Core.Prelude

import Core.Behaviors.Access
import Core.Behaviors.Append
import Core.Behaviors.Display
import Core.Behaviors.Equal
import Core.Behaviors.Hash
import Core.Behaviors.Inspect
import Core.Behaviors.Order

extern type Nil
	impl Equal, Hash, Inspect

	extern def equal(other) end
	extern def hash end
	extern def inspect end
end

extern type Num
	impl Equal, Hash, Order, Inspect

	extern def equal(other) end
	extern def hash end
	extern def order(other) end
	extern def inspect end
end

extern type Bool
	impl Equal, Hash, Inspect

	extern def equal(other) end
	extern def hash end
	extern def inspect end
end

extern type Char
	impl Equal, Hash, Order, Display, Inspect

	extern def equal(other) end
	extern def hash end
	extern def order(other) end
	extern def display end
	extern def inspect end
end

extern type Str
	impl Equal, Hash, Order, Display, Inspect

	extern def self.empty end

	extern def chars end
	extern def size end

	def empty?
		size == 0
	end

	extern def equal(other) end
	extern def hash end
	extern def order(other) end
	extern def display end
	extern def inspect end
end

extern type Digest
	impl Equal, Hash, Inspect

	extern def equal(other) end
	extern def hash end
	extern def inspect end
end

extern type List
	impl Equal, Access, Append, Inspect

	extern def size end

	def first
		self[0]
	end

	extern def equal(other) end
	extern def access(key) end
	extern def store(key, item) end
	extern def append(item) end
	extern def inspect end
end

extern type Dict
	impl Equal, Access, Inspect

	extern def size end

	extern def equal(other) end
	extern def access(key) end
	extern def store(key, item) end
	extern def inspect end
end

extern type Proc
	impl Equal, Hash, Inspect

	extern def equal(other) end
	extern def hash end
	extern def inspect end
end

extern type Type
	impl Equal, Hash, Inspect

	extern def equal(other) end
	extern def hash end
	extern def inspect end
end

extern type Proto
	impl Equal, Hash, Inspect

	extern def equal(other) end
	extern def hash end
	extern def inspect end
end

extern type Module
	impl Equal, Hash, Inspect

	extern def equal(other) end
	extern def hash end
	extern def inspect end
end
