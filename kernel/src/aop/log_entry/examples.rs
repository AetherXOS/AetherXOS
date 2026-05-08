use super::*;
use alloc::format;

/// Example of logging entry and exit with duration.
#[log_entry(info, target = "examples", duration)]
pub fn logged_function() {
    crate::core::time::delay_cycles(100);
}

/// Example of silent function that might still be useful for other AOP aspects.
#[log_entry(debug, silent)]
pub fn silent_function() {
    // No logs will be emitted
}
