module Core.Protos

proto Order
	def order(other) end

	def min(other)
		when order(other) <= 0 then self else other end
	end

	def max(other)
		when order(other) < 0 then other else self end
	end
end
