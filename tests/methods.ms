#> Unit()
#> Unit()

type Unit
	def copy()
		self
	end
end

$print(Unit().copy())

copy := &Unit().copy
$print(copy())
