use crate::platform::types::*;
use core::ptr;

pub unsafe fn introsort<F>(
    base: *mut c_char,
    nel: size_t,
    width: size_t,
    mut comp: F,
) where
    F: FnMut(*const c_void, *const c_void) -> c_int,
{
    let maxdepth = 2 * (size_t::BITS - nel.leading_zeros()) as size_t;
    introsort_helper(base, nel, width, maxdepth, &mut comp);
}

unsafe fn introsort_helper<F>(
    mut base: *mut c_char,
    mut nel: size_t,
    width: size_t,
    mut maxdepth: size_t,
    comp: &mut F,
) where
    F: FnMut(*const c_void, *const c_void) -> c_int,
{
    const THRESHOLD: size_t = 8;

    // this loop is a trick to save stack space because TCO is not a thing in Rustland
    // basically, we just change the arguments and loop rather than recursing for the second call
    // to introsort_helper()
    loop {
        if nel < THRESHOLD {
            insertion_sort(base, nel, width, comp);
            break;
        } else if nel > 1 {
            if maxdepth == 0 {
                heapsort(base, nel, width, comp);
                break;
            } else {
                let (left, right) = partition(base, nel, width, comp);
                let right_base = unsafe { base.add((right + 1) * width) };
                let right_nel = nel - (right + 1);
                maxdepth -= 1;
                if left < nel - right {
                    introsort_helper(base, left, width, maxdepth, comp);
                    base = right_base;
                    nel = right_nel;
                } else {
                    introsort_helper(right_base, right_nel, width, maxdepth, comp);
                    nel = left;
                }
            }
        }
    }
}

unsafe fn insertion_sort<F>(
    base: *mut c_char,
    nel: size_t,
    width: size_t,
    comp: &mut F,
) where
    F: FnMut(*const c_void, *const c_void) -> c_int,
{
    for i in 0..nel {
        for j in (0..i).rev() {
            let current = unsafe { base.add(j * width) };
            let prev = unsafe { base.add((j + 1) * width) };
            if comp(current as *const c_void, prev as *const c_void) > 0 {
                swap(current, prev, width);
            } else {
                break;
            }
        }
    }
}

unsafe fn heapsort<F>(
    base: *mut c_char,
    nel: size_t,
    width: size_t,
    comp: &mut F,
) where
    F: FnMut(*const c_void, *const c_void) -> c_int,
{
    heapify(base, nel, width, comp);

    let mut end = nel - 1;
    while end > 0 {
        let end_ptr = unsafe { base.add(end * width) };
        swap(end_ptr, base, width);
        end -= 1;
        heap_sift_down(base, 0, end, width, comp);
    }
}

unsafe fn heapify<F>(
    base: *mut c_char,
    nel: size_t,
    width: size_t,
    comp: &mut F,
) where
    F: FnMut(*const c_void, *const c_void) -> c_int,
{
    // we start at the last parent in the heap (the parent of the last child)
    let last_parent = (nel - 2) / 2;

    for start in (0..=last_parent).rev() {
        heap_sift_down(base, start, nel - 1, width, comp);
    }
}

unsafe fn heap_sift_down<F>(
    base: *mut c_char,
    start: size_t,
    end: size_t,
    width: size_t,
    comp: &mut F,
) where
    F: FnMut(*const c_void, *const c_void) -> c_int,
{
    // get the left child of the node at the given index
    let left_child = |idx| 2 * idx + 1;

    let mut root = start;

    while left_child(root) <= end {
        let child = left_child(root);
        let mut swap_idx = root;

        let root_ptr = unsafe { base.add(root * width) };
        let mut swap_ptr = unsafe { base.add(swap_idx * width) };
        let first_child_ptr = unsafe { base.add(child * width) };
        let second_child_ptr = unsafe { base.add((child + 1) * width) };

        if comp(swap_ptr as *const c_void, first_child_ptr as *const c_void) < 0 {
            swap_idx = child;
            swap_ptr = first_child_ptr;
        }
        if child < end && comp(swap_ptr as *const c_void, second_child_ptr as *const c_void) < 0 {
            swap_idx = child + 1;
            swap_ptr = second_child_ptr;
        }

        if swap_idx == root {
            break;
        } else {
            swap(root_ptr, swap_ptr, width);
            root = swap_idx;
        }
    }
}

#[inline]
unsafe fn partition<F>(
    base: *mut c_char,
    nel: size_t,
    width: size_t,
    comp: &mut F,
) -> (size_t, size_t)
where
    F: FnMut(*const c_void, *const c_void) -> c_int,
{
    // calculate the median of the first, middle, and last elements and use it as the pivot
    // to do fewer comparisons, also swap the elements into their correct positions
    let mut pivot = median_of_three(base, nel, width, comp);

    let mut i = 1;
    let mut j = 1;
    let mut n = nel - 2;

    // use this to deal with the Dutch national flag problem
    while j <= n {
        let i_ptr = unsafe { base.add(i * width) };
        let j_ptr = unsafe { base.add(j * width) };
        let n_ptr = unsafe { base.add(n * width) };
        let pivot_ptr = unsafe { base.add(pivot * width) };

        let comparison = comp(j_ptr as *const c_void, pivot_ptr as *const c_void);
        if comparison < 0 {
            swap(i_ptr, j_ptr, width);
            if i == pivot {
                pivot = j;
            }
            i += 1;
            j += 1;
        } else if comparison > 0 {
            swap(j_ptr, n_ptr, width);
            if n == pivot {
                pivot = j;
            }
            n -= 1;
        } else {
            j += 1;
        }
    }

    (i, n)
}

unsafe fn median_of_three<F>(
    base: *mut c_char,
    nel: size_t,
    width: size_t,
    comp: &mut F,
) -> size_t
where
    F: FnMut(*const c_void, *const c_void) -> c_int,
{
    let pivot = nel / 2;

    let mid = unsafe { base.add(pivot * width) };
    let last = unsafe { base.add((nel - 1) * width) };
    if comp(mid as *const c_void, base as *const c_void) < 0 {
        swap(mid, base, width);
    }
    if comp(last as *const c_void, mid as *const c_void) < 0 {
        swap(mid, last, width);
        if comp(mid as *const c_void, base as *const c_void) < 0 {
            swap(mid, base, width);
        }
    }

    pivot
}

#[inline]
unsafe fn swap(ptr1: *mut c_char, ptr2: *mut c_char, width: size_t) {
    ptr::swap_nonoverlapping(ptr1, ptr2, width);
}
