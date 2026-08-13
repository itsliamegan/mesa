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

#---

#> 7
#> 7
#> 20
#> 12
#> 12

def defaulted(a: 7)
	a
end

$print(defaulted)
$print(defaulted())
$print(defaulted(20))

type Tally(n: 12)
	def show(by: 0)
		n + by
	end
end

$print(Tally().show)
$print(Tally().n)

#---

#> 2
#> 2
#> 3
#> 3
#> 1
#> 1

$print("ab".size)
$print("ab".size())
$print([1, 2, 3].size)
$print([1, 2, 3].size())
$print({"a": 1}.size)
$print({"a": 1}.size())
