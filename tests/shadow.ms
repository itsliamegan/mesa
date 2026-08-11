#> 6
#> 11
#> 21
#> 100
#> 7
#> field w
#> 100
#> 1
#> 5

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

v := 1
outside := 5

def w()
	"module proc"
end

type Cell(v, w)
	def read()
		v
	end

	def write()
		v := 100
		w
	end

	def peek()
		outside
	end
end

cell := Cell(7, "field w")
$print(cell.read())
$print(cell.write())
$print(cell.v)
$print(v)
$print(cell.peek())
