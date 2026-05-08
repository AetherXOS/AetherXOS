use strum::{Display, EnumString, EnumIter, IntoStaticStr, AsRefStr};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Display, EnumString, EnumIter, IntoStaticStr, AsRefStr)]
#[strum(serialize_all = "snake_case")]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "clap", clap(rename_all = "snake_case"))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TargetArch {
    #[default]
    X86_64,
    Aarch64,
}

impl TargetArch {
    pub fn as_str(&self) -> &'static str {
        self.into()
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
    pub fn supported() -> impl Iterator<Item = Self> {
        use strum::IntoEnumIterator;
        Self::iter()
    }
}

#[cfg(test)]
mod tests {
    use super::TargetArch;
    use core::str::FromStr;

    #[test]
    fn parses_known_arches() {
        assert_eq!(
            TargetArch::from_str("x86_64").ok(),
            Some(TargetArch::X86_64)
        );
        assert_eq!(
            TargetArch::from_str("aarch64").ok(),
            Some(TargetArch::Aarch64)
        );
    }

    #[test]
    fn rejects_unknown_arch() {
        assert!(TargetArch::from_str("riscv64").is_err());
    }

    #[test]
    fn roundtrip_triple_mapping() {
        for arch in TargetArch::supported() {
            let triple = arch.to_bare_metal_triple();
            assert_eq!(TargetArch::from_bare_metal_triple(triple), Some(arch));
        }
    }
}
