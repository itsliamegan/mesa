#> Hello, world!
#> 13

str := "Hello, world!"
$print(str)
$print(str.size)

#---

#> H
#> e
#> l
#> l
#> o

each char in "Hello" do
	$print(char)
end

#---

#> Hello, world!

str := "Hello, world"
str := str + "!"
$print(str)

#---

#> true
#> true
#> true
#> true

$print("apple" < "zebra")
$print("apple" <= "apple")
$print("zebra" > "apple")
$print("zebra" >= "zebra")

#---

#> ['H', 'e', 'l', 'l', 'o']
#> ['H', 'e', 'l', 'l', 'o']
#> b
#> 5

$print("Hello".chars)
$print("Hello".chars())
$print("abc".chars[1])
$print("Hello".chars.size)

#---

#> 5
#> 5

$print("héllo".size)
$print("héllo".chars.size)
