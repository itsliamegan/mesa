import Test.Order.Order

type Task(priority)
	impl Order

	def compare(other)
		return self.priority - other.priority
	end
end

#> 1
$print(Task(1).min(Task(5)).priority)
#> 1
$print(Task(5).min(Task(1)).priority)

#:order.ms
module Test.Order

proto Order
	def compare(other) end
	def min(other)
		when self.compare(other) <= 0 then
			self
		else
			other
		end
	end
end

#---

import Test.Utils

#> 3
$print(Utils.count)
#> 9
$print(Utils.helper)

#:utils.ms
module Test.Utils

count := 3
def helper
	return 9
end

#---

import Test.Tracker.Order

#> proto Order
$print(Order)

#:tracker.ms

module Test.Tracker

proto Order
	def compare(other) end
end

#---

import Test.Ledger

#> 5
$print(Ledger.Transaction.decode(4))

#:ledger.ms

module Test.Ledger

#:ledger/transaction.ms

module Test.Ledger.Transaction

def decode(x)
	return x + 1
end

#---

import Test.Counter

Counter.bump
#> 1
$print(Counter.n)
#> def bump
$print(&Counter.bump)
#> def bump
$print(Counter.saved)

#:counter.ms

module Test.Counter

n := 0

def bump
	n := n + 1
end

saved := &bump

#---

import Test.Ledger

#> module Test.Ledger
$print(Ledger)

#:ledger.ms
module Test.Ledger

#---

#:d.ms
module Test.D

#> D
$print("D")

#:b.ms
module Test.B

import Test.D

#> B
$print("B")

#:c.ms
module Test.C

import Test.D

#> C
$print("C")

#:a.ms
module Test.A

import Test.B
import Test.C

#> A
$print("A")

#---

#:ledger.ms
module Test.Ledger

def helper
	return 1
end

def main
	return helper
end

#> 1
$print(main)

#---

#! member 'limit' on module Test.Ledger is read-only

import Test.Ledger

Ledger.limit := 5

#:ledger.ms
module Test.Ledger

limit := 8

#---

#! module Test.Ledger has no such member 'missing'

import Test.Ledger

$print(Ledger.missing)

#:ledger.ms
module Test.Ledger

#---

#! module Test.Ledger has no such member 'Utils'

import Test.Ledger
$print(Ledger.Utils)

#:ledger.ms
module Test.Ledger

import Test.Utils

#:utils.ms
module Test.Utils

#---

#! src/ledger.ms:3,1: syntax error: unexpected token EOF
#! src/package.ms:3,1: syntax error: unexpected token EOF

$print(

#:ledger.ms

module Test.Ledger
def f(

#---

#! src/bare.ms: semantic error: file declares no module
#! src/late.ms:4,1: semantic error: 'module' must be the first item in a file
#! src/twice.ms:2,1: semantic error: file declares more than one module

#:bare.ms
def f
end

#:late.ms
def f
end

module Test.Late

#:twice.ms
module Test.Twice
module Test.Again

#---

#! module 'Test.Receipt' must be declared under 'Test.Ledger'

#:ledger.ms
module Test.Ledger

#:ledger/receipt.ms
module Test.Receipt

#---

#! semantic error: duplicate member 'Ledger'

type Ledger
end

#:ledger.ms
module Test.Ledger

#---

n := 1
n := 2
#> 2
$print(n)

#---

#! semantic error: duplicate member 'total'
#! semantic error: duplicate member 'Ledger'

def total
	1
end

total := 2

Ledger := 3

type Ledger
end

#---

#! import cycle: Test.A

#:a.ms
module Test.A

import Test.B

#:b.ms
module Test.B

import Test.A

#---

#! semantic error: duplicate module 'Test.X'

#:a.ms
module Test.X

def one
end

#:b.ms
module Test.X

def two
end

#---

#! semantic error: unknown import 'Test.Missing'
#! semantic error: unknown import 'Test.Tracker.absent'

#:tracker.ms
module Test.Tracker

def helper
end

#:ring.ms
module Test.Ring

import Test.Missing
import Test.Tracker.absent

#---

#> root
$print("root")

#:a.ms
module Test.A

import Test

#> child
$print("child")

#---

#! runtime error: name 'helper' is not defined

import Test.Tracker.Order

helper

#:tracker.ms
module Test.Tracker

proto Order
	def compare(other) end
end

def helper
end

#---

#! semantic error: unknown protocol 'Order'

#:tracker.ms
module Test.Tracker

proto Order
	def compare(other) end
end

#:task.ms
module Test.Task

type Task
	impl Order
	def compare(other)
		return 0
	end
end

#---

#! semantic error: 'Tracker' is not a protocol

#:tracker.ms
module Test.Tracker

proto Order
	def compare(other) end
end

#:task.ms
module Test.Task

import Test.Tracker

type Task
	impl Tracker
	def compare(other)
		return 0
	end
end

#---

#! semantic error: 'helper' is not a protocol

#:tracker.ms
module Test.Tracker

proto Order
	def compare(other) end
end

def helper
end

#:task.ms
module Test.Task

import Test.Tracker.helper

type Task
	impl helper
	def compare(other)
		return 0
	end
end

#---

#! runtime error: name 'Ledger' is not defined

#:receipt.ms
module Test.Receipt

$print(Ledger)

#:receipt/ledger.ms
module Test.Receipt.Ledger

#---

import Test.Ledger

#> type Group
$print(Ledger.Group)

#:ledger.ms
module Test.Ledger

type Group
end

#---

import Test.Tracker

type Task(priority)
	impl Tracker.Order
end

#:tracker.ms
module Test.Tracker

proto Order
end

#---

count := 5
def helper
	return 9
end

#:child.ms
module Test.Child

import Test

#> 5
$print(Test.count)
#> 9
$print(Test.helper)

#---

#:child.ms
module Test.Child

import Test

#> 5
$print(Test.Ledger.Transaction.decode(4))

#:ledger.ms
module Test.Ledger

#:ledger/transaction.ms
module Test.Ledger.Transaction

def decode(x)
	return x + 1
end

#---

#:child.ms
module Test.Child

import Test

Test := 5
#> 5
$print(Test)

#---

import Core.IO

#> module Core.IO
$print(IO)

#---

import Core.IO

#> hello
IO.print("hello")

#---

import Core.Behaviors.Order

type Task(priority)
	impl Order

	def order(other)
		return self.priority - other.priority
	end
end

#> proto Order
$print(Order)
#> 1
$print(Task(1).min(Task(5)).priority)
#> 5
$print(Task(1).max(Task(5)).priority)
#> Task(priority: 1)
$print(Task(5).min(Task(1)))

#---

#! semantic error: required param 'b' follows defaulted param 'a'
#! semantic error: case type 'Transaction' cannot declare params

def add(a: 0, b)
	a + b
end

type Transaction(name)
	 case Credit(amount) end
end

#---

#! semantic error: duplicate member 'encode'

def encode
end
def encode
end

#---

#! semantic error: duplicate member 'Task'

type Task
end
def Task
end

#---

#! semantic error: duplicate member 'Order'

proto Order
	def before(other) end
end
type Order
end

#---

x := 1
x := 2
def f
end

#---

#! semantic error: duplicate member 'count'

count := 1
def count
end

#---

#! src/ledger/transaction: semantic error: directory has no sibling module file
#! src/package: semantic error: 'src/package/' is reserved; the root module's children live in src/

#:ledger.ms
module Test.Ledger

#:package/ext.ms
module Test.Package.Ext

#:ledger/transaction/encode.ms
module Test.Ledger.Transaction.Encode
