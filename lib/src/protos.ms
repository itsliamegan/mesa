module Core.Protos

proto Inspect
	extern def inspect end
end

proto Display
	impl Inspect

	def display end
end

proto Order
	def order(other) end

	def min(other)
		when order(other) <= 0 then self else other end
	end

	def max(other)
		when order(other) < 0 then other else self end
	end
end
