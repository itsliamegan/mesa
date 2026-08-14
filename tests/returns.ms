#> nil

def exit_if_one(num)
	when num == 1 then
		return
	else
		num
	end
end

$print(exit_if_one(1))

#---

#> nil

def exit_if_one(num)
	when num == 1 then return end
	num
end

$print(exit_if_one(1))
