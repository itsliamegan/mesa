when true then
	#> true
	$print(true)
end

when false then
else
	#> false
	$print(false)
end

when 5 then
	#> 5
	$print(5)
end

when 0 then
	#> 0
	$print(0)
end

when nil then
else
	#> nil
	$print(nil)
end

when "Str" then
	#> Str
	$print("Str")
end

when false then
	$print("one")
else when true then
	#> two
	$print("two")
else
	$print("other")
end

when false then
	$print("one")
else when false then
	$print("two")
else
	#> other
	$print("other")
end

#---

type Transaction
	case Credit(amount)
	end
	case Debit(amount)
	end
end

c := Transaction.Credit(50)
r := when c
	case Transaction.Credit then c.amount
	case Transaction.Debit then c.amount
	else nil
end
#> 50
$print(r)

d := Transaction.Debit(1)
#> 1
$print(when d case Transaction.Credit then c.amount case Transaction.Debit then d.amount end)

#> 50
$print(when c case Transaction.Debit then c.amount case Transaction.Credit then c.amount end)

#> nil
$print(when c case Transaction.Debit then c.amount end)
#> fallback
$print(when c case Transaction.Debit then c.amount else "fallback" end)

def scrutinee
	#> tapped
	$print("tapped")
	Transaction.Credit(50)
end

#> credit
$print(when scrutinee case Transaction.Debit then "debit" case Transaction.Credit then "credit" end)

#> fallback
$print(when c case Transaction then "any" else "fallback" end)

x := when 3 case Num then 3 else 0 end
#> 3
$print(x)

#---

nums := [1, 2, 3]
#> 1
#> 2
#> 3
each num in nums do
	$print(num)
end

total := 0
each num in nums do
	total := total + num
end
#> 6
$print(total)

total := 0
def count
	each num in [1, 2, 3] do
		 total := total + num
		 return nil
	end
end
count
#> 1
$print(total)

#> 5
$print(each n in [1, 2, 3, 4, 5] do
	when n == 5 then break n end
end)

#> nil
$print(each n in [1, 2, 3] do
	when n == 2 then break end
end)

#> 1
$print(each k in {1: "a", 2: "b"} do
	break k
end)

def first
	loop do
		return 1
	end
end
#> 1
$print(first)

#> nil
$print(loop do break end)

#> 5
$print(loop do break 5 end)

y := loop do
	break
	100
end
#> nil
$print(y)

i := 0
loop do
	done := i == 3
	when done then break end
	i := i + 1
end
#> 3
$print(i)

i := 0
#> 0
#> 1
#> 2
loop do
	when i > 2 then
		break
	end
	$print(i)
	i := i + 1
end

#---

#! semantic error: 'break' outside a loop

break

#---

#! semantic error: 'break' outside a loop

type Task(priority)
	def m
		break
	end
end

#---

#! runtime error: type Num is not iterable

each item in 1 do
	$print(item)
end
