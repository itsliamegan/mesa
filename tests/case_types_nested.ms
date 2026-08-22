#> 4
#> Expr.Group.Span(1, 2)

type Expr
	case Ident(name)
	end

	case Group(items)
		# An ordinary nested type inside a case.
		type Span(lo, hi)
			def width()
				self.hi - self.lo
			end
		end
	end
end

$print(Expr.Group.Span(1, 5).width())
$print(Expr.Group.Span(1, 2))

#---

#> Ast.Expr.Ident("x")
#> type Ast.Expr.Ident(name)
#> lit

# A case type nested inside another type.
type Ast
	type Expr
		case Ident(name)
		end
		case Lit(value)
		end

		def label()
			"lit"
		end
	end
end

$print(Ast.Expr.Ident("x"))
$print(Ast.Expr.Ident)
$print(Ast.Expr.Lit(1).label())

#---

#> Stmt.Assign(Expr.Ident("x"))
#> x

# Hierarchy by nesting, not subtyping: a variant's field holds an instance of
# another case type.
type Expr
	case Ident(name)
	end
end

type Stmt
	case Assign(target)
	end
end

s := Stmt.Assign(Expr.Ident("x"))
$print(s)
$print(s.target.name)
