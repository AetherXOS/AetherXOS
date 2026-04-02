//! Shared result and error helpers for small utilities and policy modules.

use alloc::string::String;
use core::fmt;

/// Minimal shared error type used by shared helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedError {
    InvalidInput,
    Unsupported,
    OutOfRange,
}

impl fmt::Display for SharedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid input"),
            Self::Unsupported => f.write_str("unsupported value"),
            Self::OutOfRange => f.write_str("value out of range"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SharedError {}

/// Rich parse error used by enum conversion macros.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseEnumError {
    pub enum_name: &'static str,
    pub input: String,
    pub expected: &'static [&'static str],
}

impl ParseEnumError {
    #[must_use]
    pub fn new(enum_name: &'static str, input: &str, expected: &'static [&'static str]) -> Self {
        Self {
            enum_name,
            input: String::from(input),
            expected,
        }
    }
}

impl fmt::Display for ParseEnumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid ")?;
        f.write_str(self.enum_name)?;
        f.write_str(" value '")?;
        f.write_str(&self.input)?;
        f.write_str("' (expected: ")?;

        for (index, expected) in self.expected.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            f.write_str(expected)?;
        }

        f.write_str(")")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseEnumError {}

/// Shared result alias for small policy/helper modules.
pub type SharedResult<T> = core::result::Result<T, SharedError>;
