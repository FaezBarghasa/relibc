.section .text
.global longjmp
.global _longjmp
.type longjmp,@function
.type _longjmp,@function
_longjmp:
longjmp:
	// Restore callee-saved registers
	ld ra, 0(a0)
	ld sp, 8(a0)
	ld s0, 16(a0)
	ld s1, 24(a0)
	ld s2, 32(a0)
	ld s3, 40(a0)
	ld s4, 48(a0)
	ld s5, 56(a0)
	ld s6, 64(a0)
	ld s7, 72(a0)
	ld s8, 80(a0)
	ld s9, 88(a0)
	ld s10, 96(a0)
	ld s11, 104(a0)
	// Restore floating-point callee-saved registers
	fld fs0, 112(a0)
	fld fs1, 120(a0)
	fld fs2, 128(a0)
	fld fs3, 136(a0)
	fld fs4, 144(a0)
	fld fs5, 152(a0)
	fld fs6, 160(a0)
	fld fs7, 168(a0)
	fld fs8, 176(a0)
	fld fs9, 184(a0)
	fld fs10, 192(a0)
	fld fs11, 200(a0)
	mv a0, a1
	bnez a1, 1f
	li a0, 1
1:
	ret
.size longjmp, . - longjmp
