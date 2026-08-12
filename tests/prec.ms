#> 5
#> 43.2
#> 123
#> 9
#> 7

num := 5
$print(num)

type Distance(amount)
end

dist := Distance(10.1)
dist.amount := 43.2
$print(dist.amount)

type Account
	def entries()
		[{"id": 123}]
	end
end

acct := Account()
amnt := acct.entries()[0]["id"]
$print(amnt)

$print((1 + 2) * 3)
$print(1 + 2 * 3)
