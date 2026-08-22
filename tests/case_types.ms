#> x
#> 1
#> true
#> false
#> Expr.Ident("x")
#> type Expr.Ident(name)

type Expr
	case Ident(name)
	end
	case Lit(value)
	end
end

$print(Expr.Ident("x").name)
$print(Expr.Lit(1).value)

$print(Expr.Ident == Expr.Ident)
$print(Expr.Ident == Expr.Lit)

$print(Expr.Ident("x"))
$print(Expr.Ident)

#---

#> an ident
#> a lit
#> an ident
#> 3
#> 7

type Node
	case Ident(name, line)
	end

	case Lit(value, line)
		def label()
			"lit"
		end
	end

	case Blank
	end

	def label()
		"ident"
	end

	def describe()
		when self.label() == "ident" then
			"an " + self.label()
		else
			"a " + self.label()
		end
	end

	def where()
		self.line
	end
end

$print(Node.Ident("x", 3).describe())
$print(Node.Lit(1, 7).describe())
$print(Node.Blank().describe())
$print(Node.Ident("x", 3).where())
$print(Node.Lit(1, 7).where())

#---

#> 0
#> 2
#> 0

type Tree
	case Leaf(value)
	end

	case Group(items)
		count := 0

		def self.empty()
			Tree.Group([])
		end

		def size()
			self.items.size
		end
	end
end

g := Tree.Group([Tree.Leaf(1), Tree.Leaf(2)])
$print(g.count)
$print(g.size())
$print(Tree.Group.empty().size())

#---

#> x
#> y

type Expr
	case Ident(label) end

	def label
		"z"
	end
end

expr := Expr.Ident("x")
$print(expr.label)
expr.label := "y"
$print(expr.label)
