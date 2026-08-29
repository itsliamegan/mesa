import Core.Errors.Error

def unused
	raise Error.KeyError("boom")
end

def unused_rescue
	do
		1
	rescue e case Error.KeyError then
		$print(e.message)
	else
		$print("other")
	end
end

#> parsed
$print("parsed")

x := do
	1 + 2
end
#> 3
$print(x)

do
	raise Error.KeyError("boom")
rescue e case Error.KeyError then
	#> caught: boom
	$print("caught: " + e.message)
end

do
	unknown()
rescue e case Error.NameError then
	#> name 'unknown' is not defined
	$print(e.message)
end

do
	raise Error.KeyError("boom")
rescue e case Error.IndexError then
	$print("wrong arm")
else
	#> other
	$print("other")
end

do
	raise 5
rescue e case Num then
	#> 5
	$print(e)
end

do
	raise "boom"
rescue e case Str then
	#> boom
	$print(e)
end

do
	raise [1, 2]
rescue e case List then
	#> [1, 2]
	$print(e)
end

type Task(priority)
	def status
		"queued"
	end
end

do
	raise Task(1)
rescue e case Task then
	#> 1
	$print(e.priority)
end

do
	each x in 1 do
	end
rescue e case Error.ProtocolError then
	#> Error.ProtocolError(ProtocolError.NotIterable("type Num is not iterable"))
	$print(e)
end

do
	5[0]
rescue e case Error.ProtocolError then
	#> Error.ProtocolError(ProtocolError.NotAccessible("type Num is not accessible"))
	$print(e)
end

do
	{} << "x"
rescue e case Error.ProtocolError then
	#> Error.ProtocolError(ProtocolError.NotAppendable("type Dict is not appendable"))
	$print(e)
end

do
	Task(1) < Task(2)
rescue e case Error.ProtocolError then
	#> Error.ProtocolError(ProtocolError.NotOrderable("type Task is not orderable"))
	$print(e)
end

do
	n := 1
	n()
rescue e case Error.TypeError then
	#> Error.TypeError(TypeError.NotCallable("type Num is not callable"))
	$print(e)
end

do
	Char()
rescue e case Error.TypeError then
	#> Error.TypeError(TypeError.NotConstructible("type Char cannot be constructed"))
	$print(e)
end

do
	&1
rescue e case Error.TypeError then
	#> Error.TypeError(TypeError.NotInvokable("type Num is not invokable"))
	$print(e)
end

do
	n := 5
	m := 7
	when n case m then "seven" end
rescue e case Error.TypeError then
	#> Error.TypeError(TypeError.CaseNonType("type Num cannot be matched against"))
	$print(e)
end

do
	$print([1, 2, 3]["a"])
rescue e case Error.TypeError then
	#> Error.TypeError(TypeError.IndexNonNum("index "a" is not a number"))
	$print(e)
end

do
	$print(5.1 + "str")
rescue e case Error.TypeError then
	#> Error.TypeError(TypeError.ArithNonNum("type Str cannot be used in arithmetic"))
	$print(e)
end

do
	$print("str" + 5.1)
rescue e case Error.TypeError then
	#> Error.TypeError(TypeError.ConcatNonStr("type Num cannot be concatenated"))
	$print(e)
end

def need(a)
	a
end

def pair(a, b)
	b
end

def named(a, b: 1)
	a
end

def twice(a, b: 1)
	a
end

do
	need()
rescue e case Error.ArgumentError then
	#> Error.ArgumentError(ArgumentError.Missing("missing arg 'a'"))
	$print(e)
end

do
	pair(1, 2, 3)
rescue e case Error.ArgumentError then
	#> Error.ArgumentError(ArgumentError.TooMany("too many args; have 3, want at most 2"))
	$print(e)
end

do
	named(1, x: 2)
rescue e case Error.ArgumentError then
	#> Error.ArgumentError(ArgumentError.Unknown("no param named 'x'"))
	$print(e)
end

do
	twice(1, a: 2)
rescue e case Error.ArgumentError then
	#> Error.ArgumentError(ArgumentError.Duplicate("arg 'a' given twice"))
	$print(e)
end

do
	$print([1, 2, 3][3])
rescue e case Error.IndexError then
	#> Error.IndexError(IndexError.OutOfRange("index 3 is out of bounds"))
	$print(e)
end

do
	$print([1, 2, 3][1.7])
rescue e case Error.IndexError then
	#> Error.IndexError(IndexError.NonIntegral("index 1.7 is not a whole number"))
	$print(e)
end

do
	Task(1).unknown
rescue e case Error.MemberError then
	#> Error.MemberError(MemberError.Missing("type Task has no such member 'unknown'"))
	$print(e)
end

do
	Task(1).status := "done"
rescue e case Error.MemberError then
	#> Error.MemberError(MemberError.ReadOnly("member 'status' on type Task is read-only"))
	$print(e)
end

do
	$print(nope)
rescue e case Error.NameError then
	#> Error.NameError("name 'nope' is not defined")
	$print(e)
end

do
	$print({"a": 1}["b"])
rescue e case Error.KeyError then
	#> Error.KeyError("key "b" not found")
	$print(e)
end

#---

#! runtime error: Error.KeyError("boom")

import Core.Errors.Error

raise Error.KeyError("boom")

#---

#! runtime error: Error.KeyError("boom")

import Core.Errors.Error

do
	raise Error.KeyError("boom")
rescue e case Error.IndexError then
	$print("wrong arm")
end

#---

#! runtime error: 5

raise 5

#---

#! runtime error: "boom"

raise "boom"

#---

#! runtime error: type Error()

import Core.Errors.Error

raise Error

#---

#! runtime error: type Error.KeyError(message)

import Core.Errors.Error

raise Error.KeyError

#---

#! runtime error: Point(1, 2)

type Point(x, y) end

raise Point(1, 2)
