use aop_macros::retry;
use alloc::format;

#[retry(retries = 3)]
pub fn example_retry() -> Result<(), &'static str> {
    static mut ATTEMPTS: u32 = 0;
    unsafe {
        ATTEMPTS += 1;
        if ATTEMPTS < 2 {
            return Err("temporary error");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_retry() {
        assert!(example_retry().is_ok());
    }
}
