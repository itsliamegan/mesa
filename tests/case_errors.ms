#! case_errors.ms:9,5: runtime error: type Expr cannot be constructed
#! 	at <main> (case_errors.ms:9,5)

type Expr
	case Ident(name)
	end
end

Expr("x")

#---

#! case_errors.ms:22,7: runtime error: type Expr cannot be constructed
#! 	at make (case_errors.ms:22,7)
#! 	at <main> (case_errors.ms:26,10)

type Expr
	case Ident(name)
	end

	def self.make()
		self("x")
	end
end

Expr.make()

#---

#! case_errors.ms:40,16: runtime error: type Expr.Ident has no such member 'value'
#! 	at <main> (case_errors.ms:40,16)

type Expr
	case Ident(name)
	end
	case Lit(value)
	end
end

Expr.Ident("x").value

#---

#! case_errors.ms:56,23: runtime error: member 'label' on type Expr.Ident is read-only
#! 	at <main> (case_errors.ms:56,23)

type Expr
	case Ident(name)
	end

	def label()
		"e"
	end
end

Expr.Ident("x").label := 1

#---

#! case_errors.ms:74,23: runtime error: member 'label' on type Expr.Ident is read-only
#! 	at <main> (case_errors.ms:74,23)

# A parent method name is refused as a write target even when the variant has a
# real field of that name — the same rule a single type already applies.
type Expr
	case Ident(label)
	end

	def label()
		"e"
	end
end

Expr.Ident("x").label := 1
