def id(val)
	&val
end

#> def id(val)
$print(id(&id))

def unit
end

#> def unit
$print(&unit)

#---

type Account(name, balance: 0)
	def deposit(amount: 5)
		balance + amount
	end

	def self.opened(name: "Ada")
		Account(name)
	end

	def entries
		[{"id": 123}]
	end
end
#> 0
$print(Account("Ada").balance)
#> 50
$print(Account("Ada", balance: 50).balance)
#> Ada
$print(Account.opened().name)
#> 15
$print(Account("Ada", balance: 10).deposit())
#> 105
$print(Account("Ada", balance: 5).deposit(amount: 100))

acct := Account("Ada")
#> 123
$print(acct.entries[0]["id"])

def add(a, b)
	a + b
end
#> 3
$print(add(1, 2))
#> 3
$print(add(1, b: 2))
#> 3
$print(add(a: 1, b: 2))
#> 30
$print(add(b: 20, a: 10))

def offset(n, by: 100)
	n + by
end
#> 101
$print(offset(1))
#> 201
$print(offset(1, by: 200))
#> 101
$print(offset(n: 1))
#> 201
$print(offset(by: 200, n: 1))

def chain(a, b: a + 1, c: b * 2)
	a + b + c
end
#> 7
$print(chain(1))
#> 31
$print(chain(1, 10))
#> 103
$print(chain(1, c: 100))

def mark(n)
	#> 1
	$print(n)
	n
end
def pick(a: mark(1), b: mark(2))
	b
end
#> 2
$print(pick(b: 2))

type Rect(w, h: w)
	area := w * h
end
#> 9
$print(Rect(3).area)
#> 12
$print(Rect(3, 4).area)
#> 12
$print(Rect(h: 4, w: 3).area)
#> 25
$print(Rect(w: 5).area)
#> Rect(w: 3, h: 3)
$print(Rect(3))

#---

def five()
	5
end

#> 5
$print(five)
#> 5
$print(five())

type Task(priority)
	def get()
		priority
	end

	def show()
		get
	end
end

#> 9
$print(Task(9).show())

def six
	3
end

#> 3
$print(six)

t := Task(9)
#> 9
$print(t.get)
#> 9
$print(t.get())

def defaulted(a: 7)
	a
end

#> 7
$print(defaulted)
#> 7
$print(defaulted())
#> 20
$print(defaulted(20))

type Circle(r: 12)
	def show(by: 0)
		r + by
	end
end

#> 12
$print(Circle().show)
#> 12
$print(Circle().r)

#> 2
$print("ab".size)
#> 2
$print("ab".size())
#> 3
$print([1, 2, 3].size)
#> 3
$print([1, 2, 3].size())
#> 1
$print({"a": 1}.size)
#> 1
$print({"a": 1}.size())

#> def five
$print(&five)

#> def get
$print(&t.get)

f := &five
#> 5
$print(f())

def id(val)
	&val
end

#> def id(val)
$print(id(&id))

#---

def exit_if_one(num)
	when num == 1 then
		return
	else
		num
	end
end

#> nil
$print(exit_if_one(1))

def exit_if_one_short(num)
	when num == 1 then return end
	num
end

#> nil
$print(exit_if_one_short(1))

#---

#! runtime error: type Num is not callable

num := 1
num()

#---

#! runtime error: missing arg 'b'

def either(a, b)
	b
end
either(2)

#---

#! runtime error: missing arg 'a'

def one(a)
	a
end
one

#---

#! runtime error: too many args; have 3, want at most 2

def pair(a, b)
	b
end
pair(1, 2, 3)

#---

#! runtime error: missing arg 'h'

type Rect(w, h)
end
Rect(3)

#---

#! runtime error: no param named 'x'

def named(a, b: 1)
	a
end
named(1, x: 2)

#---

#! runtime error: arg 'a' given twice

def twice(a, b: 1)
	a
end
twice(1, a: 2)

#---

#! runtime error: missing args 'a', 'b'

def needs(a, b, c: 1)
	a
end
needs()

#---

#! runtime error: type Num is not invocable

$print(&1)

#---

#! semantic error: duplicate param 'amount'

def total(amount, amount)
end
