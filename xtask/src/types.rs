use aethercore_common::TargetArch;
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};

/// Unified architecture type for all xtask operations.
pub type Arch = TargetArch;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    clap::ValueEnum,
    Display,
    EnumString,
    EnumIter,
    IntoStaticStr,
    AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum Bootloader {
    #[default]
    Limine,
    Multiboot2,
    Grub,
    Direct,
}

impl Bootloader {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    clap::ValueEnum,
    Display,
    EnumString,
    EnumIter,
    IntoStaticStr,
    AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum ImageFormat {
    #[default]
    Iso,
    Img,
    Vhd,
}

impl ImageFormat {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    clap::ValueEnum,
    Display,
    EnumString,
    EnumIter,
    IntoStaticStr,
    AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum TestTier {
    #[default]
    Fast,
    Integration,
    Nightly,
}

impl TestTier {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    clap::ValueEnum,
    Display,
    EnumString,
    EnumIter,
    IntoStaticStr,
    AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum BuildProfile {
    #[default]
    Debug,
    Release,
}

impl BuildProfile {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }
}

impl BuildProfile {
    pub fn is_release(self) -> bool {
        matches!(self, Self::Release)
    }
}
