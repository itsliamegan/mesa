#> 0
#> 1
#> 2
#> 5
#> 5
#> 3
#> <native Counter>
#> type Counter
#> 3
#> 5
#> 0

import Core.Counter

c := Counter.zero
$print(c.count)
$print(c.bump)
$print(c.bump)
$print(Counter.at(5).count)
$print(Counter.at(5).bump_by(0))
$print(Counter.zero.bump_by(3))
$print(c)
$print(Counter)

# 'order' is satisfied natively; 'min' and 'max' are acquired from the protocol
# and run as mesa procs on native data.
$print(Counter.at(3).min(Counter.at(5)).count)
$print(Counter.at(3).max(Counter.at(5)).count)
$print(Counter.at(4).order(Counter.at(4)))

#---

#! runtime error: type Counter cannot be constructed

import Core.Counter

Counter()
