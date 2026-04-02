use core::fmt;
use core::str::FromStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TargetArch {
    #[default]
    X86_64,
    Aarch64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseTargetArchError;

impl fmt::Display for ParseTargetArchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unsupported target architecture")
    }
}

impl TargetArch {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
        }
    }

    #[must_use]
    pub const fn to_bare_metal_triple(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64-unknown-none",
            Self::Aarch64 => "aarch64-unknown-none",
        }
    }

    #[must_use]
    pub fn from_bare_metal_triple(value: &str) -> Option<Self> {
        match value {
            "x86_64-unknown-none" => Some(Self::X86_64),
            "aarch64-unknown-none" => Some(Self::Aarch64),
            _ => None,
        }
    }

    #[must_use]
    pub const fn supported() -> &'static [Self] {
        &[Self::X86_64, Self::Aarch64]
    }
}

impl fmt::Display for TargetArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TargetArch {
    type Err = ParseTargetArchError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "x86_64" => Ok(Self::X86_64),
            "aarch64" => Ok(Self::Aarch64),
            _ => Err(ParseTargetArchError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TargetArch;
    use core::str::FromStr;

    #[test]
    fn parses_known_arches() {
        assert_eq!(TargetArch::from_str("x86_64").ok(), Some(TargetArch::X86_64));
        assert_eq!(TargetArch::from_str("aarch64").ok(), Some(TargetArch::Aarch64));
    }

    #[test]
    fn rejects_unknown_arch() {
        assert!(TargetArch::from_str("riscv64").is_err());
    }

    #[test]
    fn roundtrip_triple_mapping() {
        for arch in TargetArch::supported() {
            let triple = arch.to_bare_metal_triple();
            assert_eq!(TargetArch::from_bare_metal_triple(triple), Some(*arch));
        }
    }
}