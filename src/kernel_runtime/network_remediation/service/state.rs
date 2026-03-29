use crate::kernel_runtime::networking::{
    E1000_IO_ERROR_STREAK, E1000_REBIND_FAILURE_STREAK, VIRTIO_IO_ERROR_STREAK,
    VIRTIO_REBIND_FAILURE_STREAK,
};

pub(super) struct DriverIoHealthState {
    pub(super) io_error_streak: &'static core::sync::atomic::AtomicU64,
    pub(super) rebind_failure_streak: &'static core::sync::atomic::AtomicU64,
    pub(super) quarantine_reason: &'static str,
}

pub(super) struct DriverIoHealthContext {
    pub(super) driver_failed: bool,
    pub(super) io_error_streak: u64,
    pub(super) rebind_failure_streak: u64,
}

impl DriverIoHealthState {
    pub(super) fn record_io_error(&self) -> u64 {
        self.io_error_streak
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed)
            + 1
    }

    pub(super) fn rebind_failures(&self) -> u64 {
        self.rebind_failure_streak
            .load(core::sync::atomic::Ordering::Relaxed)
    }

    pub(super) fn record_rebind_failure(&self) -> u64 {
        self.rebind_failure_streak
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed)
            + 1
    }

    pub(super) fn clear_io_errors(&self) {
        self.io_error_streak
            .store(0, core::sync::atomic::Ordering::Relaxed);
    }

    pub(super) fn clear_rebind_failures(&self) {
        self.rebind_failure_streak
            .store(0, core::sync::atomic::Ordering::Relaxed);
    }

    pub(super) fn clear_all(&self) {
        self.clear_io_errors();
        self.clear_rebind_failures();
    }
}

pub(super) fn virtio_state() -> DriverIoHealthState {
    DriverIoHealthState {
        io_error_streak: &VIRTIO_IO_ERROR_STREAK,
        rebind_failure_streak: &VIRTIO_REBIND_FAILURE_STREAK,
        quarantine_reason: "virtio-rebind-failure-threshold",
    }
}

pub(super) fn e1000_state() -> DriverIoHealthState {
    DriverIoHealthState {
        io_error_streak: &E1000_IO_ERROR_STREAK,
        rebind_failure_streak: &E1000_REBIND_FAILURE_STREAK,
        quarantine_reason: "e1000-rebind-failure-threshold",
    }
}
