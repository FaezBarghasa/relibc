
use core::ptr;

const HEAP_SIZE: usize = 1024 * 1024; // 1 MB heap

static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
static mut NEXT: usize = 0;

pub unsafe fn alloc(size: usize, align: usize) -> *mut u8 {
    let mut next = NEXT;
    if align > 1 {
        next = (next + align - 1) & !(align - 1);
    }

    if next + size > HEAP_SIZE {
        return ptr::null_mut();
    }

    NEXT = next + size;
    &mut HEAP[next] as *mut u8
}

pub unsafe fn alloc_zeroed(size: usize, align: usize) -> *mut u8 {
    let ptr = alloc(size, align);
    if !ptr.is_null() {
        ptr.write_bytes(0, size);
    }
    ptr
}
