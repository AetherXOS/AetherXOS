//! Base structures for bitfield and register value manipulation.

pub trait BitField {
    type Storage: Copy;
    fn mask() -> Self::Storage;
    fn shift() -> usize;
}

macro_rules! impl_bitfield {
    ($name:ident, $ty:ident) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $name {
            pub mask: $ty,
            pub shift: u32,
        }

        impl $name {
            pub const fn new(mask: $ty, shift: u32) -> Self {
                Self { mask, shift }
            }

            #[inline(always)]
            pub const fn read(&self, val: $ty) -> $ty {
                (val & (self.mask << self.shift)) >> self.shift
            }

            #[inline(always)]
            pub const fn write(&self, val: $ty, field_val: $ty) -> $ty {
                (val & !(self.mask << self.shift)) | ((field_val & self.mask) << self.shift)
            }

            #[inline(always)]
            pub const fn bit(&self, val: $ty) -> bool {
                (val & (1 << self.shift)) != 0
            }

            #[inline(always)]
            pub const fn set_bit(&self, val: $ty, set: bool) -> $ty {
                if set { val | (1 << self.shift) } else { val & !(1 << self.shift) }
            }
        }
    };
}

impl_bitfield!(BitField8, u8);
impl_bitfield!(BitField16, u16);
impl_bitfield!(BitField32, u32);
impl_bitfield!(BitField64, u64);

pub trait PacketRead {
    /// # Safety
    /// The caller must guarantee that `data` is at least `offset + size_of::<Self>()` bytes long.
    unsafe fn read_be(data: &[u8], offset: usize) -> Self;
    /// # Safety
    /// The caller must guarantee that `data` is at least `offset + size_of::<Self>()` bytes long.
    unsafe fn read_le(data: &[u8], offset: usize) -> Self;
}

macro_rules! impl_packet_read {
    ($($ty:ident),*) => {
        $(
            impl PacketRead for $ty {
                #[inline(always)]
                unsafe fn read_be(data: &[u8], offset: usize) -> Self {
                    let ptr = unsafe { data.as_ptr().add(offset) } as *const [u8; core::mem::size_of::<Self>()];
                    Self::from_be_bytes(unsafe { core::ptr::read_unaligned(ptr) })
                }
                #[inline(always)]
                unsafe fn read_le(data: &[u8], offset: usize) -> Self {
                    let ptr = unsafe { data.as_ptr().add(offset) } as *const [u8; core::mem::size_of::<Self>()];
                    Self::from_le_bytes(unsafe { core::ptr::read_unaligned(ptr) })
                }
            }
        )*
    };
}
impl_packet_read!(u16, i16, u32, i32, u64, i64);

impl PacketRead for u8 {
    #[inline(always)] unsafe fn read_be(data: &[u8], offset: usize) -> Self { unsafe { *data.get_unchecked(offset) } }
    #[inline(always)] unsafe fn read_le(data: &[u8], offset: usize) -> Self { unsafe { *data.get_unchecked(offset) } }
}

impl PacketRead for i8 {
    #[inline(always)] unsafe fn read_be(data: &[u8], offset: usize) -> Self { unsafe { *data.get_unchecked(offset) as i8 } }
    #[inline(always)] unsafe fn read_le(data: &[u8], offset: usize) -> Self { unsafe { *data.get_unchecked(offset) as i8 } }
}

