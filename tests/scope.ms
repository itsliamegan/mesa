#> 6

total := 0

def count()
	nums := [1, 2, 3]
	each num in nums do
		total := total + num
	end
end

count()

$print(total)
