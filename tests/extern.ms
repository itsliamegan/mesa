#! semantic error: extern type 'Counter' has no native implementation

extern type Counter
end

#---

#! semantic error: unknown protocol 'Bumpable'

extern type Counter impl Bumpable
end

#---

import Core.Counter

c := Counter.zero
#> 0
$print(c.count)
#> 1
$print(c.bump)
#> 2
$print(c.bump)
#> 5
$print(Counter.at(5).count)
#> 5
$print(Counter.at(5).bump_by(0))
#> 3
$print(Counter.zero.bump_by(3))
#> <native Counter>
$print(c)
#> type Counter
$print(Counter)

# 'order' is satisfied natively; 'min' and 'max' are acquired from the protocol
# and run as mesa procs on native data.
#> 3
$print(Counter.at(3).min(Counter.at(5)).count)
#> 5
$print(Counter.at(3).max(Counter.at(5)).count)
#> 0
$print(Counter.at(4).order(Counter.at(4)))
#> true
$print(Counter.at(3) < Counter.at(5))
#> true
$print(Counter.at(3) <= Counter.at(5))
#> true
$print(Counter.at(5) > Counter.at(3))
#> true
$print(Counter.at(5) >= Counter.at(3))

#---

#! runtime error: type Counter cannot be constructed

import Core.Counter

Counter()

#---

import Core.Counter

#> <native Counter>
$print(Counter.zero.inspect)
#> 5
$print((5).inspect)
#> "laundry"
$print("laundry".inspect)
#> ['a', 'b']
$print(['a', 'b'].inspect)
