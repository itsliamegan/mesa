#> 1
#> 1

import Test.Order.Order

type Task(priority)
	impl Order

	def compare(other)
		return self.priority - other.priority
	end
end

$print(Task(1).min(Task(5)).priority)
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

#> 3
#> 9

import Test.Utils

$print(Utils.count)
$print(Utils.helper())

#:utils.ms
module Test.Utils

count := 3
def helper()
	return 9
end

#---

#> proto Order

import Test.Protos.Order

$print(Order)

#:protos.ms

module Test.Protos

proto Order
	def compare(other) end
end

#---

#> 5

import Test.Codec

$print(Codec.Json.decode(4))

#:codec.ms

module Test.Codec

#:codec/json.ms

module Test.Codec.Json

def decode(x)
	return x + 1
end

#---

#> 1

import Test.Counter

Counter.bump()
$print(Counter.n)

#:counter.ms

module Test.Counter

n := 0

def bump()
	n := n + 1
end

#---

#> module Test.Codec

import Test.Codec

$print(Codec)

#:codec.ms
module Test.Codec

#---

#> D
#> B
#> C
#> A

#:a.ms
module Test.A

import Test.B
import Test.C

$print("A")

#:b.ms
module Test.B

import Test.D

$print("B")

#:c.ms
module Test.C

import Test.D

$print("C")

#:d.ms
module Test.D

$print("D")

#---

#> 1

#:codec.ms
module Test.Codec

def helper()
	return 1
end

def main()
	return helper()
end

$print(main())

#---

#! member 'limit' on module Test.Codec is read-only

import Test.Codec

Codec.limit := 5

#:codec.ms
module Test.Codec

limit := 8

#---

#! module Test.Codec has no such member 'missing'

import Test.Codec

$print(Codec.missing)

#:codec.ms
module Test.Codec

#---

#! module Test.Codec has no such member 'Utils'

import Test.Codec
$print(Codec.Utils)

#:codec.ms
module Test.Codec

import Test.Utils

#:utils.ms
module Test.Utils

#---

#! src/codec.ms:3,1: syntax error: unexpected token EOF
#! src/package.ms:3,1: syntax error: unexpected token EOF

$print(

#:codec.ms

module Test.Codec
def f(

#---

#! src/bare.ms: semantic error: file declares no module
#! src/late.ms:4,1: semantic error: 'module' must be the first item in a file
#! src/twice.ms:2,1: semantic error: file declares more than one module

#:bare.ms
def f()
end

#:late.ms
def f()
end

module Test.Late

#:twice.ms
module Test.Twice
module Test.Again

#---

#! module 'Test.Json' must be declared under 'Test.Codec'

#:codec.ms
module Test.Codec

#:codec/json.ms
module Test.Json

#---

#! semantic error: duplicate member 'Codec'

type Codec
end

#:codec.ms
module Test.Codec

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

def one()
end

#:b.ms
module Test.X

def two()
end

#---

#! semantic error: unknown import 'Test.Missing'
#! semantic error: unknown import 'Test.Protos.absent'

#:protos.ms
module Test.Protos

def helper()
end

#:ring.ms
module Test.Ring

import Test.Missing
import Test.Protos.absent

#---

#> root
#> child

$print("root")

#:a.ms
module Test.A

import Test

$print("child")

#---

#! runtime error: name 'helper' is not defined

import Test.Protos.Order

helper()

#:protos.ms
module Test.Protos

proto Order
	def compare(other) end
end

def helper()
end

#---

#! semantic error: unknown protocol 'Order'

#:protos.ms
module Test.Protos

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

#! semantic error: 'Protos' is not a protocol

#:protos.ms
module Test.Protos

proto Order
	def compare(other) end
end

#:task.ms
module Test.Task

import Test.Protos

type Task
	impl Protos
	def compare(other)
		return 0
	end
end

#---

#! semantic error: 'helper' is not a protocol

#:protos.ms
module Test.Protos

proto Order
	def compare(other) end
end

def helper()
end

#:task.ms
module Test.Task

import Test.Protos.helper

type Task
	impl helper
	def compare(other)
		return 0
	end
end

#---

#! runtime error: name 'Codec' is not defined

#:json.ms
module Test.Json

$print(Codec)

#:json/codec.ms
module Test.Json.Codec

#---

#> type Frame()

import Test.Codec

$print(Codec.Frame)

#:codec.ms
module Test.Codec

type Frame
end


#---

import Test.Protos

type Point
	impl Protos.Order
end

#:protos.ms
module Test.Protos

proto Order
end

#---

#> 5
#> 9

count := 5
def helper()
	return 9
end

#:child.ms
module Test.Child

import Test

$print(Test.count)
$print(Test.helper())

#---

#> 5

#:child.ms
module Test.Child

import Test

$print(Test.Codec.Json.decode(4))

#:codec.ms
module Test.Codec

#:codec/json.ms
module Test.Codec.Json

def decode(x)
	return x + 1
end

#---

#> 5

#:child.ms
module Test.Child

import Test

Test := 5
$print(Test)
