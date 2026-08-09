#> 1
#> 2
#> 3
#> 6
#> 1

nums := [1, 2, 3]
each num in nums do
	$print(num)
end

total := 0
each num in nums do
	total := total + num
end
$print(total)

total := 0
def count()
	each num in [1, 2, 3] do
		 total := total + num
		 return nil
	end
end
count()
$print(total)
