#> (Num)	5
#> (Num)	43.2
#> (Num)	123

num := 5
$dbg(num)

type Distance(amount)
end

dist := Distance(10.1)
dist.amount := 43.2
$dbg(dist.amount)

type Account
	def entries()
		[{"id": 123}]
	end
end

acct := Account()
amnt := acct.entries()[0]["id"]
$dbg(amnt)
