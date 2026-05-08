//! AetherCursor: Advanced Bit/Byte Processing Engine for AetherXOS.
//! Provides high-performance, lazy, and endian-aware data manipulation.
//! Supports both checked (safe) and unchecked (fast-path) operations.

use super::{BitField32, BitField64, PacketRead, PacketWrite};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    Big,
    Little,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorError {
    Overflow,
    InvalidData,
}

pub type Result<T> = core::result::Result<T, CursorError>;

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
    #[inline(always)] pub fn skip(&mut self, len: usize) -> Result<()> {
        if self.pos + len > self.data.len() { return Err(CursorError::Overflow); }
        self.pos += len;
        Ok(())
    }
    #[inline(always)] pub fn remaining(&self) -> usize { self.data.len().saturating_sub(self.pos) }
    #[inline(always)] pub fn is_empty(&self) -> bool { self.pos >= self.data.len() }
    
    #[inline(always)]
    pub fn remaining_slice(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }

    // --- Generic Checked Reads ---
    
    pub fn read<T: PacketRead>(&mut self, endian: Endian) -> Result<T> {
        let size = core::mem::size_of::<T>();
        if self.pos + size > self.data.len() {
            return Err(CursorError::Overflow);
        }
        let val = unsafe {
            match endian {
                Endian::Big => T::read_be(self.data, self.pos),
                Endian::Little => T::read_le(self.data, self.pos),
            }
        };
        self.pos += size;
        Ok(val)
    }

    #[inline(always)] pub fn read_be<T: PacketRead>(&mut self) -> Result<T> { self.read(Endian::Big) }
    #[inline(always)] pub fn read_le<T: PacketRead>(&mut self) -> Result<T> { self.read(Endian::Little) }

    // --- Generic Peek Methods ---

    pub fn peek<T: PacketRead>(&self, endian: Endian, offset: usize) -> Result<T> {
        let size = core::mem::size_of::<T>();
        let p = self.pos + offset;
        if p + size > self.data.len() {
            return Err(CursorError::Overflow);
        }
        let val = unsafe {
            match endian {
                Endian::Big => T::read_be(self.data, p),
                Endian::Little => T::read_le(self.data, p),
            }
        };
        Ok(val)
    }

    #[inline(always)] pub fn peek_be<T: PacketRead>(&self, offset: usize) -> Result<T> { self.peek(Endian::Big, offset) }
    #[inline(always)] pub fn peek_le<T: PacketRead>(&self, offset: usize) -> Result<T> { self.peek(Endian::Little, offset) }

    // --- Unchecked Fast-Path ---

    /// # Safety
    /// Caller must ensure that the cursor has enough remaining bytes for this operation.
    #[inline(always)]
    pub unsafe fn read_unchecked<T: PacketRead>(&mut self, endian: Endian) -> T {
        let size = core::mem::size_of::<T>();
        let val = unsafe {
            match endian {
                Endian::Big => T::read_be(self.data, self.pos),
                Endian::Little => T::read_le(self.data, self.pos),
            }
        };
        self.pos += size;
        val
    }

    /// # Safety
    /// Caller must ensure that the cursor has enough remaining bytes for this operation.
    #[inline(always)]
    pub unsafe fn peek_unchecked<T: PacketRead>(&self, endian: Endian, offset: usize) -> T {
        let p = self.pos + offset;
        unsafe {
            match endian {
                Endian::Big => T::read_be(self.data, p),
                Endian::Little => T::read_le(self.data, p),
            }
        }
    }


    // --- Specific Types (Compatibility & Convenience) ---

    pub fn read_u8(&mut self) -> Result<u8> { self.read_be() }
    pub fn read_i8(&mut self) -> Result<i8> { self.read_be() }
    
    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        let slice = self.data.get(self.pos..self.pos+len).ok_or(CursorError::Overflow)?;
        self.pos += len;
        Ok(slice)
    }

    // --- Variable Length Integers (LEB128) ---

    pub fn read_varint_u64(&mut self) -> Result<u64> {
        let mut res = 0u64;
        let mut shift = 0;
        loop {
            let byte = self.read_u8()?;
            res |= ((byte & 0x7F) as u64) << shift;
            if byte & 0x80 == 0 {
                return Ok(res);
            }
            shift += 7;
            if shift >= 64 { return Err(CursorError::InvalidData); }
        }
    }

    // --- Bitfield Support ---

    pub fn read_bitfield32(&mut self, field: BitField32) -> Result<u32> {
        let val: u32 = self.read_be()?;
        Ok(field.read(val))
    }

    pub fn read_bitfield64(&mut self, field: BitField64) -> Result<u64> {
        let val: u64 = self.read_be()?;
        Ok(field.read(val))
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
    #[inline(always)] pub fn skip(&mut self, len: usize) -> Result<()> {
        if self.pos + len > self.data.len() { return Err(CursorError::Overflow); }
        self.pos += len;
        Ok(())
    }
    #[inline(always)] pub fn remaining(&self) -> usize { self.data.len().saturating_sub(self.pos) }

    // --- Generic Checked Writes ---

    pub fn write<T: PacketWrite>(&mut self, val: T, endian: Endian) -> Result<()> {
        let size = core::mem::size_of::<T>();
        if self.pos + size > self.data.len() {
            return Err(CursorError::Overflow);
        }
        unsafe {
            match endian {
                Endian::Big => val.write_be(self.data, self.pos),
                Endian::Little => val.write_le(self.data, self.pos),
            }
        }
        self.pos += size;
        Ok(())
    }

    #[inline(always)] pub fn write_be<T: PacketWrite>(&mut self, val: T) -> Result<()> { self.write(val, Endian::Big) }
    #[inline(always)] pub fn write_le<T: PacketWrite>(&mut self, val: T) -> Result<()> { self.write(val, Endian::Little) }

    pub fn write_u8(&mut self, val: u8) -> Result<()> { self.write_be(val) }
    
    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        if self.pos + bytes.len() > self.data.len() { return Err(CursorError::Overflow); }
        self.data[self.pos..self.pos+bytes.len()].copy_from_slice(bytes);
        self.pos += bytes.len();
        Ok(())
    }

    // --- Variable Length Integers (LEB128) ---

    pub fn write_varint_u64(&mut self, mut val: u64) -> Result<()> {
        loop {
            let mut byte = (val & 0x7F) as u8;
            val >>= 7;
            if val != 0 {
                byte |= 0x80;
            }
            self.write_u8(byte)?;
            if val == 0 { break; }
        }
        Ok(())
    }
}

