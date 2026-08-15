#> x
#> 1
#> x

type Expr
	case Ident(name)
	end
	case Lit(value)
	end
end

e := Expr.Ident("x")
r := when e
	case Expr.Ident then e.name
	case Expr.Lit then e.value
	else nil
end
$print(r)

l := Expr.Lit(1)
$print(when l case Expr.Ident then e.name case Expr.Lit then l.value end)

$print(when e case Expr.Lit then e.value case Expr.Ident then e.name end)

#---

#> num
#> str
#> other

$print(when 5 case Num then "num" case Str then "str" else "other" end)
$print(when "hi" case Num then "num" case Str then "str" else "other" end)
$print(when true case Num then "num" case Str then "str" else "other" end)

#---

#> nil
#> fallback

type Expr
	case Ident(name)
	end
	case Lit(value)
	end
end

e := Expr.Ident("x")
$print(when e case Expr.Lit then e.value end)
$print(when e case Expr.Lit then e.value else "fallback" end)

#---

#> tapped
#> hit

def scrutinee()
	$print("tapped")
	5
end

$print(when scrutinee() case Str then "str" case Num then "hit" end)

#---

#> fallback

type Expr
	case Ident(name)
	end
end

e := Expr.Ident("x")
$print(when e case Expr then "parent" else "fallback" end)

#---

#> 3

x := when 3 case Num then 3 else 0 end
$print(x)
