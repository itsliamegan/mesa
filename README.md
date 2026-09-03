# Mesa Programming Language

Mesa is a **hobby programming language** designed primarily for writing
**personal software** including web applications and command-line utilities.

It is inspired by Ruby, JavaScript, and Swift.

It is **dynamically** & **strongly** typed but features a subset of compile-time
semantic analysis which can achieve some of the correctness assurances of more
sophisticated static type systems.

Its is extensively documented: the [language](doc/language.md),
[libraries](doc/libraries.md), and [implementation](doc/implementation.md) each
have a comprehensive manual.

```mesa
module Ledger

import Core.IO
import Core.Behaviors.Equal
import Core.Behaviors.Hash
import Core.Behaviors.Display

type Amount(cents: 0)
	impl Display

	def self.of_dollars(dollars)
		self(cents: dollars * 100)
	end

	def dollars
		self.cents / 100
	end

	def display
		when self.cents < 0 then
			"-$" + Str(self.dollars)
		else
			"$" + Str(self.dollars)
		end
	end
end

type Transaction
	case Credit(amount) end
	case Debit(amount) end
end

type Account(id, transactions)
	impl Equal, Hash

	def balance
		total := 0
		each transaction in transactions do
			total := when transaction
			case Transaction.Credit then
				total + transaction.amount.cents
			case Transaction.Debit then
				total - transaction.amount.cents
			end
		end
		Amount(cents: total)
	end

	def equal(other)
		id == other.id
	end

	def hash
		id.hash
	end
end

account := Account(45203, [
	Transaction.Credit(Amount.of_dollars(105.21)),
	Transaction.Debit(Amount.of_dollars(30.74)),
])

IO.print(account.balance) # => $74.47
```
