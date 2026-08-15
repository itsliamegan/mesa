#! semantic error: required param 'b' follows defaulted param 'a'

def f(a: 1, b)
end

#---

#! semantic error: required param 'unit' follows defaulted param 'amount'

type Distance(amount: 0, unit)
end

#---

#! semantic error: required param 'to' follows defaulted param 'from'

type Trip(n)
	def leg(from: 0, to)
	end
end

#---

#! semantic error: required param 'y' follows defaulted param 'x'

type Point(n)
	def self.at(x: 0, y)
	end
end
