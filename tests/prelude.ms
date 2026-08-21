#> type Str
#> type Num
#> type List
#> type Char
#> 0
#> []
#> shadowed

$print(Str)
$print(Num)
$print(List)
$print(Char)
$print(Str().size)
$print(List())

Str := "shadowed"
$print(Str)
