#> proto Order
#> low
#> high
#> low
#> high

proto Order
	def compare(other) end

	def min(other)
		when self.compare(other) <= 0 then return self else return other end
	end

	def max(other)
		when self.compare(other) <= 0 then return other else return self end
	end
end

type Task(priority, name)
	impl Order

	def compare(other)
		return self.priority - other.priority
	end
end

$print(Order)

low := Task(1, "low")
high := Task(5, "high")
$print(low.min(high).name)
$print(low.max(high).name)
$print(high.min(low).name)
$print(high.max(low).name)

#---

#> true
#> false
#> false
#> true
#> always full

proto Sized
	def size() end

	def empty()
		return self.size() == 0
	end

	def full()
		return not self.empty()
	end
end

type Bag(n)
	impl Sized

	def size()
		return self.n
	end
end

type Crate(n)
	impl Sized

	def size()
		return self.n
	end

	def full()
		return "always full"
	end
end

$print(Bag(0).empty())
$print(Bag(3).empty())
$print(Bag(0).full())
$print(Bag(3).full())
$print(Crate(0).full())

#---

#> 32
#> 1

type Two(base)
	impl Power

	def of()
		return self.base
	end
end

proto Power
	def of() end

	def pow(n)
		when n == 0 then return 1 end
		return self.of() * self.pow(n - 1)
	end
end

$print(Two(2).pow(5))
$print(Two(7).pow(0))

#---

#> 2
#> 9
#> 4

proto Order
	def compare(other) end

	def min(other)
		when self.compare(other) <= 0 then return self else return other end
	end

	def max(other)
		when self.compare(other) <= 0 then return other else return self end
	end
end

type Shape
	impl Order

	case Circle(r)
	end

	case Square(s)
		impl Order

		def compare(other)
			return self.s - other.s
		end
	end

	def compare(other)
		return 0
	end
end

$print(Shape.Circle(2).min(Shape.Circle(9)).r)
$print(Shape.Square(9).max(Shape.Square(4)).s)
$print(Shape.Square(9).min(Shape.Square(4)).s)
