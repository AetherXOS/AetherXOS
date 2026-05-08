use super::*;
use alloc::format;

/// Example of high-performance task tracing.
#[perf_trace(threshold = 1000)]
pub fn intensive_computation() {
    let mut sum = 0;
    for i in 0..1000 {
        sum += i;
    }
    core::hint::black_box(sum);
}

/// Example of async task tracing.
#[perf_trace(threshold = 5000)]
pub async fn async_computation() -> u32 {
    let mut sum = 0;
    for i in 0..500 {
        sum += i;
    }
    sum
}
