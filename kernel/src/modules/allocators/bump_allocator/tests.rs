//! # Safety
//!
//! All """unsafe""" blocks in this module are justified by the calling
//! functions which validate addresses, alignment, and invariants beforehand.
//!
use super::*;
use core::alloc::Layout;

#[test_case]
fn test_bump_allocator_basic_allocation() {
    let mut allocator = BumpAllocator::new();
    let mut buffer = [0usize; 512];
    let start = buffer.as_mut_ptr() as usize;
    let size = buffer.len() * core::mem::size_of::<usize>();
    unsafe { allocator.init(start, size); }

    let layout1 = Layout::from_size_align(128, 8).expect("unwrap failed - see module SAFETY docs");
    let ptr1 = unsafe { allocator.alloc(layout1) };
    assert!(!ptr1.is_null());

    let layout2 = Layout::from_size_align(256, 16).expect("unwrap failed - see module SAFETY docs");
    let ptr2 = unsafe { allocator.alloc(layout2) };
    assert!(!ptr2.is_null());
    assert!((ptr2 as usize) > (ptr1 as usize));
}

#[test_case]
fn test_bump_allocator_oom() {
    let mut allocator = BumpAllocator::new();
    let mut buffer = [0usize; 128];
    let start = buffer.as_mut_ptr() as usize;
    let size = buffer.len() * core::mem::size_of::<usize>();
    unsafe { allocator.init(start, size); }

    let layout = Layout::from_size_align(2048, 8).expect("unwrap failed - see module SAFETY docs");
    let ptr = unsafe { allocator.alloc(layout) };
    assert!(ptr.is_null(), "Expected allocation to fail (OOM)");
}


