#![cfg_attr(not(test), no_std)]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::harness::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

pub mod harness;
pub mod kernel;
pub mod sanitizers;
pub mod virtualization;
pub mod fuzzing;
pub mod hardware;

#[cfg(test)]
#[no_mangle]
pub fn main() -> Result<(), ()> {
    test_main();
    Ok(())
}
