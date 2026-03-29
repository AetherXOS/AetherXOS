use super::libc::{c_int, c_long, c_void, size_t, EINVAL, ENOSYS};

pub fn sys_read(_fd: c_int, _buf: *mut c_void, _count: size_t) -> c_long {
    -ENOSYS as c_long
}

pub fn sys_write(_fd: c_int, _buf: *const c_void, _count: size_t) -> c_long {
    -ENOSYS as c_long
}

pub fn sys_open(_pathname: *const c_char, _flags: c_int, _mode: u32) -> c_long {
    -ENOSYS as c_long
}

pub fn sys_close(_fd: c_int) -> c_long {
    -ENOSYS as c_long
}

pub fn sys_mmap(
    _addr: *mut c_void,
    _length: size_t,
    _prot: c_int,
    _flags: c_int,
    _fd: c_int,
    _offset: i64,
) -> c_long {
    -ENOSYS as c_long
}

pub fn sys_munmap(_addr: *mut c_void, _length: size_t) -> c_long {
    -ENOSYS as c_long
}

pub fn sys_brk(_addr: *mut c_void) -> c_long {
    -ENOSYS as c_long
}

pub fn sys_mprotect(_addr: *mut c_void, _len: size_t, _prot: c_int) -> c_long {
    -ENOSYS as c_long
}

pub fn sys_getpid() -> c_long {
    1
}

pub fn sys_gettid() -> c_long {
    1
}

pub fn sys_exit(_status: c_int) -> ! {
    loop {}
}

pub fn sys_exit_group(_status: c_int) -> ! {
    loop {}
}

pub fn sys_nanosleep(_req: *const Timespec, _rem: *mut Timespec) -> c_long {
    -ENOSYS as c_long
}

pub fn sys_clock_gettime(_clock_id: c_int, _tp: *mut Timespec) -> c_long {
    -ENOSYS as c_long
}

pub fn sys_futex(
    _uaddr: *mut u32,
    _op: c_int,
    _val: u32,
    _timeout: *const Timespec,
    _uaddr2: *mut u32,
    _val3: u32,
) -> c_long {
    -ENOSYS as c_long
}

pub struct Timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

pub type c_char = u8;
