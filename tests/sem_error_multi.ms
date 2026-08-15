#! semantic error: required param 'b' follows defaulted param 'a'
#! semantic error: case type 'Tree' cannot declare params

def add(a: 0, b)
	a + b
end

type Tree(name)
	 case Node(data) end
end
