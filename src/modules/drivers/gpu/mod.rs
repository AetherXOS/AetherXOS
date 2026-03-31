use core::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

static GPU_STACK_INITIALIZED: AtomicBool = AtomicBool::new(false);
static GPU_BACKEND: AtomicU8 = AtomicU8::new(0);
static GPU_KMS_READY: AtomicBool = AtomicBool::new(false);
static GPU_INPUT_READY: AtomicBool = AtomicBool::new(false);
static GPU_HEARTBEAT_TICKS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GpuBackend {
    None = 0,
    Framebuffer = 1,
    VirtIoGpu = 2,
}

crate::impl_enum_u8_default_conversions!(GpuBackend { None, Framebuffer, VirtIoGpu }, default = None);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuStackState {
    Uninitialized,
    Initialized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuHealthTier {
    Healthy,
    Degraded,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuHealthAction {
    None,
    ReinitializeStack,
    PreferFramebufferFallback,
}

#[derive(Debug, Clone, Copy)]
pub struct GpuHealthThresholds {
    pub max_heartbeat_staleness_ticks: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct GpuHealthReport {
    pub initialized: bool,
    pub backend_supports_acceleration: bool,
    pub desktop_session_ready: bool,
    pub heartbeat_staleness_ticks: u64,
    pub heartbeat_stale_breach: bool,
    pub tier: GpuHealthTier,
}

#[derive(Debug, Clone, Copy)]
pub struct GpuStackSnapshot {
    pub state: GpuStackState,
    pub backend: GpuBackend,
    pub kms_ready: bool,
    pub input_ready: bool,
    pub desktop_session_ready: bool,
    pub heartbeat_ticks: u64,
}

pub fn init_gpu_stack() {
    GPU_STACK_INITIALIZED.store(true, Ordering::Relaxed);
}

pub fn set_gpu_backend(backend: GpuBackend) {
    GPU_BACKEND.store(backend.to_u8(), Ordering::Relaxed);
}

pub fn gpu_backend_supports_acceleration() -> bool {
    !matches!(
        GpuBackend::from_u8(GPU_BACKEND.load(Ordering::Relaxed)).unwrap_or(GpuBackend::None),
        GpuBackend::None
    )
}

pub fn configure_gpu_backend_for_desktop(
    backend: GpuBackend,
    kms_ready: bool,
    input_ready: bool,
) {
    init_gpu_stack();
    set_gpu_backend(backend);
    GPU_KMS_READY.store(kms_ready, Ordering::Relaxed);
    GPU_INPUT_READY.store(input_ready, Ordering::Relaxed);
}

pub fn mark_kms_ready() {
    GPU_KMS_READY.store(true, Ordering::Relaxed);
}

pub fn mark_input_ready() {
    GPU_INPUT_READY.store(true, Ordering::Relaxed);
}

pub fn is_desktop_session_ready() -> bool {
    GPU_STACK_INITIALIZED.load(Ordering::Relaxed)
    && gpu_backend_supports_acceleration()
        && GPU_KMS_READY.load(Ordering::Relaxed)
        && GPU_INPUT_READY.load(Ordering::Relaxed)
}

pub fn note_gpu_heartbeat(ticks: u64) {
    GPU_HEARTBEAT_TICKS.store(ticks, Ordering::Relaxed);
}

pub fn gpu_health_thresholds() -> GpuHealthThresholds {
    GpuHealthThresholds {
        max_heartbeat_staleness_ticks: 25_000,
    }
}

pub fn evaluate_gpu_health(now_ticks: u64) -> GpuHealthReport {
    let initialized = GPU_STACK_INITIALIZED.load(Ordering::Relaxed);
    let backend_supports_acceleration = gpu_backend_supports_acceleration();
    let desktop_session_ready = is_desktop_session_ready();
    let heartbeat = GPU_HEARTBEAT_TICKS.load(Ordering::Relaxed);
    let heartbeat_staleness_ticks = now_ticks.saturating_sub(heartbeat);
    let heartbeat_stale_breach = initialized
        && heartbeat > 0
        && heartbeat_staleness_ticks > gpu_health_thresholds().max_heartbeat_staleness_ticks;

    let tier = if !initialized || !backend_supports_acceleration {
        GpuHealthTier::Critical
    } else if !desktop_session_ready || heartbeat_stale_breach {
        GpuHealthTier::Degraded
    } else {
        GpuHealthTier::Healthy
    };

    GpuHealthReport {
        initialized,
        backend_supports_acceleration,
        desktop_session_ready,
        heartbeat_staleness_ticks,
        heartbeat_stale_breach,
        tier,
    }
}

pub fn recommended_gpu_health_action(report: GpuHealthReport) -> GpuHealthAction {
    if !report.initialized {
        return GpuHealthAction::ReinitializeStack;
    }
    if !report.backend_supports_acceleration {
        return GpuHealthAction::PreferFramebufferFallback;
    }
    if report.heartbeat_stale_breach {
        return GpuHealthAction::ReinitializeStack;
    }
    GpuHealthAction::None
}

pub fn current_gpu_health() -> GpuHealthReport {
    evaluate_gpu_health(crate::kernel::watchdog::global_tick())
}

pub fn gpu_stack_snapshot() -> GpuStackSnapshot {
    let state = if GPU_STACK_INITIALIZED.load(Ordering::Relaxed) {
        GpuStackState::Initialized
    } else {
        GpuStackState::Uninitialized
    };
    let backend =
        GpuBackend::from_u8(GPU_BACKEND.load(Ordering::Relaxed)).unwrap_or(GpuBackend::None);
    let kms_ready = GPU_KMS_READY.load(Ordering::Relaxed);
    let input_ready = GPU_INPUT_READY.load(Ordering::Relaxed);
    GpuStackSnapshot {
        state,
        backend,
        kms_ready,
        input_ready,
        desktop_session_ready: is_desktop_session_ready(),
        heartbeat_ticks: GPU_HEARTBEAT_TICKS.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
mod tests;
