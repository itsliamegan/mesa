#> 6
#> 11
#> 21
#> 100

type Box(x)
	def bump()
		x := x + 1
	end

	def shadow()
		each x in [10, 20] do
			x := x + 1
			$print(x)
		end
	end
end

b := Box(5)
b.bump()
$print(b.x)

c := Box(100)
c.shadow()
$print(c.x)
