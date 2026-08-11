#> true
#> false
#> 5
#> 0
#> nil
#> Str
#> two
#> other

when true then
	$print(true)
end

when false then
else
	$print(false)
end

when 5 then
	$print(5)
end

when 0 then
	$print(0)
end

when nil then
else
	$print(nil)
end

when "Str" then
	$print("Str")
end

when false then
	$print("one")
else when true then
	$print("two")
else
	$print("other")
end

when false then
	$print("one")
else when false then
	$print("two")
else
	$print("other")
end
