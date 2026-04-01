use core::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};

static OPENGL_CONTEXT_PATH_READY: AtomicBool = AtomicBool::new(false);
static OPENGL_VERSION_MAJOR: AtomicU16 = AtomicU16::new(0);
static OPENGL_VERSION_MINOR: AtomicU16 = AtomicU16::new(0);
static OPENGL_EXTENSION_MASK: AtomicU32 = AtomicU32::new(0);

pub const OPENGL_EXT_FBO: u32 = 1 << 0;
pub const OPENGL_EXT_VBO: u32 = 1 << 1;
pub const OPENGL_EXT_SHADER_OBJECTS: u32 = 1 << 2;
pub const OPENGL_EXT_TEXTURE_STORAGE: u32 = 1 << 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenGlRuntimeSnapshot {
    pub context_ready: bool,
    pub version_major: u16,
    pub version_minor: u16,
    pub extension_mask: u32,
    pub driver_accel_ready: bool,
}

pub fn mark_opengl_context_path_ready() {
    OPENGL_CONTEXT_PATH_READY.store(true, Ordering::Relaxed);
}

pub fn register_opengl_runtime(version_major: u16, version_minor: u16, extension_mask: u32) {
    OPENGL_VERSION_MAJOR.store(version_major, Ordering::Relaxed);
    OPENGL_VERSION_MINOR.store(version_minor, Ordering::Relaxed);
    OPENGL_EXTENSION_MASK.store(extension_mask, Ordering::Relaxed);
}

pub fn is_opengl_context_path_ready() -> bool {
    OPENGL_CONTEXT_PATH_READY.load(Ordering::Relaxed)
}

fn has_required_extensions(mask: u32) -> bool {
    let required = OPENGL_EXT_FBO
        | OPENGL_EXT_VBO
        | OPENGL_EXT_SHADER_OBJECTS
        | OPENGL_EXT_TEXTURE_STORAGE;
    (mask & required) == required
}

fn version_meets_minimum(major: u16, minor: u16) -> bool {
    major > 3 || (major == 3 && minor >= 3)
}

fn driver_accel_ready() -> bool {
    #[cfg(feature = "drivers")]
    {
        let snapshot = crate::modules::drivers::gpu::gpu_stack_snapshot();
        snapshot.desktop_session_ready
            && !matches!(snapshot.backend, crate::modules::drivers::gpu::GpuBackend::None)
    }
    #[cfg(not(feature = "drivers"))]
    {
        true
    }
}

pub fn opengl_runtime_snapshot() -> OpenGlRuntimeSnapshot {
    OpenGlRuntimeSnapshot {
        context_ready: is_opengl_context_path_ready(),
        version_major: OPENGL_VERSION_MAJOR.load(Ordering::Relaxed),
        version_minor: OPENGL_VERSION_MINOR.load(Ordering::Relaxed),
        extension_mask: OPENGL_EXTENSION_MASK.load(Ordering::Relaxed),
        driver_accel_ready: driver_accel_ready(),
    }
}

pub fn opengl_runtime_contract_supported() -> bool {
    let snapshot = opengl_runtime_snapshot();
    snapshot.context_ready
        && snapshot.driver_accel_ready
        && version_meets_minimum(snapshot.version_major, snapshot.version_minor)
        && has_required_extensions(snapshot.extension_mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn opengl_contract_requires_context_and_required_extensions() {
        register_opengl_runtime(3, 3, OPENGL_EXT_FBO | OPENGL_EXT_VBO);
        assert!(!opengl_runtime_contract_supported());

        mark_opengl_context_path_ready();
        register_opengl_runtime(
            3,
            3,
            OPENGL_EXT_FBO
                | OPENGL_EXT_VBO
                | OPENGL_EXT_SHADER_OBJECTS
                | OPENGL_EXT_TEXTURE_STORAGE,
        );
        assert!(opengl_runtime_contract_supported());
    }
}
