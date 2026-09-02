type Transaction
	case Credit(amount, note)
		type Period(start, finish)
			def days
				self.finish - self.start
			end
		end
	end

	case Debit(amount, note)
		def label
			"debit"
		end
	end

	case Void
	end

	def label
		"credit"
	end

	def describe
		"a " + self.label
	end

	def where
		self.note
	end
end

#> 50
$print(Transaction.Credit(50, "rent").amount)
#> 1
$print(Transaction.Debit(1, "atm").amount)

#> true
$print(Transaction.Credit == Transaction.Credit)
#> false
$print(Transaction.Credit == Transaction.Debit)

#> Transaction.Credit(amount: 50, note: "rent")
$print(Transaction.Credit(50, "rent"))
#> type Transaction.Credit(amount, note)
$print(Transaction.Credit)

#> a credit
$print(Transaction.Credit(1, "rent").describe)
#> a debit
$print(Transaction.Debit(1, "atm").describe)
#> a credit
$print(Transaction.Void().describe)
#> rent
$print(Transaction.Credit(1, "rent").where)
#> atm
$print(Transaction.Debit(1, "atm").where)

#> 4
$print(Transaction.Credit.Period(1, 5).days)
#> Transaction.Credit.Period(start: 1, finish: 2)
$print(Transaction.Credit.Period(1, 2))

type Entry
	case Posted(transaction)
	end

	case Batch(entries)
		count := 0

		def self.empty
			Entry.Batch([])
		end

		def size
			self.entries.size
		end
	end
end

b := Entry.Batch([Entry.Posted(Transaction.Credit(1, "rent")), Entry.Posted(Transaction.Debit(2, "atm"))])
#> Entry.Posted(transaction: Transaction.Credit(amount: 1, note: "rent"))
$print(b.entries[0])
#> 1
$print(b.entries[0].transaction.amount)
#> 0
$print(b.count)
#> 2
$print(b.size)
#> 0
$print(Entry.Batch.empty.size)

type Ledger
	type Transaction
		case Credit(amount)
		end
		case Debit(amount)
		end

		def label
			"debit"
		end
	end
end

#> Ledger.Transaction.Credit(amount: 50)
$print(Ledger.Transaction.Credit(50))
#> type Ledger.Transaction.Credit(amount)
$print(Ledger.Transaction.Credit)
#> debit
$print(Ledger.Transaction.Debit(1).label)

type Receipt
	case Line(note)
	end

	def note
		"z"
	end
end

line := Receipt.Line("x")
#> x
$print(line.note)
line.note := "y"
#> y
$print(line.note)

#---

#! runtime error: type Transaction cannot be constructed

type Transaction
	case Credit(amount)
	end
end

Transaction(5)

#---

#! runtime error: type Transaction cannot be constructed

type Transaction
	case Credit(amount)
	end

	def self.make
		self(5)
	end
end

Transaction.make

#---

#! runtime error: type Transaction.Credit has no such member 'note'

type Transaction
	case Credit(amount)
	end
	case Debit(note)
	end
end

Transaction.Credit(5).note

#---

#! runtime error: member 'label' on type Transaction.Credit is read-only

type Transaction
	case Credit(amount)
	end

	def label
		"credit"
	end
end

Transaction.Credit(5).label := 1

#---

#! runtime error: type Num cannot be matched against

n := 5
m := 7
when n case m then "seven" end

#---

#! semantic error: case type 'Transaction' cannot declare params
#! semantic error: case type 'Ledger' cannot declare field 'seen'

type Transaction(shared)
	case Credit(amount)
	end
end

type Ledger
	case Credit(amount)
	end
	seen := false
end
