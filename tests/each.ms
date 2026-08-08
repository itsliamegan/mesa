#> 1
#> 2
#> 3
#> 6

nums := [1, 2, 3]
each num in nums do
	$print(num)
end

total := 0
each num in nums do
	total := total + num
end
$print(total)
