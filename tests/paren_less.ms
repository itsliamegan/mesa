#> 5
#> 5
#> 9
#> 3
#> 9
#> 9

def five()
	5
end

$print(five)
$print(five())

type Box(n)
	def get()
		n
	end

	def show()
		get
	end
end

$print(Box(9).show())

def six
	3
end

$print(six)

b := Box(9)
$print(b.get)
$print(b.get())
