mod checks;
mod contracts;
mod self_test;

pub use self::self_test::{run_boot_self_tests, BootHealthReport};

#[cfg(test)]
mod tests;
