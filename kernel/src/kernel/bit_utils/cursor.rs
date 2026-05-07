//! AetherCursor: Advanced Bit/Byte Processing Engine for AetherXOS.
//! Provides high-performance, lazy, and endian-aware data manipulation.
//! Supports both checked (safe) and unchecked (fast-path) operations.

use core::convert::TryInto;
use super::{BitField32, BitField64};

#[derive(Debug, Clone, Copy)]
pub struct DataCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> DataCursor<'a> {
    #[inline(always)]
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    #[inline(always)] pub fn pos(&self) -> usize { self.pos }
    #[inline(always)] pub fn set_pos(&mut self, pos: usize) { self.pos = pos; }
    #[inline(always)] pub fn skip(&mut self, len: usize) { self.pos += len; }
    #[inline(always)] pub fn remaining(&self) -> usize { self.data.len().saturating_sub(self.pos) }
    #[inline(always)] pub fn is_empty(&self) -> bool { self.pos >= self.data.len() }

    pub fn read_u8(&mut self) -> Option<u8> {
        let val = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(val)
    }

    pub fn read_i8(&mut self) -> Option<i8> {
        self.read_u8().map(|v| v as i8)
    }

    pub fn read_bytes(&mut self, len: usize) -> Option<&'a [u8]> {
        let slice = self.data.get(self.pos..self.pos+len)?;
        self.pos += len;
        Some(slice)
    }

    /// # Safety
    /// Caller must ensure that the cursor has enough remaining bytes for this operation.
    #[inline(always)]
    pub unsafe fn read_u8_unchecked(&mut self) -> u8 {
        let val = unsafe { *self.data.get_unchecked(self.pos) };
        self.pos += 1;
        val
    }

    /// # Safety
    /// Caller must ensure that the cursor has enough remaining bytes for this operation.
    #[inline(always)]
    pub unsafe fn read_bytes_unchecked(&mut self, len: usize) -> &'a [u8] {
        let slice = unsafe { core::slice::from_raw_parts(self.data.as_ptr().add(self.pos), len) };
        self.pos += len;
        slice
    }

    pub fn peek_u8(&self, offset: usize) -> Option<u8> {
        self.data.get(self.pos + offset).copied()
    }

    pub fn read_bitfield32(&mut self, field: BitField32) -> Option<u32> {
        let val = self.read_u32_be()?;
        Some(field.read(val))
    }

    pub fn read_bitfield64(&mut self, field: BitField64) -> Option<u64> {
        let val = self.read_u64_be()?;
        Some(field.read(val))
    }
}

pub struct DataWriter<'a> {
    data: &'a mut [u8],
    pos: usize,
}

impl<'a> DataWriter<'a> {
    #[inline(always)]
    pub fn new(data: &'a mut [u8]) -> Self {
        Self { data, pos: 0 }
    }

    #[inline(always)] pub fn pos(&self) -> usize { self.pos }
    #[inline(always)] pub fn set_pos(&mut self, pos: usize) { self.pos = pos; }
    #[inline(always)] pub fn skip(&mut self, len: usize) { self.pos += len; }
    #[inline(always)] pub fn remaining(&self) -> usize { self.data.len().saturating_sub(self.pos) }

    pub fn write_u8(&mut self, val: u8) -> Result<(), &'static str> {
        if self.pos + 1 > self.data.len() { return Err("buffer overflow"); }
        self.data[self.pos] = val;
        self.pos += 1;
        Ok(())
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        if self.pos + bytes.len() > self.data.len() { return Err("buffer overflow"); }
        self.data[self.pos..self.pos+bytes.len()].copy_from_slice(bytes);
        self.pos += bytes.len();
        Ok(())
    }

    pub fn write_bitfield32(&mut self, field: BitField32, val: u32, field_val: u32) -> Result<(), &'static str> {
        self.write_u32_be(field.write(val, field_val))
    }

    pub fn write_bitfield64(&mut self, field: BitField64, val: u64, field_val: u64) -> Result<(), &'static str> {
        self.write_u64_be(field.write(val, field_val))
    }
}

