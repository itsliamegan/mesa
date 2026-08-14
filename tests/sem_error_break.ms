#! sem_error_break.ms:3,1: semantic error: 'break' outside a loop

break

#---

#! sem_error_break.ms:11,3: semantic error: 'break' outside a loop

type T()
	def m()
		break
	end
end
