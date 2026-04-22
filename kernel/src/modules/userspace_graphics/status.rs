use core::sync::atomic::{AtomicBool, Ordering};

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
