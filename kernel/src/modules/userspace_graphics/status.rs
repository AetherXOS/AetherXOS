use core::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy)]
pub struct UserspaceGraphicsStackSnapshot {
    pub framebuffer_ready: bool,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub framebuffer_bpp: u8,
    pub gpu_backend: crate::modules::drivers::GpuBackend,
    pub gpu_desktop_ready: bool,
    pub opengl_ready: bool,
    pub opengl_version_major: u16,
    pub opengl_version_minor: u16,
    pub vulkan_ready: bool,
    pub vulkan_api_version: u32,
    pub wayland_percent: u8,
    pub x11_percent: u8,
    pub weighted_percent: u8,
}

static WAYLAND_RUNTIME_ENABLED: AtomicBool = AtomicBool::new(true);
static X11_RUNTIME_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn set_wayland_runtime_enabled(enabled: bool) {
    WAYLAND_RUNTIME_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn set_x11_runtime_enabled(enabled: bool) {
    X11_RUNTIME_ENABLED.store(enabled, Ordering::Relaxed);
}

#[inline(always)]
pub fn wayland_runtime_enabled() -> bool {
    WAYLAND_RUNTIME_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn x11_runtime_enabled() -> bool {
    X11_RUNTIME_ENABLED.load(Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy)]
pub struct UserspaceGraphicsReadiness {
    pub wayland_percent: u8,
    pub x11_percent: u8,
    pub weighted_percent: u8,
}

pub fn readiness_snapshot() -> UserspaceGraphicsReadiness {
    let wayland = {
        #[cfg(feature = "linux_userspace_wayland")]
        {
            if wayland_runtime_enabled() {
                let checks = [
                    crate::modules::userspace_graphics::wayland::protocol_socket_supported(),
                    crate::modules::userspace_graphics::wayland::shm_path_supported(),
                    crate::modules::userspace_graphics::wayland::has_wire_header_parser(),
                    crate::modules::userspace_graphics::wayland::wayland_protocol_semantics_supported(),
                ];
                let supported = checks.into_iter().filter(|check| *check).count() as u8;
                ((supported as u16 * 100) / 4) as u8
            } else {
                0
            }
        }
        #[cfg(not(feature = "linux_userspace_wayland"))]
        {
            0
        }
    };

    let x11 = {
        #[cfg(feature = "linux_userspace_x11")]
        {
            if x11_runtime_enabled() {
                let checks = [
                    crate::modules::userspace_graphics::x11::unix_display_socket_supported(),
                    crate::modules::userspace_graphics::x11::has_setup_parser(),
                    crate::modules::userspace_graphics::x11::has_reply_parser(),
                    crate::modules::userspace_graphics::x11::has_server_packet_parser(),
                    crate::modules::userspace_graphics::x11::x11_core_protocol_supported(),
                    crate::modules::userspace_graphics::x11::x11_reply_event_semantics_supported(),
                ];
                let supported = checks.into_iter().filter(|check| *check).count() as u8;
                ((supported as u16 * 100) / 6) as u8
            } else {
                0
            }
        }
        #[cfg(not(feature = "linux_userspace_x11"))]
        {
            0
        }
    };

    let weighted = ((wayland as u16 + x11 as u16) / 2) as u8;
    UserspaceGraphicsReadiness {
        wayland_percent: wayland,
        x11_percent: x11,
        weighted_percent: weighted,
    }
}

pub fn log_readiness(context: &str) {
    let readiness = readiness_snapshot();
    crate::klog_info!(
        "[userspace_graphics] {} wayland={} x11={} weighted={}",
        context,
        readiness.wayland_percent,
        readiness.x11_percent,
        readiness.weighted_percent,
    );
    crate::kernel::debug_trace::record_optional(
        "userspace.graphics",
        context,
        Some(readiness.weighted_percent as u64),
        false,
    );
}

pub fn graphics_stack_snapshot() -> UserspaceGraphicsStackSnapshot {
    let framebuffer = crate::modules::drivers::framebuffer_stats();
    let gpu = crate::modules::drivers::gpu_stack_snapshot();
    let opengl = crate::modules::userspace_graphics::opengl::opengl_runtime_snapshot();
    let vulkan = crate::modules::userspace_graphics::vulkan::vulkan_runtime_snapshot();
    let readiness = readiness_snapshot();

    UserspaceGraphicsStackSnapshot {
        framebuffer_ready: framebuffer.initialized,
        framebuffer_width: framebuffer.width,
        framebuffer_height: framebuffer.height,
        framebuffer_bpp: framebuffer.bpp,
        gpu_backend: gpu.backend,
        gpu_desktop_ready: gpu.desktop_session_ready,
        opengl_ready: crate::modules::userspace_graphics::opengl::opengl_runtime_contract_supported(),
        opengl_version_major: opengl.version_major,
        opengl_version_minor: opengl.version_minor,
        vulkan_ready: crate::modules::userspace_graphics::vulkan::vulkan_runtime_contract_supported(),
        vulkan_api_version: vulkan.api_version,
        wayland_percent: readiness.wayland_percent,
        x11_percent: readiness.x11_percent,
        weighted_percent: readiness.weighted_percent,
    }
}

pub fn log_stack_summary(context: &str) {
    let snapshot = graphics_stack_snapshot();
    crate::klog_info!(
        "[graphics_stack] {} fb_ready={} fb={}x{}x{} gpu_backend={:?} gpu_desktop_ready={} opengl_ready={} opengl={}.{} vulkan_ready={} vulkan_api={:#x} wayland={} x11={} weighted={}",
        context,
        snapshot.framebuffer_ready,
        snapshot.framebuffer_width,
        snapshot.framebuffer_height,
        snapshot.framebuffer_bpp,
        snapshot.gpu_backend,
        snapshot.gpu_desktop_ready,
        snapshot.opengl_ready,
        snapshot.opengl_version_major,
        snapshot.opengl_version_minor,
        snapshot.vulkan_ready,
        snapshot.vulkan_api_version,
        snapshot.wayland_percent,
        snapshot.x11_percent,
        snapshot.weighted_percent,
    );
    crate::kernel::debug_trace::record_optional(
        "graphics.stack",
        context,
        Some(snapshot.weighted_percent as u64),
        false,
    );
}
