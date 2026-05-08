//! Typed MMIO helpers: MappedRegion and VolatileCell for safe-ish access.
//!
//! These are minimal, zero-dependency helpers that wrap raw pointer
//! operations behind small typed wrappers and document safety.

use core::marker::PhantomData;

/// A volatile cell wrapper over a MMIO register.
#[repr(transparent)]
pub struct VolatileCell<T> {
    _marker: PhantomData<T>,
}

impl<T> VolatileCell<T> {
    /// SAFETY: Caller must ensure `addr` is a valid MMIO mapped address for type `T`.
    #[inline(always)]
    pub unsafe fn read_at(addr: usize) -> T {
        unsafe { core::ptr::read_volatile(addr as *const T) }
    }

    /// SAFETY: Caller must ensure `addr` is a valid MMIO mapped address for type `T`.
    #[inline(always)]
    pub unsafe fn write_at(addr: usize, val: T) {
        unsafe { core::ptr::write_volatile(addr as *mut T, val) };
    }
}

/// Lightweight mapped region handle. Zero-sized; encodes base at type level.
pub struct MappedRegion<const BASE: usize>;

impl<const BASE: usize> MappedRegion<BASE> {
    /// Read a register at byte `offset` from the base as `T`.
    /// SAFETY: offset + size_of::<T> must be valid within the mapped region.
    #[inline(always)]
    pub unsafe fn read<T>() -> T {
        unsafe { VolatileCell::<T>::read_at(BASE) }
    }

    /// Read at offset.
    #[inline(always)]
    pub unsafe fn read_offset<T>(offset: usize) -> T {
        unsafe { VolatileCell::<T>::read_at(BASE + offset) }
    }

    /// Write at offset.
    #[inline(always)]
    pub unsafe fn write_offset<T>(offset: usize, val: T) {
        unsafe { VolatileCell::<T>::write_at(BASE + offset, val) };
    }
}
