#! syn_errors.ms:6,11: syntax error: positional argument after keyword argument

def add(a, b)
	a + b
end
add(a: 1, 2)
#---
#! syn_errors.ms:10,8: syntax error: character literal must hold exactly one character

$print('ab')
#---
#! syn_errors.ms:14,8: syntax error: character literal must hold exactly one character

$print('')
#---
#! syn_errors.ms:18,8: syntax error: unterminated character literal

$print('a)
#---
#! syn_errors.ms:22,4: syntax error: unexpected token EQ

&x := 1
#---
#! syn_errors.ms:28,2: syntax error: unexpected token CASE

type Expr
	seen := false
	case Ident(name)
	end
end
#---
#! syn_errors.ms:36,3: syntax error: unexpected token CASE

type Expr
	case Ident(name)
		case Inner(x)
		end
	end
end
