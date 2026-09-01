module Core.Prelude

import Core.Protos.Order

extern type Nil
end

extern type Num impl Order
	extern def order(other) end
end

extern type Bool
end

extern type Char impl Order
	extern def order(other) end
end

extern type Str impl Order
	extern def size end
	extern def chars end
	extern def order(other) end
	extern def self.empty end

	def empty?
		size == 0
	end
end

extern type List
	extern def size end

	def first
		self[0]
	end
end

extern type Dict
	extern def size end
end

extern type Proc
end

extern type Type
end

extern type Proto
end

extern type Module
end
