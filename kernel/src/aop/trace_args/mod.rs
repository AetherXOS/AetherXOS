use aop_macros::trace_args;
use alloc::format;

#[trace_args(info)]
pub fn example_args(a: i32, b: &str) {
    let _ = a;
    let _ = b;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_trace_args() {
        example_args(42, "test");
    }
}
