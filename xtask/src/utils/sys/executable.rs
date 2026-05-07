use anyhow::Result;

/// A trait for commands that can be executed independently.
pub trait Executable {
    fn execute(&self) -> Result<()>;
}