/// BitCursor: High-performance bit-level stream processor.
pub struct BitCursor<'a> {
    cursor: DataCursor<'a>,
    bit_pos: u8,
    current_byte: u8,
}

impl<'a> BitCursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            cursor: DataCursor::new(data),
            bit_pos: 8,
            current_byte: 0,
        }
    }

    pub fn read_bit(&mut self) -> Result<bool> {
        if self.bit_pos == 8 {
            self.current_byte = self.cursor.read_u8()?;
            self.bit_pos = 0;
        }
        let bit = (self.current_byte & (1 << (7 - self.bit_pos))) != 0;
        self.bit_pos += 1;
        Ok(bit)
    }

    pub fn read_bits(&mut self, mut count: u8) -> Result<u64> {
        if count > 64 { return Err(CursorError::InvalidData); }
        let mut res = 0u64;
        while count > 0 {
            res <<= 1;
            if self.read_bit()? {
                res |= 1;
            }
            count -= 1;
        }
        Ok(res)
    }
}

/// BitWriter: High-performance bit-level stream encoder.
pub struct BitWriter<'a> {
    writer: DataWriter<'a>,
    bit_pos: u8,
    current_byte: u8,
}

impl<'a> BitWriter<'a> {
    pub fn new(data: &'a mut [u8]) -> Self {
        Self {
            writer: DataWriter::new(data),
            bit_pos: 0,
            current_byte: 0,
        }
    }

    pub fn write_bit(&mut self, bit: bool) -> Result<()> {
        if bit {
            self.current_byte |= 1 << (7 - self.bit_pos);
        }
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.writer.write_u8(self.current_byte)?;
            self.bit_pos = 0;
            self.current_byte = 0;
        }
        Ok(())
    }

    pub fn write_bits(&mut self, val: u64, mut count: u8) -> Result<()> {
        if count > 64 { return Err(CursorError::InvalidData); }
        while count > 0 {
            let bit = (val & (1 << (count - 1))) != 0;
            self.write_bit(bit)?;
            count -= 1;
        }
        Ok(())
    }

    /// Flushes any remaining bits to the underlying buffer.
    pub fn flush(&mut self) -> Result<()> {
        if self.bit_pos > 0 {
            self.writer.write_u8(self.current_byte)?;
            self.bit_pos = 0;
            self.current_byte = 0;
        }
        Ok(())
    }
}

