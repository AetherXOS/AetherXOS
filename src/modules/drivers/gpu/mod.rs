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

pub fn mark_kms_ready() {
    GPU_KMS_READY.store(true, Ordering::Relaxed);
}

pub fn mark_input_ready() {
    GPU_INPUT_READY.store(true, Ordering::Relaxed);
}

pub fn is_desktop_session_ready() -> bool {
    GPU_STACK_INITIALIZED.load(Ordering::Relaxed)
        && GPU_KMS_READY.load(Ordering::Relaxed)
        && GPU_INPUT_READY.load(Ordering::Relaxed)
}

pub fn note_gpu_heartbeat(ticks: u64) {
    GPU_HEARTBEAT_TICKS.store(ticks, Ordering::Relaxed);
}

pub fn gpu_stack_snapshot() -> GpuStackSnapshot {
    let state = if GPU_STACK_INITIALIZED.load(Ordering::Relaxed) {
        GpuStackState::Initialized
    } else {
        GpuStackState::Uninitialized
    };
    let backend = GpuBackend::from_u8(GPU_BACKEND.load(Ordering::Relaxed));
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
