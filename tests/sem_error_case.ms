#! semantic error: case type 'Expr' cannot declare params

type Expr(shared)
	case Ident(name)
	end
end

#---

#! semantic error: case type 'Expr' cannot declare field 'seen'

type Expr
	case Ident(name)
	end
	seen := false
end
