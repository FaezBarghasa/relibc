.global sigsetjmp
.global __sigsetjmp
.type sigsetjmp,%function
.type __sigsetjmp,%function
sigsetjmp:
__sigsetjmp:
	beqz a1,1f
	sd ra,[a0,#208]
	sd s0,[a0,#208+8+8]
	mv s0,a0
	call __sigsetjmp_tail
	mv a0,s0
	ld ra,[a0,#208]
	ld s0,[a0,#208+8+8]
1:
	b setjmp
.hidden __sigsetjmp_tail
