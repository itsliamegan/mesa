#! semantic error: extern type 'Counter' has no native implementation

extern type Counter
end

#---

#! semantic error: unknown protocol 'Bumpable'

extern type Counter impl Bumpable
end

#---

#! syntax error: unexpected token IDENT

extern type Counter
	step := 1
end

#---

#! syntax error: unexpected token LPAREN

extern type Counter(step)
end
