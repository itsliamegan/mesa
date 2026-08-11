#> true
#> true
#> true
#> true
#> true
#> true

type Box(val)
	def empty?()
		self.val == nil
	end
end

done! := true
$print(done!)

b := Box(nil)
$print(b.empty?())

a := 4
$print(a != 2)
$print(a!=2)

c! := 4
$print(c! == 4)
$print(c!==4)
