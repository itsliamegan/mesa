#! sem_error_case.ms:3,1: semantic error: case type 'Expr' cannot declare params

type Expr(shared)
	case Ident(name)
	end
end

#---

#! sem_error_case.ms:15,2: semantic error: case type 'Expr' cannot declare field 'seen'

type Expr
	case Ident(name)
	end
	seen := false
end
