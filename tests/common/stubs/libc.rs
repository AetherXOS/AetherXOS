pub type c_int = i32;
pub type c_uint = u32;
pub type c_long = i64;
pub type c_ulong = u64;
pub type c_void = core::mem::MaybeUninit<u8>;
pub type size_t = usize;
pub type ssize_t = isize;
pub type off_t = i64;
pub type pid_t = i32;
pub type uid_t = u32;
pub type gid_t = u32;

pub const EINVAL: c_int = 22;
pub const ENOENT: c_int = 2;
pub const EEXIST: c_int = 17;
pub const EACCES: c_int = 13;
pub const EPERM: c_int = 1;
pub const ENOMEM: c_int = 12;
pub const EAGAIN: c_int = 11;
pub const EINTR: c_int = 4;
pub const EBADF: c_int = 9;
pub const EFAULT: c_int = 14;
pub const ENOTSUP: c_int = 95;
pub const ETIMEDOUT: c_int = 110;
pub const EINPROGRESS: c_int = 115;
pub const EALREADY: c_int = 114;
pub const ENOTCONN: c_int = 107;
pub const ECONNRESET: c_int = 104;
pub const EPIPE: c_int = 32;
pub const ENOSYS: c_int = 38;

pub const O_RDONLY: c_int = 0;
pub const O_WRONLY: c_int = 1;
pub const O_RDWR: c_int = 2;
pub const O_CREAT: c_int = 64;
pub const O_EXCL: c_int = 128;
pub const O_TRUNC: c_int = 512;
pub const O_APPEND: c_int = 1024;
pub const O_NONBLOCK: c_int = 2048;

pub const SEEK_SET: c_int = 0;
pub const SEEK_CUR: c_int = 1;
pub const SEEK_END: c_int = 2;

pub const PROT_NONE: c_int = 0;
pub const PROT_READ: c_int = 1;
pub const PROT_WRITE: c_int = 2;
pub const PROT_EXEC: c_int = 4;

pub const MAP_PRIVATE: c_int = 2;
pub const MAP_SHARED: c_int = 1;
pub const MAP_ANONYMOUS: c_int = 32;
pub const MAP_FIXED: c_int = 16;

pub fn stub_memcpy(_dest: *mut c_void, _src: *const c_void, _n: size_t) -> *mut c_void {
    core::ptr::null_mut()
}

pub fn stub_memset(_s: *mut c_void, _c: c_int, _n: size_t) -> *mut c_void {
    core::ptr::null_mut()
}

pub fn stub_memcmp(_s1: *const c_void, _s2: *const c_void, _n: size_t) -> c_int {
    0
}

pub fn stub_strlen(_s: *const c_char) -> size_t {
    0
}

pub fn stub_strcpy(_dest: *mut c_char, _src: *const c_char) -> *mut c_char {
    core::ptr::null_mut()
}

pub fn stub_strncmp(_s1: *const c_char, _s2: *const c_char, _n: size_t) -> c_int {
    0
}

pub type c_char = u8;
