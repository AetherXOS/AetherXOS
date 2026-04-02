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
            const EXPECTED: &'static [&'static str] = &[$($str),+];

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

            #[inline(always)]
            pub fn parse(s: &str) -> core::result::Result<Self, $crate::result::ParseEnumError> {
                Self::from_str_enum(s).ok_or_else(|| {
                    $crate::result::ParseEnumError::new(
                        stringify!($name),
                        s,
                        Self::EXPECTED,
                    )
                })
            }
        }

        impl From<$name> for $raw {
            #[inline(always)]
            fn from(v: $name) -> Self {
                v as $raw
            }
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
            type Err = $crate::result::ParseEnumError;

            #[inline(always)]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::parse(s)
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.as_str()).finish()
            }
        }
    };

    ($vis:vis enum $name:ident : $raw:ty {
        $($variant:ident $(= $value:expr)?),+ $(,)?
    }) => {
        $crate::define_enum!($vis enum $name : $raw {
            $($variant $(= $value)? => stringify!($variant)),+
        });
    };
}

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

            #[inline(always)]
            pub const fn from_bits_strict(bits: $type) -> Option<Self> {
                Self::from_bits(bits)
            }
        }

        impl From<$type> for $name {
            #[inline(always)]
            fn from(bits: $type) -> Self {
                Self::from_raw(bits)
            }
        }

        impl From<$name> for $type {
            #[inline(always)]
            fn from(f: $name) -> Self {
                f.to_raw()
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let mut first = true;
                $(
                    if self.contains(Self::$flag) {
                        if !first {
                            f.write_str("|")?;
                        }
                        f.write_str(stringify!($flag))?;
                        first = false;
                    }
                )+

                if first {
                    f.write_str("empty")
                } else {
                    Ok(())
                }
            }
        }
    };
}

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

#[macro_export]
macro_rules! declare_counter_u64 {
    ($vis:vis $name:ident) => {
        $vis static $name: core::sync::atomic::AtomicU64 =
            core::sync::atomic::AtomicU64::new(0);
    };
}

#[macro_export]
macro_rules! counter_inc {
    ($name:ident) => {
        $name.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
    };
}

#[macro_export]
macro_rules! counter_add {
    ($name:ident, $value:expr) => {
        $name.fetch_add($value, core::sync::atomic::Ordering::Relaxed)
    };
}

#[macro_export]
macro_rules! counter_load {
    ($name:ident) => {
        $name.load(core::sync::atomic::Ordering::Relaxed)
    };
}

#[macro_export]
macro_rules! const_assert {
    ($cond:expr $(,)?) => {
        const _: () = assert!($cond);
    };
}

#[macro_export]
macro_rules! const_assert_size_eq {
    ($left:ty, $right:ty $(,)?) => {
        $crate::const_assert!(core::mem::size_of::<$left>() == core::mem::size_of::<$right>());
    };
}

#[macro_export]
macro_rules! const_assert_align_eq {
    ($left:ty, $right:ty $(,)?) => {
        $crate::const_assert!(core::mem::align_of::<$left>() == core::mem::align_of::<$right>());
    };
}