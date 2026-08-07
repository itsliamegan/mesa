#> 3
#> 6.17
#> 2
#> 12
#> 2.5
#> 7
#> 10
#> 5
#> 1
#> 11
#> 6

$print(1 + 2)
$print(5.27 + 0.9)
$print(5 - 3)
$print(4 * 3)
$print(10 / 4)
$print(1 + 2 * 3)
$print(2 * 3 + 4)
$print(10 - 2 - 3)
$print(20 / 4 / 5)
$print(2 + 3 * 4 - 6 / 2)

type Box(val)
end

box := Box(5)
$print(box.val + 1)
