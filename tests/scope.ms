msg := "Hello, world!"
#> Hello, world!
$print(msg)

total := 0

def count
	nums := [1, 2, 3]
	each num in nums do
		total := total + num
	end
end

count

#> 6
$print(total)

type Task(priority)
	def bump
		priority := priority + 1
	end

	def shadow
		each priority in [10, 20] do
			priority := priority + 1
			$print(priority)
		end
	end
end

task := Task(5)
task.bump
#> 6
$print(task.priority)

other := Task(100)
#> 11
#> 21
other.shadow
#> 100
$print(other.priority)

w := 1
outside := 5

def h
	"module proc"
end

type Rect(w, h)
	def read
		w
	end

	def write
		w := 100
		h
	end

	def peek
		outside
	end
end

rect := Rect(7, "field h")
#> 7
$print(rect.read)
#> field h
$print(rect.write)
#> 100
$print(rect.w)
#> 1
$print(w)
#> 5
$print(rect.peek)

type Tag(label)
	def empty?
		self.label == nil
	end
end

done! := true
#> true
$print(done!)

tag := Tag(nil)
#> true
$print(tag.empty?)

a := 4
#> true
$print(a != 2)
#> true
$print(a!=2)

c! := 4
#> true
$print(c! == 4)
#> true
$print(c!==4)

#> type Str
$print(Str)
#> type Num
$print(Num)
#> type List
$print(List)
#> type Char
$print(Char)
#> 0
$print(Str().size)
#> []
$print(List())

Str := "shadowed"
#> shadowed
$print(Str)

#---

#! runtime error: name 'unknown' is not defined

$print(unknown)

#---

#! runtime error: name 'inner' is not defined

def outer
	inner
end
outer

#---

#! runtime error: name 'unknown' is not defined

type Ctor(a, b: unknown)
end
Ctor(1)

#---

#! runtime error: name 'leak' is not defined

each n in [1, 2] do
	when n == 1 then
		leak := true
	end
	when n == 2 then
		$print(leak)
	end
end

#---

#! runtime error: name 'leak' is not defined

each k in {1: "a", 2: "b"} do
	when k == 1 then
		leak := true
	end
	when k == 2 then
		$print(leak)
	end
end

#---

import Test.Other

Num := 5

#> 5
$print(Num)
#> type Num
$print(Other.num)

def shadows
	Str := "local"
	Str
end

#> local
$print(shadows)
#> type Str
$print(Str)

#:other.ms
module Test.Other

def num
	Num
end
