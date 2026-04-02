use core::fmt;
use core::str::FromStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, clap::ValueEnum)]
pub enum Bootloader {
    #[default]
    Limine,
    Multiboot2,
    Grub,
    Direct,
}

impl Bootloader {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Limine => "limine",
            Self::Multiboot2 => "multiboot2",
            Self::Grub => "grub",
            Self::Direct => "direct",
        }
    }
}

impl fmt::Display for Bootloader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Bootloader {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "limine" => Ok(Self::Limine),
            "multiboot2" => Ok(Self::Multiboot2),
            "grub" => Ok(Self::Grub),
            "direct" => Ok(Self::Direct),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, clap::ValueEnum)]
pub enum ImageFormat {
    #[default]
    Iso,
    Img,
    Vhd,
}

impl ImageFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Iso => "iso",
            Self::Img => "img",
            Self::Vhd => "vhd",
        }
    }
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ImageFormat {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "iso" => Ok(Self::Iso),
            "img" => Ok(Self::Img),
            "vhd" => Ok(Self::Vhd),
            _ => Err(()),
        }
    }
}
