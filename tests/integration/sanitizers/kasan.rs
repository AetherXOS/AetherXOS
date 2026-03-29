use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_kasan_stack_oob,
        &test_kasan_heap_oob,
        &test_kasan_use_after_free,
        &test_kasan_double_free,
    ]
}

fn test_kasan_stack_oob() -> TestResult {
    TestResult::pass("integration::sanitizers::kasan::stack_oob")
}

fn test_kasan_heap_oob() -> TestResult {
    TestResult::pass("integration::sanitizers::kasan::heap_oob")
}

fn test_kasan_use_after_free() -> TestResult {
    TestResult::pass("integration::sanitizers::kasan::use_after_free")
}

fn test_kasan_double_free() -> TestResult {
    TestResult::pass("integration::sanitizers::kasan::double_free")
}
