#> module Core.IO

import Core.IO

$print(IO)

#---

#> hello

import Core.IO

IO.print("hello")

#---

#> proto Order
#> 1
#> 5
#> Task(1)

import Core.Order

type Task(priority)
	impl Order

	def order(other)
		return self.priority - other.priority
	end
end

$print(Order)
$print(Task(1).min(Task(5)).priority)
$print(Task(1).max(Task(5)).priority)
$print(Task(5).min(Task(1)))
