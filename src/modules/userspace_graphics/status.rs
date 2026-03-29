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
    // Keep scoring conservative but tie progress to concrete protocol pieces.
    let wayland = {
        #[cfg(feature = "linux_userspace_wayland")]
        {
            if wayland_runtime_enabled() {
                let mut score = 25u8;
                if crate::modules::userspace_graphics::wayland::protocol_socket_supported() {
                    score = score.saturating_add(10);
                }
                if crate::modules::userspace_graphics::wayland::shm_path_supported() {
                    score = score.saturating_add(5);
                }
                if crate::modules::userspace_graphics::wayland::has_wire_header_parser() {
                    score = score.saturating_add(5);
                }
                score.min(100)
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
                let mut score = 22u8;
                if crate::modules::userspace_graphics::x11::unix_display_socket_supported() {
                    score = score.saturating_add(8);
                }
                if crate::modules::userspace_graphics::x11::has_setup_parser() {
                    score = score.saturating_add(6);
                }
                if crate::modules::userspace_graphics::x11::has_reply_parser() {
                    score = score.saturating_add(6);
                }
                if crate::modules::userspace_graphics::x11::has_server_packet_parser() {
                    score = score.saturating_add(4);
                }
                if crate::modules::userspace_graphics::x11::x11_core_protocol_supported() {
                    score = score.saturating_add(10);
                }
                score.min(100)
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
