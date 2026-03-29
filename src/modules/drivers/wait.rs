#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverWaitFallbackKind {
    Fail,
    Retry,
}

#[derive(Debug, Clone, Copy)]
pub struct DriverWaitDescriptor {
    pub driver: &'static str,
    pub operation: &'static str,
    pub max_spins: usize,
    pub fallback: DriverWaitFallbackKind,
    pub timeout_events: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct DriverWaitPolicySnapshot {
    pub nvme_disable_ready: DriverWaitDescriptor,
    pub nvme_controller_ready: DriverWaitDescriptor,
    pub nvme_admin: DriverWaitDescriptor,
    pub nvme_io: DriverWaitDescriptor,
    pub ahci_read: DriverWaitDescriptor,
    pub ahci_write: DriverWaitDescriptor,
    pub e1000_reset: DriverWaitDescriptor,
}

pub fn snapshot() -> DriverWaitPolicySnapshot {
    let nvme = crate::modules::drivers::nvme::wait_stats();
    let ahci = crate::modules::drivers::ahci::wait_stats();
    let e1000 = crate::modules::drivers::e1000::wait_stats();

    DriverWaitPolicySnapshot {
        nvme_disable_ready: DriverWaitDescriptor {
            driver: "nvme",
            operation: "disable_ready_wait",
            max_spins: nvme.disable_ready_timeout_spins,
            fallback: DriverWaitFallbackKind::Fail,
            timeout_events: nvme.disable_ready_timeouts,
        },
        nvme_controller_ready: DriverWaitDescriptor {
            driver: "nvme",
            operation: "controller_ready_wait",
            max_spins: nvme.controller_ready_timeout_spins,
            fallback: DriverWaitFallbackKind::Fail,
            timeout_events: nvme.controller_ready_timeouts,
        },
        nvme_admin: DriverWaitDescriptor {
            driver: "nvme",
            operation: "admin_cq_wait",
            max_spins: nvme.admin_timeout_spins,
            fallback: DriverWaitFallbackKind::Fail,
            timeout_events: nvme.admin_timeouts,
        },
        nvme_io: DriverWaitDescriptor {
            driver: "nvme",
            operation: "io_cq_wait",
            max_spins: nvme.io_timeout_spins,
            fallback: DriverWaitFallbackKind::Fail,
            timeout_events: nvme.io_timeouts,
        },
        ahci_read: DriverWaitDescriptor {
            driver: "ahci",
            operation: "read_completion_wait",
            max_spins: ahci.io_timeout_spins,
            fallback: DriverWaitFallbackKind::Fail,
            timeout_events: ahci.read_timeouts,
        },
        ahci_write: DriverWaitDescriptor {
            driver: "ahci",
            operation: "write_completion_wait",
            max_spins: ahci.io_timeout_spins,
            fallback: DriverWaitFallbackKind::Fail,
            timeout_events: ahci.write_timeouts,
        },
        e1000_reset: DriverWaitDescriptor {
            driver: "e1000",
            operation: "reset_wait",
            max_spins: e1000.reset_timeout_spins,
            fallback: DriverWaitFallbackKind::Retry,
            timeout_events: e1000.reset_timeouts,
        },
    }
}
