#! syntax error: positional argument after keyword argument

def add(a, b)
	a + b
end
add(a: 1, 2)

#---

#! syntax error: character literal must hold exactly one character

$print('ab')

#---

#! syntax error: character literal must hold exactly one character

$print('')

#---

#! syntax error: unterminated character literal

$print('a)

#---

#! syntax error: unexpected token EQ

&x := 1

#---

#! syntax error: unexpected token CASE

type Expr
	seen := false
	case Ident(name)
	end
end

#---

#! syntax error: unexpected token CASE

type Expr
	case Ident(name)
		case Inner(x)
		end
	end
end

#---

#! syntax error: unexpected token LPAREN

type Point
	(1)
end

#---

#! syntax error: unexpected token IDENT

type Bad
	def m()
		1
	end
	x := 1
end

#---

#! syntax error: unexpected token NUM

type T(a)
	1 + 1
end

#---

#! syntax error: unexpected token EQ

x
:= 5
