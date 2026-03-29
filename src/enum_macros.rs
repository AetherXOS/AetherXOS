#[macro_export]
macro_rules! impl_enum_u8_option_conversions {
    ($enum:ident { $($variant:ident),+ $(,)? }) => {
        impl $enum {
            #[inline(always)]
            pub const fn to_u8(self) -> u8 {
                self as u8
            }

            #[inline(always)]
            pub const fn from_u8(value: u8) -> Option<Self> {
                match value {
                    $(x if x == Self::$variant as u8 => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }

        impl From<$enum> for u8 {
            #[inline(always)]
            fn from(value: $enum) -> Self {
                value as u8
            }
        }

        impl core::convert::TryFrom<u8> for $enum {
            type Error = ();

            #[inline(always)]
            fn try_from(value: u8) -> Result<Self, Self::Error> {
                Self::from_u8(value).ok_or(())
            }
        }
    };
}

#[macro_export]
macro_rules! impl_enum_u8_default_conversions {
    ($enum:ident { $($variant:ident),+ $(,)? }, default = $default:ident) => {
        impl $enum {
            #[inline(always)]
            pub const fn to_u8(self) -> u8 {
                self as u8
            }

            #[inline(always)]
            pub const fn try_from_u8(value: u8) -> Option<Self> {
                match value {
                    $(x if x == Self::$variant as u8 => Some(Self::$variant),)+
                    _ => None,
                }
            }

            #[inline(always)]
            pub const fn from_u8(value: u8) -> Self {
                match Self::try_from_u8(value) {
                    Some(v) => v,
                    None => Self::$default,
                }
            }
        }

        impl From<$enum> for u8 {
            #[inline(always)]
            fn from(value: $enum) -> Self {
                value as u8
            }
        }

        impl core::convert::TryFrom<u8> for $enum {
            type Error = ();

            #[inline(always)]
            fn try_from(value: u8) -> Result<Self, Self::Error> {
                Self::try_from_u8(value).ok_or(())
            }
        }
    };
}

#[macro_export]
macro_rules! impl_enum_str_conversions {
    ($enum:ident { $($variant:ident => $name:expr),+ $(,)? }) => {
        impl $enum {
            #[inline(always)]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)+
                }
            }

            #[inline(always)]
            pub fn from_str(value: &str) -> Option<Self> {
                match value {
                    $($name => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }

        impl core::fmt::Display for $enum {
            #[inline(always)]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl core::str::FromStr for $enum {
            type Err = ();

            #[inline(always)]
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($name => Ok(Self::$variant),)+
                    _ => Err(()),
                }
            }
        }
    };
}
