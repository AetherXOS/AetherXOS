#[cfg(test)]
mod tests {
    pub mod lifecycle_tests;
    pub mod resource_tests;
    pub mod signal_tests;
    pub mod util;
    pub mod wait_tests;

    pub use lifecycle_tests::*;
    pub use resource_tests::*;
    pub use signal_tests::*;
    pub use wait_tests::*;
}
