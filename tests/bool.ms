#> true
$print(nil == nil)
#> true
$print(true != false)

#> true
$print(true and true)
#> false
$print(true and false)
#> true
$print(true or false)
#> false
$print(false or false)

#> false
$print(not true)
#> true
$print(not false)
#> false
$print(not 0)
#> false
$print(not 1)
#> true
$print(not nil)

#> false
$print(not true and false)
#> true
$print(not 1 == 2)
#> false
$print(not 1 == 1)

#> 2
$print(1 and 2)
#> nil
$print(nil and 2)
#> 1
$print(1 or 2)
#> 2
$print(nil or 2)
