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

#! each.ms:38,10: runtime error: name 'leak' is not defined
#! 	at <main> (each.ms:38,10)

each n in [1, 2] do
	when n == 1 then
		leak := true
	end
	when n == 2 then
		$print(leak)
	end
end

#---

#! each.ms:52,10: runtime error: name 'leak' is not defined
#! 	at <main> (each.ms:52,10)

each k in {1: "a", 2: "b"} do
	when k == 1 then
		leak := true
	end
	when k == 2 then
		$print(leak)
	end
end

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
