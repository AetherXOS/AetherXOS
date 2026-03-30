//! Macros for professional enum and flag management.
//! Reduces boilerplate for conversions, string representation, and bitwise operations.

/// Defines an enum with a powerful set of conversions and helper methods.
/// Supports:
/// - `to_raw`, `from_raw`
/// - `as_str`, `from_str`
/// - `Display`, `FromStr`, `From`, `TryFrom`, `Debug`
#[macro_export]
macro_rules! define_enum {
    ($vis:vis enum $name:ident : $raw:ty {
        $($variant:ident $(= $value:expr)? => $str:expr),+ $(,)?
    }) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr($raw)]
        $vis enum $name {
            $($variant $(= $value)?,)+
        }

        impl $name {
            #[inline(always)]
            pub const fn to_raw(self) -> $raw {
                self as $raw
            }

            #[inline(always)]
            pub const fn to_u8(self) -> u8 {
                self as u8
            }

            

            #[inline(always)]
            pub const fn from_raw(value: $raw) -> Option<Self> {
                match value {
                    $(x if x == Self::$variant as $raw => Some(Self::$variant),)+
                    _ => None,
                }
            }

            #[inline(always)]
            pub const fn from_u8(value: u8) -> Option<Self> {
                Self::from_raw(value as $raw)
            }

            #[inline(always)]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $str,)+
                }
            }

            #[inline(always)]
            pub fn from_str_enum(s: &str) -> Option<Self> {
                match s {
                    $($str => Some(Self::$variant),)+
                    _ => None,
                }
            }

            #[inline(always)]
            pub fn from_str(s: &str) -> Option<Self> {
                Self::from_str_enum(s)
            }

        }

        impl From<$name> for $raw {
            #[inline(always)]
            fn from(v: $name) -> Self { v as $raw }
        }

        impl core::convert::TryFrom<$raw> for $name {
            type Error = ();
            #[inline(always)]
            fn try_from(v: $raw) -> Result<Self, Self::Error> {
                Self::from_raw(v).ok_or(())
            }
        }

        impl core::fmt::Display for $name {
            #[inline(always)]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl core::str::FromStr for $name {
            type Err = ();
            #[inline(always)]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::from_str_enum(s).ok_or(())
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_tuple(stringify!($name))
                    .field(&self.as_str())
                    .finish()
            }
        }
    };

    // Version with automatic stringification
    ($vis:vis enum $name:ident : $raw:ty {
        $($variant:ident $(= $value:expr)?),+ $(,)?
    }) => {
        $crate::define_enum!($vis enum $name : $raw {
            $($variant $(= $value)? => stringify!($variant)),+
        });
    };
}

/// Implements a bitflags-style structure with professional extras.
#[macro_export]
macro_rules! define_flags {
    ($vis:vis struct $name:ident : $type:ty {
        $($flag:ident = $value:expr),+ $(,)?
    }) => {
        bitflags::bitflags! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            $vis struct $name: $type {
                $(const $flag = $value;)+
            }
        }

        impl $name {
            #[inline(always)]
            pub const fn to_raw(self) -> $type {
                self.bits()
            }

            #[inline(always)]
            pub const fn from_raw(bits: $type) -> Self {
                Self::from_bits_truncate(bits)
            }
        }

        impl From<$type> for $name {
            #[inline(always)]
            fn from(bits: $type) -> Self { Self::from_raw(bits) }
        }

        impl From<$name> for $type {
            #[inline(always)]
            fn from(f: $name) -> Self { f.to_raw() }
        }
    };
}

/// Implements common numeric and string conversions for enums.
#[macro_export]
macro_rules! impl_enum_u8_default_conversions {
    ($name:ident { $($variant:ident),* $(,)? }, default = $default_variant:ident) => {
        impl $name {
            #[inline(always)]
            pub const fn to_u8(self) -> u8 { self as u8 }
            #[inline(always)]
            pub const fn from_u8(v: u8) -> Option<Self> {
                match v {
                    $(x if x == Self::$variant as u8 => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }
        impl Default for $name {
            fn default() -> Self { Self::$default_variant }
        }
    };
    ($name:ident { $($variant:ident),* $(,)? }) => {
        impl $name {
            #[inline(always)]
            pub const fn to_u8(self) -> u8 { self as u8 }
            #[inline(always)]
            pub const fn from_u8(v: u8) -> Option<Self> {
                match v {
                    $(x if x == Self::$variant as u8 => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_enum_u8_option_conversions {
    ($name:ident { $($variant:ident),* $(,)? }) => {
        impl $name {
            #[inline(always)]
            pub const fn to_u8(self) -> u8 { self as u8 }
        }
    };
}

#[macro_export]
macro_rules! impl_enum_str_conversions {
    ($name:ident { $($variant:ident => $str:expr),* $(,)? }) => {
        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $str,)*
                }
            }
        }
    };
}
