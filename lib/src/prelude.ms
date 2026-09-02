module Core.Prelude

import Core.Protos.Display
import Core.Protos.Order

extern type Nil
	extern def inspect end
end

extern type Num impl Order
	extern def order(other) end
	extern def inspect end
end

extern type Bool
	extern def inspect end
end

extern type Char impl Order, Display
	extern def order(other) end
	extern def display end
	extern def inspect end
end

extern type Str impl Order, Display
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

extern type List
	extern def size end
	extern def inspect end

	def first
		self[0]
	end
end

extern type Dict
	extern def size end
	extern def inspect end
end

extern type Proc
	extern def inspect end
end

extern type Type
	extern def inspect end
end

extern type Proto
	extern def inspect end
end

extern type Module
	extern def inspect end
end
