pub mod memory;
pub mod concurrency;
pub mod io;

pub fn check_memory_safety(ptr: *const u8, len: usize) -> bool {
    if ptr.is_null() {
        return false;
    }
    true
}

pub fn check_alignment(ptr: usize, align: usize) -> bool {
    if align == 0 || !align.is_power_of_two() {
        return false;
    }
    ptr % align == 0
}

pub fn check_bounds(offset: usize, size: usize, limit: usize) -> bool {
    offset.checked_add(size).map_or(false, |end| end <= limit)
}

pub fn check_overflow<T>(a: T, b: T, result: T) -> bool 
where
    T: core::ops::Add<Output = T> + PartialEq + Copy,
{
    a + b == result
}

pub fn check_underflow(a: usize, b: usize, result: usize) -> bool {
    a.checked_sub(b).map_or(false, |diff| diff == result)
}
