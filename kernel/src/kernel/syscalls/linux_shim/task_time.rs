use super::*;

#[path = "task_time/ids_ops.rs"]
mod ids_ops;
#[path = "task_time/robust_ops.rs"]
mod robust_ops;
#[path = "task_time/signal_sched_ops.rs"]
mod signal_sched_ops;
#[path = "task_time/time_ops.rs"]
mod time_ops;

#[cfg(not(feature = "linux_compat"))]
pub(crate) use ids_ops::{
    sys_linux_getpid, sys_linux_getppid, sys_linux_gettid, sys_linux_set_tid_address,
};
#[cfg(not(feature = "linux_compat"))]
pub(crate) use robust_ops::{sys_linux_get_robust_list, sys_linux_set_robust_list};
#[cfg(not(feature = "linux_compat"))]
pub(crate) use signal_sched_ops::{
    sys_linux_kill, sys_linux_sched_get_priority_max, sys_linux_sched_get_priority_min,
    sys_linux_tgkill,
};
#[cfg(not(feature = "linux_compat"))]
pub(crate) use time_ops::{sys_linux_clock_gettime, sys_linux_clock_nanosleep};
