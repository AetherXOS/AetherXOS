#[path = "io/fd_ops.rs"]
mod fd_ops;
#[path = "io/path_ops.rs"]
mod path_ops;

#[cfg(not(feature = "linux_compat"))]
pub(crate) use fd_ops::{sys_linux_close, sys_linux_lseek, sys_linux_read, sys_linux_write};
#[cfg(not(feature = "linux_compat"))]
pub(crate) use path_ops::{
    sys_linux_access, sys_linux_faccessat, sys_linux_faccessat2, sys_linux_mkdirat, sys_linux_openat,
    sys_linux_openat2, sys_linux_readlinkat, sys_linux_renameat, sys_linux_renameat2,
    sys_linux_unlinkat,
};
