#> 1

def first()
	loop do
		return 1
	end
end
$print(first())

#---

#> nil

$print(loop do break end)

#---

#> 5

$print(loop do break 5 end)

#---

#> nil

x := loop do
	break
	100
end
$print(x)

#---

#> 3

i := 0
loop do
	done := i == 3
	when done then break end
	i := i + 1
end
$print(i)

#---

#> 0
#> 1
#> 2

i := 0
loop do
	when i > 2 then
		break
	end
	$print(i)
	i := i + 1
end
