use super::helpers::linux_fault;
use crate::kernel::syscalls::{with_user_read_bytes, with_user_write_bytes};
use core::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Fd(pub i32);

impl From<usize> for Fd {
    fn from(v: usize) -> Self {
        Self(v as i32)
    }
}

impl Fd {
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
    pub fn as_u32(&self) -> u32 {
        self.0 as u32
    }
    pub fn as_isize(&self) -> isize {
        self.0 as isize
    }
    pub fn as_i32(&self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct UserPtr<T> {
    pub addr: usize,
    _marker: PhantomData<*const T>,
}

impl<T> UserPtr<T> {
    pub fn new(addr: usize) -> Self {
        Self {
            addr,
            _marker: PhantomData,
        }
    }
    pub fn is_null(&self) -> bool {
        self.addr == 0
    }
    pub fn cast<U>(&self) -> UserPtr<U> {
        UserPtr::new(self.addr)
    }
    pub fn add(&self, count: usize) -> Self {
        Self::new(self.addr.wrapping_add(count * core::mem::size_of::<T>()))
    }
    pub fn offset(&self, count: usize) -> Self {
        self.add(count)
    }

    pub fn read(&self) -> Result<T, usize>
    where
        T: Copy,
    {
        if self.is_null() {
            return Err(linux_fault());
        }
        with_user_read_bytes(self.addr, core::mem::size_of::<T>(), |src| {
            let mut tmp = unsafe { core::mem::zeroed() };
            unsafe {
                core::ptr::copy_nonoverlapping(
                    src.as_ptr(),
                    &mut tmp as *mut T as *mut u8,
                    core::mem::size_of::<T>(),
                )
            };
            tmp
        })
        .map_err(|_| linux_fault())
    }

    pub fn write(&self, val: &T) -> Result<(), usize>
    where
        T: Copy,
    {
        if self.is_null() {
            return Err(linux_fault());
        }
        with_user_write_bytes(self.addr, core::mem::size_of::<T>(), |dst| {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    val as *const T as *const u8,
                    dst.as_mut_ptr(),
                    core::mem::size_of::<T>(),
                )
            };
            0
        })
        .map(|_| ())
        .map_err(|_| linux_fault())
    }

    pub fn write_bytes(&self, bytes: &[u8]) -> Result<(), usize> {
        if self.is_null() {
            return Err(linux_fault());
        }
        with_user_write_bytes(self.addr, bytes.len(), |dst| {
            dst.copy_from_slice(bytes);
            0
        })
        .map(|_| ())
        .map_err(|_| linux_fault())
    }

    pub fn read_bytes_with<F>(&self, len: usize, mut f: F) -> Result<usize, usize>
    where
        F: FnMut(&[u8]) -> usize,
    {
        if self.is_null() {
            return Err(linux_fault());
        }
        with_user_read_bytes(self.addr, len, |src| f(src)).map_err(|_| linux_fault())
    }

    pub fn read_bytes_with_limit<F>(&self, len: usize, limit: usize, f: F) -> Result<usize, usize>
    where
        F: FnMut(&[u8]) -> usize,
    {
        let actual = len.min(limit);
        self.read_bytes_with(actual, f)
    }

    pub fn read_bytes(&self, out: &mut [u8]) -> Result<(), usize> {
        self.read_bytes_with(out.len(), |src| {
            out.copy_from_slice(src);
            out.len()
        })
        .map(|_| ())
    }
}

/// Specialized write helper for unknown lengths (like read() syscalls).
impl UserPtr<u8> {
    pub fn write_bytes_with<F>(&self, len: usize, mut f: F) -> Result<usize, usize>
    where
        F: FnMut(&mut [u8]) -> usize,
    {
        if self.is_null() {
            return Err(linux_fault());
        }
        with_user_write_bytes(self.addr, len, |dst| f(dst)).map_err(|_| linux_fault())
    }

    pub fn as_str(&self) -> Result<&'static str, usize> {
        let s = super::helpers::read_user_c_string(
            self.addr,
            crate::config::KernelConfig::vfs_max_mount_path(),
        )?;
        let boxed: alloc::boxed::Box<str> = s.into_boxed_str();
        let leaked: &'static str = alloc::boxed::Box::leak(boxed);
        Ok(leaked)
    }
}

pub struct UserString {
    pub ptr: UserPtr<u8>,
}
impl UserString {
    pub fn new(addr: usize) -> Self {
        Self {
            ptr: UserPtr::new(addr),
        }
    }
    pub fn read(&self, max_len: usize) -> Result<alloc::string::String, usize> {
        super::helpers::read_user_c_string(self.ptr.addr, max_len)
    }
}
