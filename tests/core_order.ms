#> 3
#> 5
#> b
#> a
#> a
#> z

x := 5
y := 3
$print(x.min(y))
$print(x.max(y))
$print("a".max("b"))
$print("a".min("b"))
$print('a'.min('z'))
$print('a'.max('z'))

#---

#> true
#> false
#> 10

$print("".empty?)
$print("x".empty?)
$print([10, 20].first)
