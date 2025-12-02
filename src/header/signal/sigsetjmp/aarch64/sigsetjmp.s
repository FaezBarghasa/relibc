.global sigsetjmp
.global __sigsetjmp
.type sigsetjmp,%function
.type __sigsetjmp,%function
sigsetjmp:
__sigsetjmp:
	cbz x1,1f
	str x30,[x0,#176]
	str x19,[x0,#176+8+8]
	mov x19,x0
	bl __sigsetjmp_tail
	mov x0,x19
	ldr x30,[x0,#176]
	ldr x19,[x0,#176+8+8]
1:
	b setjmp
.hidden __sigsetjmp_tail
