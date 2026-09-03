module Core

import Core.Behaviors.Order

extern type Counter impl Order
	extern def count end
	extern def bump end
	extern def order(other) end
	extern def self.zero end

	def bump_by(n)
		i := 0
		loop do
			when i >= n then break end
			self.bump
			i := i + 1
		end
		self.count
	end

	def self.at(n)
		c := Counter.zero
		c.bump_by(n)
		c
	end
end
