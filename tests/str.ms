str := "Hello, world!"
#> Hello, world!
$print(str)
#> 13
$print(str.size)

#> H
#> e
#> l
#> l
#> o
each char in "Hello" do
	$print(char)
end

str := "Hello, world"
str := str + "!"
#> Hello, world!
$print(str)

#> true
$print("str" == "str")
#> true
$print("this" != "that")
#> true
$print("apple" < "zebra")
#> true
$print("apple" <= "apple")
#> true
$print("zebra" > "apple")
#> true
$print("zebra" >= "zebra")

#> true
$print('a' < 'z')
#> true
$print('b' <= 'b')
#> true
$print('Z' > 'A')
#> true
$print('B' >= 'B')

#> ['H', 'e', 'l', 'l', 'o']
$print("Hello".chars)
#> ['H', 'e', 'l', 'l', 'o']
$print("Hello".chars())
#> b
$print("abc".chars[1])
#> 5
$print("Hello".chars.size)

#> 5
$print("héllo".size)
#> 5
$print("héllo".chars.size)

#> 0
$print(Str.empty.size)
#> 0
$print(Str.empty().size)
#> true
$print(Str.empty == "")
#> Hello
$print(Str.empty + "Hello")

#---

#! runtime error: type Num cannot be concatenated

$print("str" + 5.1)
