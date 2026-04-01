#[derive(Debug, Clone, Copy)]
pub(super) struct BackendSupportSnapshot {
    pub execution_backend: &'static str,
    pub irq_backend: &'static str,
    pub memory_backend: &'static str,
    pub launch_path: &'static str,
}

#[inline(always)]
pub(super) fn current_backend_support(
    backend: &'static str,
    hardware_accel: bool,
    gic: crate::hal::aarch64::gic::GicStats,
    memory_isolation_ready: bool,
) -> BackendSupportSnapshot {
    let execution_backend = match backend {
        "el2" => "el2",
        _ => "none",
    };
    let irq_backend = if gic.version >= 3 {
        "gicv3"
    } else if gic.initialized {
        "gicv2"
    } else {
        "none"
    };
    let memory_backend = if memory_isolation_ready {
        "stage2"
    } else if hardware_accel {
        "stage1"
    } else {
        "none"
    };
    let launch_path = match backend {
        "el2" if hardware_accel => "eret-el2",
        _ => "none",
    };

    BackendSupportSnapshot {
        execution_backend,
        irq_backend,
        memory_backend,
        launch_path,
    }
}
