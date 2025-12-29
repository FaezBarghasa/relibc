#[repr(C)]
pub struct Stack {
    pub argc: isize,
    pub argv: *mut *mut u8,
    pub base: *mut usize,
    pub len: usize,
}
