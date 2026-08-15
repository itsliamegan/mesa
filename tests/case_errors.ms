#! runtime error: type Expr cannot be constructed

type Expr
	case Ident(name)
	end
end

Expr("x")

#---

#! runtime error: type Expr cannot be constructed

type Expr
	case Ident(name)
	end

	def self.make()
		self("x")
	end
end

Expr.make()

#---

#! runtime error: type Expr.Ident has no such member 'value'

type Expr
	case Ident(name)
	end
	case Lit(value)
	end
end

Expr.Ident("x").value

#---

#! runtime error: member 'label' on type Expr.Ident is read-only

type Expr
	case Ident(name)
	end

	def label()
		"e"
	end
end

Expr.Ident("x").label := 1

#---

#! runtime error: type Num cannot be matched against

n := 5
m := 7
when n case m then "seven" end

#---

#! syntax error: unexpected token LPAREN

def f()
	1
end

when 1 case f() then "x" end
