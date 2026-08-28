num := 5
#> 5
$print(num)

type Point(x, y)
end

pt := Point(10.1, 0)
pt.x := 43.2
#> 43.2
$print(pt.x)

#> 9
$print((1 + 2) * 3)
#> 7
$print(1 + 2 * 3)

#> 3
$print(1 + 2)
#> 6.17
$print(5.27 + 0.9)
#> 2
$print(5 - 3)
#> 12
$print(4 * 3)
#> 2.5
$print(10 / 4)
#> 7
$print(1 + 2 * 3)
#> 10
$print(2 * 3 + 4)
#> 5
$print(10 - 2 - 3)
#> 1
$print(20 / 4 / 5)
#> 11
$print(2 + 3 * 4 - 6 / 2)

#> -5
$print(-5)
#> 1
$print(-2 + 3)
#> 1
$print(3 + -2)
#> -6
$print(-2 * 3)
#> 6
$print(-2 * -3)
#> 12
$print(10 - -2)
#> 5
$print(- -5)

type Task(priority)
end

task := Task(5)
#> 6
$print(task.priority + 1)
#> -5
$print(-task.priority)

#> true
$print(5 == 5)
#> true
$print(4 != 2)
#> true
$print(2 < 3)
#> true
$print(3 > 0)
#> true
$print(3 <= 3)
#> true
$print(2 <= 3)
#> true
$print(7 >= 7)
#> true
$print(7 >= 6)

#---

#! runtime error: type Str cannot be used in arithmetic

$print(5.1 + "str")
