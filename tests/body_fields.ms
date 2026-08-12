#> 24
#> 12

type Rect(w, h)
	area := w * h
	half_area := area / 2
end

r := Rect(4, 6)
$print(r.area)
$print(r.half_area)

#---

#> 6

type Counter(start)
	count := start
end

c := Counter(5)
c.count := c.count + 1
$print(c.count)

#---

#! body_fields.ms:33,2: syntax error: unexpected token IDENT

type Bad
	def m()
		1
	end
	x := 1
end

#---

#! body_fields.ms:41,2: syntax error: unexpected token NUM

type T(a)
	1 + 1
end
