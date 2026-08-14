#> 3
#> 3
#> 0
#> 9

type LinkedList
	type Iter(pos)
		def display()
			self.pos
		end
	end

	type Str
	end

	def self.new_iter()
		LinkedList.Iter(0)
	end
end

it := LinkedList.Iter(3)
$print(it.pos)
$print(it.display())
$print(LinkedList.new_iter().pos)

type A
	type B
		type C(val)
		end
	end
end

c := A.B.C(9)
$print(c.val)

# A nested type names itself qualified, since the bare name cannot be typed.
#> LinkedList.Iter(0)
#> A.B.C(9)

$print(LinkedList.Iter(0))
$print(c)
