#! src/positional_after_keyword.ms:4,11: syntax error: positional argument after keyword argument
#! src/multi_char_literal.ms:1,8: syntax error: character literal must hold exactly one character
#! src/empty_char_literal.ms:1,8: syntax error: character literal must hold exactly one character
#! src/unterminated_char_literal.ms:1,8: syntax error: unterminated character literal
#! src/bad_ref_assign.ms:1,4: syntax error: unexpected token EQ
#! src/case_after_field.ms:3,2: syntax error: unexpected token CASE
#! src/nested_case.ms:3,3: syntax error: unexpected token CASE
#! src/paren_in_type_body.ms:2,2: syntax error: unexpected token LPAREN
#! src/field_after_method.ms:5,2: syntax error: unexpected token IDENT
#! src/num_in_type_body.ms:2,2: syntax error: unexpected token NUM
#! src/newline_before_assign.ms:2,1: syntax error: unexpected token EQ
#! src/multi_num_tokens.ms:1,10: syntax error: unexpected token NUM
#! src/bad_char.ms:1,6: syntax error: unexpected character '@'
#! src/unknown_builtin.ms:1,1: syntax error: unknown builtin '$unknown'
#! src/call_as_case.ms:5,14: syntax error: unexpected token LPAREN
#! src/extern_field.ms:2,2: syntax error: unexpected token IDENT
#! src/extern_ctor.ms:1,20: syntax error: unexpected token LPAREN
#! src/extern_def_body.ms:3,3: syntax error: unexpected token NUM
#! src/extern_def_outside.ms:2,2: syntax error: unexpected token EXTERN
#! src/extern_case.ms:2,2: syntax error: unexpected token CASE
#! src/extern_nested_type.ms:2,2: syntax error: unexpected token TYPE

#:positional_after_keyword.ms
def add(a, b)
	a + b
end
add(a: 1, 2)

#:multi_char_literal.ms
$print('ab')

#:empty_char_literal.ms
$print('')

#:unterminated_char_literal.ms
$print('a)

#:bad_ref_assign.ms
&x := 1

#:case_after_field.ms
type Expr
	seen := false
	case Ident(name)
	end
end

#:nested_case.ms
type Expr
	case Ident(name)
		case Inner(x)
		end
	end
end

#:paren_in_type_body.ms
type Point
	(1)
end

#:field_after_method.ms
type Bad
	def m()
		1
	end
	x := 1
end

#:num_in_type_body.ms
type T(a)
	1 + 1
end

#:newline_before_assign.ms
x
:= 5

#:multi_num_tokens.ms
$print(1 2)

#:bad_char.ms
x := @

#:unknown_builtin.ms
$unknown(1)

#:call_as_case.ms
def f()
	1
end

when 1 case f() then "x" end

#:extern_field.ms
extern type Counter
	step := 1
end

#:extern_ctor.ms
extern type Counter(step)
end

#:extern_def_body.ms
extern type Counter
	extern def count
		1
	end
end

#:extern_def_outside.ms
type Tally
	extern def count end
end

#:extern_case.ms
extern type Counter
	case Zero end
end

#:extern_nested_type.ms
extern type Counter
	type Inner end
end
