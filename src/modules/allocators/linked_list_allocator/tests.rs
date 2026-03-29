use super::*;
use core::alloc::Layout;

#[test_case]
fn test_linked_list_allocator_basic() {
    let allocator = LinkedListAllocator::new();
    let mut buffer = [0usize; 1024];
    let start = buffer.as_mut_ptr() as usize;
    let size = buffer.len() * core::mem::size_of::<usize>();
    allocator.init(start, size);

    let layout = Layout::from_size_align(256, 8).unwrap();
    let ptr1 = unsafe { allocator.alloc(layout) };
    assert!(!ptr1.is_null());

    let ptr2 = unsafe { allocator.alloc(layout) };
    assert!(!ptr2.is_null());
    assert!(ptr1 != ptr2);

    unsafe { allocator.dealloc(ptr1, layout) };
    let ptr3 = unsafe { allocator.alloc(layout) };
    assert!(!ptr3.is_null());
}