macro_rules! impl_cursor_methods {
    ($ty:ident, $read_be:ident, $read_le:ident, $read_be_unchecked:ident, $peek_be:ident, $peek_le:ident, $write_be:ident, $write_le:ident) => {
        impl<'a> DataCursor<'a> {
            pub fn $read_be(&mut self) -> Option<$ty> {
                let val = $ty::from_be_bytes(self.data.get(self.pos..self.pos+core::mem::size_of::<$ty>())?.try_into().ok()?);
                self.pos += core::mem::size_of::<$ty>();
                Some(val)
            }
            pub fn $read_le(&mut self) -> Option<$ty> {
                let val = $ty::from_le_bytes(self.data.get(self.pos..self.pos+core::mem::size_of::<$ty>())?.try_into().ok()?);
                self.pos += core::mem::size_of::<$ty>();
                Some(val)
            }
            /// # Safety
            /// Caller must ensure that the cursor has enough remaining bytes for this operation.
            #[inline(always)]
            pub unsafe fn $read_be_unchecked(&mut self) -> $ty {
                let ptr = unsafe { self.data.as_ptr().add(self.pos) } as *const [u8; core::mem::size_of::<$ty>()];
                let val = $ty::from_be_bytes(unsafe { core::ptr::read_unaligned(ptr) });
                self.pos += core::mem::size_of::<$ty>();
                val
            }
            pub fn $peek_be(&self, offset: usize) -> Option<$ty> {
                let p = self.pos + offset;
                Some($ty::from_be_bytes(self.data.get(p..p+core::mem::size_of::<$ty>())?.try_into().ok()?))
            }
            pub fn $peek_le(&self, offset: usize) -> Option<$ty> {
                let p = self.pos + offset;
                Some($ty::from_le_bytes(self.data.get(p..p+core::mem::size_of::<$ty>())?.try_into().ok()?))
            }
        }
        
        impl<'a> DataWriter<'a> {
            pub fn $write_be(&mut self, val: $ty) -> Result<(), &'static str> {
                if self.pos + core::mem::size_of::<$ty>() > self.data.len() { return Err("buffer overflow"); }
                self.data[self.pos..self.pos+core::mem::size_of::<$ty>()].copy_from_slice(&val.to_be_bytes());
                self.pos += core::mem::size_of::<$ty>();
                Ok(())
            }
            pub fn $write_le(&mut self, val: $ty) -> Result<(), &'static str> {
                if self.pos + core::mem::size_of::<$ty>() > self.data.len() { return Err("buffer overflow"); }
                self.data[self.pos..self.pos+core::mem::size_of::<$ty>()].copy_from_slice(&val.to_le_bytes());
                self.pos += core::mem::size_of::<$ty>();
                Ok(())
            }
        }
    };
}

impl_cursor_methods!(u16, read_u16_be, read_u16_le, read_u16_be_unchecked, peek_u16_be, peek_u16_le, write_u16_be, write_u16_le);
impl_cursor_methods!(i16, read_i16_be, read_i16_le, read_i16_be_unchecked, peek_i16_be, peek_i16_le, write_i16_be, write_i16_le);
impl_cursor_methods!(u32, read_u32_be, read_u32_le, read_u32_be_unchecked, peek_u32_be, peek_u32_le, write_u32_be, write_u32_le);
impl_cursor_methods!(i32, read_i32_be, read_i32_le, read_i32_be_unchecked, peek_i32_be, peek_i32_le, write_i32_be, write_i32_le);
impl_cursor_methods!(u64, read_u64_be, read_u64_le, read_u64_be_unchecked, peek_u64_be, peek_u64_le, write_u64_be, write_u64_le);
impl_cursor_methods!(i64, read_i64_be, read_i64_le, read_i64_be_unchecked, peek_i64_be, peek_i64_le, write_i64_be, write_i64_le);

