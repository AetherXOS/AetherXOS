#[cfg(not(feature = "linux_compat"))]
use super::{linux_errno, sys_yield};
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
use super::{with_user_read_bytes, with_user_write_bytes};

#[path = "poll_select/compat.rs"]
mod compat;
#[path = "poll_select/poll.rs"]
mod poll;
#[path = "poll_select/select.rs"]
mod select;

#[cfg(not(feature = "linux_compat"))]
use compat::linux_poll_fd_limit;

#[cfg(not(feature = "linux_compat"))]
pub(super) use poll::{sys_linux_poll, sys_linux_ppoll};
#[cfg(not(feature = "linux_compat"))]
pub(super) use select::{sys_linux_pselect6, sys_linux_select};
