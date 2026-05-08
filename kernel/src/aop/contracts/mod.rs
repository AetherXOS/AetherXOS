use aop_macros::precondition;
use alloc::format;

#[precondition(val > 0)]
#[postcondition(__res < 100)]
pub fn example_contracts(val: u32) -> u32 {
    val % 100
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_contracts() {
        assert_eq!(example_contracts(10), 10);
    }
}
