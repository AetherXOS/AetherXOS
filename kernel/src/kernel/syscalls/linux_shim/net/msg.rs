#[path = "msg/compat.rs"]
mod compat;
#[path = "msg/message_ops.rs"]
mod message_ops;
#[path = "msg/message_support.rs"]
mod message_support;

#[cfg(not(feature = "linux_compat"))]
pub(super) use message_ops::{sys_linux_ioctl, sys_linux_recvmsg, sys_linux_sendmsg};
