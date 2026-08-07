#> (Num)	3
#> (Num)	6.17
#> (Num)	2
#> (Num)	12
#> (Num)	2.5
#> (Num)	7
#> (Num)	10
#> (Num)	5
#> (Num)	1
#> (Num)	11
#> (Num)	6

$dbg(1 + 2)
$dbg(5.27 + 0.9)
$dbg(5 - 3)
$dbg(4 * 3)
$dbg(10 / 4)
$dbg(1 + 2 * 3)
$dbg(2 * 3 + 4)
$dbg(10 - 2 - 3)
$dbg(20 / 4 / 5)
$dbg(2 + 3 * 4 - 6 / 2)

type Box(val)
end

box := Box(5)
$dbg(box.val + 1)
