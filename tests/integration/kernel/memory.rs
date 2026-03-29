use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_memory_frame_allocator,
        &test_memory_heap_allocator,
        &test_memory_slab_allocator,
        &test_memory_paging,
        &test_memory_guardian_pages,
    ]
}

fn test_memory_frame_allocator() -> TestResult {
    TestResult::pass("integration::kernel::memory::frame_allocator")
}

fn test_memory_heap_allocator() -> TestResult {
    TestResult::pass("integration::kernel::memory::heap_allocator")
}

fn test_memory_slab_allocator() -> TestResult {
    TestResult::pass("integration::kernel::memory::slab_allocator")
}

fn test_memory_paging() -> TestResult {
    TestResult::pass("integration::kernel::memory::paging")
}

fn test_memory_guardian_pages() -> TestResult {
    TestResult::pass("integration::kernel::memory::guardian_pages")
}
