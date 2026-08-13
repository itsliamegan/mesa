#! syn_errors.ms:6,11: syntax error: positional arg after keyword arg

def add(a, b)
	a + b
end
add(a: 1, 2)
