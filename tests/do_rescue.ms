#> parsed

import Core.Errors.Error

def unused()
	raise Error.KeyError("boom")
end

def unused_rescue()
	do
		1
	rescue e case Error.KeyError then
		$print(e.message)
	else
		$print("other")
	end
end

$print("parsed")

#---

#! runtime error: Error.KeyError("boom")

import Core.Errors.Error

raise Error.KeyError("boom")

#---

#> 3

x := do
	1 + 2
end
$print(x)

#---

#> caught: boom

import Core.Errors.Error

do
	raise Error.KeyError("boom")
rescue e case Error.KeyError then
	$print("caught: " + e.message)
end

#---

#> name 'unknown' is not defined

import Core.Errors.Error

do
	unknown()
rescue e case Error.NameError then
	$print(e.message)
end

#---

#> other

import Core.Errors.Error

do
	raise Error.KeyError("boom")
rescue e case Error.IndexError then
	$print("wrong arm")
else
	$print("other")
end

#---

#! runtime error: Error.KeyError("boom")

import Core.Errors.Error

do
	raise Error.KeyError("boom")
rescue e case Error.IndexError then
	$print("wrong arm")
end

#---

#! runtime error: type Num cannot be raised

raise 5

#---

#! runtime error: type Type cannot be raised

import Core.Errors.Error

raise Error

#---

#! runtime error: type Type cannot be raised

import Core.Errors.Error

raise Error.KeyError

#---

#! runtime error: type Point cannot be raised

type Point(x, y) end

raise Point(1, 2)
