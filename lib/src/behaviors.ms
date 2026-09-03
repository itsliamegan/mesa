module Core.Behaviors

proto Equal
	extern def equal(other) end
end

proto Hash
	impl Equal

	extern def hash end
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

proto Access
	def access(key) end
	extern def store(key, item) end
end

proto Append
	def append(item) end
end

proto Display
	impl Inspect

	def display end
end

proto Inspect
	extern def inspect end
end
