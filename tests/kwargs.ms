#> 3
#> 3
#> 3
#> 30

def add(a, b)
	a + b
end
$print(add(1, 2))
$print(add(1, b: 2))
$print(add(a: 1, b: 2))
$print(add(b: 20, a: 10))

#---

#> 101
#> 201
#> 101
#> 201

def offset(n, by: 100)
	n + by
end
$print(offset(1))
$print(offset(1, by: 200))
$print(offset(n: 1))
$print(offset(by: 200, n: 1))

#---

#> 7
#> 31
#> 103

def chain(a, b: a + 1, c: b * 2)
	a + b + c
end
$print(chain(1))
$print(chain(1, 10))
$print(chain(1, c: 100))

#---

#> 1
#> 2

def mark(n)
	$print(n)
	n
end
def pick(a: mark(1), b: mark(2))
	b
end
$print(pick(b: 2))

#---

#> 9
#> 12
#> 12
#> 25
#> Rect(3, 3)

type Rect(w, h: w)
	area := w * h
end
$print(Rect(3).area)
$print(Rect(3, 4).area)
$print(Rect(h: 4, w: 3).area)
$print(Rect(w: 5).area)
$print(Rect(3))

#---

#> 0
#> 50
#> 7

type Acct(owner, balance: 0)
	def deposit(amount: 5)
		balance + amount
	end

	def self.opened(owner: 7)
		Acct(owner)
	end
end
$print(Acct(1).balance)
$print(Acct(1, balance: 50).balance)
$print(Acct.opened().owner)

#---

#> 15
#> 105

type Acct(owner, balance: 0)
	def deposit(amount: 5)
		balance + amount
	end
end
$print(Acct(1, balance: 10).deposit())
$print(Acct(1, balance: 5).deposit(amount: 100))
