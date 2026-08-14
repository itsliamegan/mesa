#> def id(val)

def id(val)
	&val
end

$print(id(&id))

#---

#> def unit

def unit
end

$print(&unit)
