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

#---

#> 5

$print(each n in [1, 2, 3, 4, 5] do
	when n == 5 then break n end
end)

#---

#> nil

$print(each n in [1, 2, 3] do
	when n == 2 then break end
end)

#---

#> 1

$print(each k in {1: "a", 2: "b"} do
	break k
end)
