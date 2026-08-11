#> Amount(500)
#> Amount(0)
#> 0

type Amount(cents)
	def self.of_dollars(n)
		Amount(n * 100)
	end

	def self.zero()
		self(0)
	end

	def display()
		self.cents
	end
end

$print(Amount.of_dollars(5))
$print(Amount.zero())
$print(Amount.zero().display())
