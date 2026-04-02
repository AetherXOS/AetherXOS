pub(crate) mod common;
pub mod logind;
pub mod pipe;

#[cfg(feature = "ipc_binder")]
pub mod binder;
#[cfg(feature = "ipc_dbus")]
pub mod dbus;
#[cfg(feature = "ipc_futex")]
pub mod futex;
#[cfg(feature = "ipc_message_passing")]
pub mod message_passing;
#[cfg(feature = "ipc_ring_buffer")]
pub mod ring_buffer;
#[cfg(feature = "ipc_sysv_sem")]
pub mod semaphores;
#[cfg(feature = "ipc_shared_memory")]
pub mod shared_memory;
#[cfg(feature = "ipc_signal_only")]
pub mod signal_only;
#[cfg(feature = "ipc_sysv_msg")]
pub mod sysv_msg;
#[cfg(feature = "ipc_unix_domain")]
pub mod unix_socket;
#[cfg(feature = "ipc_zero_copy")]
pub mod zero_copy;

#[cfg(feature = "ipc_binder")]
pub use binder::{
    binder_acquire, binder_create, binder_release, binder_stats, binder_transact, BinderStats,
};
#[cfg(feature = "ipc_dbus")]
pub use dbus::{
    dbus_consume, dbus_publish, dbus_stats, dbus_subscribe, dbus_take_stats,
    heartbeat_session_service,
    list_session_services, mark_session_service_degraded, mark_session_service_ready,
    register_session_service, DbusStats, SessionServiceSnapshot, SessionServiceState,
};
#[cfg(feature = "ipc_futex")]
pub use futex::Futex;
#[cfg(feature = "ipc_message_passing")]
pub use message_passing::MessagePassing;
pub use pipe::{PipeEnd, PipeId, PipeRegistry, PipeStats};
pub use logind::{mark_session_active, register_session, session_snapshot};
#[cfg(feature = "ipc_ring_buffer")]
pub use ring_buffer::RingBuffer;
#[cfg(feature = "ipc_sysv_sem")]
pub use semaphores::{semctl, semget, semop, SemId, SemKey};
#[cfg(feature = "ipc_shared_memory")]
pub use shared_memory::{shm_get, shm_rmid, ShmId, ShmKey};
#[cfg(feature = "ipc_signal_only")]
pub use signal_only::SignalOnly;
#[cfg(feature = "ipc_sysv_msg")]
pub use sysv_msg::{msgget, msgrcv, msgsnd, MsgId, MsgKey};
#[cfg(feature = "ipc_unix_domain")]
pub use unix_socket::{unix_bind, unix_connect, UnixSocket};
#[cfg(feature = "ipc_zero_copy")]
pub use zero_copy::ZeroCopy;

pub mod selector {
    use super::*;

    #[cfg(all(feature = "ipc_zero_copy", param_ipc = "ZeroCopy"))]
    pub type ActiveIpc = ZeroCopy;

    #[cfg(all(feature = "ipc_message_passing", param_ipc = "MessagePassing"))]
    pub type ActiveIpc = MessagePassing;

    #[cfg(all(feature = "ipc_signal_only", param_ipc = "SignalOnly"))]
    pub type ActiveIpc = SignalOnly;

    #[cfg(all(feature = "ipc_ring_buffer", param_ipc = "Pipes"))]
    pub type ActiveIpc = RingBuffer;

    #[cfg(all(feature = "ipc_ring_buffer", param_ipc = "RingBuffer"))]
    pub type ActiveIpc = RingBuffer;

    #[cfg(all(feature = "ipc_futex", param_ipc = "Futex"))]
    pub type ActiveIpc = Futex;

    #[cfg(not(any(
        all(feature = "ipc_zero_copy", param_ipc = "ZeroCopy"),
        all(feature = "ipc_message_passing", param_ipc = "MessagePassing"),
        all(feature = "ipc_signal_only", param_ipc = "SignalOnly"),
        all(feature = "ipc_ring_buffer", param_ipc = "Pipes"),
        all(feature = "ipc_ring_buffer", param_ipc = "RingBuffer"),
        all(feature = "ipc_futex", param_ipc = "Futex")
    )))]
    #[cfg(feature = "ipc_zero_copy")]
    pub type ActiveIpc = ZeroCopy;

    #[cfg(not(any(
        all(feature = "ipc_zero_copy", param_ipc = "ZeroCopy"),
        all(feature = "ipc_message_passing", param_ipc = "MessagePassing"),
        all(feature = "ipc_signal_only", param_ipc = "SignalOnly"),
        all(feature = "ipc_ring_buffer", param_ipc = "Pipes"),
        all(feature = "ipc_ring_buffer", param_ipc = "RingBuffer"),
        all(feature = "ipc_futex", param_ipc = "Futex"),
        feature = "ipc_zero_copy"
    )))]
    #[cfg(feature = "ipc_message_passing")]
    pub type ActiveIpc = MessagePassing;

    #[cfg(not(any(
        all(feature = "ipc_zero_copy", param_ipc = "ZeroCopy"),
        all(feature = "ipc_message_passing", param_ipc = "MessagePassing"),
        all(feature = "ipc_signal_only", param_ipc = "SignalOnly"),
        all(feature = "ipc_ring_buffer", param_ipc = "Pipes"),
        all(feature = "ipc_ring_buffer", param_ipc = "RingBuffer"),
        all(feature = "ipc_futex", param_ipc = "Futex"),
        feature = "ipc_zero_copy",
        feature = "ipc_message_passing"
    )))]
    #[cfg(feature = "ipc_signal_only")]
    pub type ActiveIpc = SignalOnly;
}
