module Core.Prelude

import Core.Order

extern type Nil
end

extern type Num impl Order
end

extern type Bool
end

extern type Char impl Order
end

extern type Str impl Order
	def empty?
		size == 0
	end
end

extern type List
	def first
		self[0]
	end
end

extern type Dict
end

extern type Proc
end

extern type Type
end

extern type Proto
end

extern type Module
end
