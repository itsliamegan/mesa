#> Hello, world!
#> 13

str := "Hello, world!"
$print(str)
$print(str.size)
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