#[macro_export]
macro_rules! aether_packet {
    (pub struct $struct_name:ident<'a> { $($fields:tt)* }) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name<'a>(&'a [u8]);

        impl<'a> $struct_name<'a> {
            pub const MIN_SIZE: usize = $crate::aether_packet!(@calc_size 0, $($fields)*);

            #[inline(always)]
            pub fn check_len(data: &[u8]) -> bool {
                data.len() >= Self::MIN_SIZE
            }

            #[inline(always)]
            pub fn new(data: &'a [u8]) -> Option<Self> {
                if Self::check_len(data) {
                    Some(Self(data))
                } else {
                    None
                }
            }

            /// # Safety
            /// Caller must guarantee that `data` is at least `Self::MIN_SIZE` bytes long.
            #[inline(always)]
            pub unsafe fn new_unchecked(data: &'a [u8]) -> Self {
                Self(data)
            }

            #[inline(always)]
            pub fn as_bytes(&self) -> &'a [u8] {
                self.0
            }

            $crate::aether_packet!(@impl_getters 0, $($fields)*);
        }

        impl<'a> core::fmt::Debug for $struct_name<'a> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let mut ds = f.debug_struct(stringify!($struct_name));
                $crate::aether_packet!(@impl_debug self, ds, $($fields)*);
                ds.finish()
            }
        }
    };

    // --- Size Calculation ---
    (@calc_size $acc:expr, $name:ident : $ty:ident $( ( $e:ident ) )? ; $($rest:tt)*) => {
        $crate::aether_packet!(@calc_size $acc + core::mem::size_of::<$ty>(), $($rest)*)
    };
    (@calc_size $acc:expr, $name:ident : [u8; $len:expr] ; $($rest:tt)*) => {
        $crate::aether_packet!(@calc_size $acc + $len, $($rest)*)
    };
    (@calc_size $acc:expr, bitfield $b:ident : $bty:ident $( ( $e:ident ) )? { $($bits:tt)* } $($rest:tt)*) => {
        $crate::aether_packet!(@calc_size $acc + core::mem::size_of::<$bty>(), $($rest)*)
    };
    (@calc_size $acc:expr, ) => { $acc };

    // --- High-Performance Getters (Fast-Path) ---
    (@impl_getters $offset:expr, $name:ident : [u8; $len:expr] ; $($rest:tt)*) => {
        #[inline(always)] pub fn $name(&self) -> [u8; $len] { unsafe { core::ptr::read_unaligned(self.0.as_ptr().add($offset) as *const [u8; $len]) } }
        $crate::aether_packet!(@impl_getters $offset + $len, $($rest)*);
    };
    (@impl_getters $offset:expr, $name:ident : $ty:ident ; $($rest:tt)*) => {
        #[inline(always)] pub fn $name(&self) -> $ty { unsafe { <$ty as $crate::kernel::bit_utils::PacketRead>::read_be(self.0, $offset) } }
        $crate::aether_packet!(@impl_getters $offset + core::mem::size_of::<$ty>(), $($rest)*);
    };
    (@impl_getters $offset:expr, $name:ident : $ty:ident (le) ; $($rest:tt)*) => {
        #[inline(always)] pub fn $name(&self) -> $ty { unsafe { <$ty as $crate::kernel::bit_utils::PacketRead>::read_le(self.0, $offset) } }
        $crate::aether_packet!(@impl_getters $offset + core::mem::size_of::<$ty>(), $($rest)*);
    };
    (@impl_getters $offset:expr, bitfield $block_name:ident : $bty:ident { $($bit_name:ident : $bit_ty:ty = $start:tt .. $end:tt ; )* } $($rest:tt)*) => {
        $(
            #[inline(always)] pub fn $bit_name(&self) -> $bit_ty {
                let val = unsafe { <$bty as $crate::kernel::bit_utils::PacketRead>::read_be(self.0, $offset) } as u64;
                let mask = (1 << ($end - $start)) - 1;
                ((val >> $start) & mask) as $bit_ty
            }
        )*
        $crate::aether_packet!(@impl_getters $offset + core::mem::size_of::<$bty>(), $($rest)*);
    };
    (@impl_getters $offset:expr, bitfield $block_name:ident : $bty:ident (le) { $($bit_name:ident : $bit_ty:ty = $start:tt .. $end:tt ; )* } $($rest:tt)*) => {
        $(
            #[inline(always)] pub fn $bit_name(&self) -> $bit_ty {
                let val = unsafe { <$bty as $crate::kernel::bit_utils::PacketRead>::read_le(self.0, $offset) } as u64;
                let mask = (1 << ($end - $start)) - 1;
                ((val >> $start) & mask) as $bit_ty
            }
        )*
        $crate::aether_packet!(@impl_getters $offset + core::mem::size_of::<$bty>(), $($rest)*);
    };
    (@impl_getters $offset:expr, ) => {};

    // --- Auto Debug Derivation ---
    (@impl_debug $self:ident, $ds:ident, $name:ident : $ty:ident $( ( $e:ident ) )? ; $($rest:tt)*) => {
        $ds.field(stringify!($name), &$self.$name());
        $crate::aether_packet!(@impl_debug $self, $ds, $($rest)*);
    };
    (@impl_debug $self:ident, $ds:ident, $name:ident : [u8; $len:expr] ; $($rest:tt)*) => {
        $ds.field(stringify!($name), &$self.$name());
        $crate::aether_packet!(@impl_debug $self, $ds, $($rest)*);
    };
    (@impl_debug $self:ident, $ds:ident, bitfield $block_name:ident : $bty:ident $( ( $e:ident ) )? { $($bit_name:ident : $bit_ty:ty = $start:tt .. $end:tt ; )* } $($rest:tt)*) => {
        $( $ds.field(stringify!($bit_name), &$self.$bit_name()); )*
        $crate::aether_packet!(@impl_debug $self, $ds, $($rest)*);
    };
    (@impl_debug $self:ident, $ds:ident, ) => {};
}

pub mod paging;

pub mod interrupts;
pub mod io;
pub mod cursor;
pub mod view;
pub mod x86_64_arch;
pub mod aarch64_arch;

pub use cursor::{DataCursor, DataWriter};
pub use view::PacketView;

pub use paging::{generic as paging_generic, x86 as paging_x86, BitField64 as PagingBitField64, PAGE_SIZE};
pub use interrupts::{apic, gic, pic};
pub use io::*;
pub use crate::kernel::bit_utils::io::{com, perf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_bitfield32_read_write() {
        let field = BitField32::new(0b11, 4);
        let val = 0u32;
        let new_val = field.write(val, 2);
        assert_eq!(new_val, 2 << 4);
        assert_eq!(field.read(new_val), 2);
    }

    #[test_case]
    fn test_bitfield32_bit() {
        let field = BitField32::new(1, 3);
        let val = 1 << 3;
        assert!(field.bit(val));
        assert!(!field.bit(0));
        let val2 = field.set_bit(0, true);
        assert_eq!(val2, 1 << 3);
        assert!(!field.bit(field.set_bit(val2, false)));
    }
}
