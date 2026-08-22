#! runtime error: type Widget is not orderable

# A locally-declared protocol, same package as 'Widget', whose required verb
# happens to also be spelled 'order'. Conformance is identity-based, not
# name-based: implementing this 'Order' is not implementing 'Core.Order', so
# the '<' operator — which only ever dispatches through 'Core.Order' — must
# still reject the comparison.
proto Order
	def order(other) end
end

type Widget(n)
	impl Order

	def order(other)
		self.n - other.n
	end
end

$print(Widget(1) < Widget(2))
