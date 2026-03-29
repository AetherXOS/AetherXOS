use super::status;
use super::transport;
mod protocol;

pub use self::protocol::*;

pub fn protocol_socket_supported() -> bool {
    status::wayland_runtime_enabled()
}

pub fn shm_path_supported() -> bool {
    // Memfd/inotify/eventfd/timerfd/signalfd groundwork exists; full compositor ABI is pending.
    status::wayland_runtime_enabled()
}

pub fn has_wire_header_parser() -> bool {
    true
}

pub fn socket_preflight(display_env: &str) -> bool {
    transport::wayland_endpoint_from_env(display_env).is_some()
}

pub fn connect_sockaddr_precheck(sockaddr_un: &[u8]) -> bool {
    let Some(probe) = transport::probe_sockaddr_un_display_target(sockaddr_un) else {
        return false;
    };
    probe.is_display_socket && (probe.endpoint.path.contains("wayland-") || probe.endpoint.path.starts_with("@wayland-"))
}

pub fn validate_client_handshake_prefix(frame: &[u8]) -> bool {
    let Some(header) = parse_wire_header(frame) else {
        return false;
    };

    // Most clients start on wl_display (object id 1) with at least one payload word.
    header.object_id == 1 && header.byte_len >= 12 && is_complete_frame(frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn validate_wayland_handshake_accepts_display_frame() {
        let object_id = 1u32;
        let opcode = 1u16;
        let byte_len = 12u16;
        let word = ((byte_len as u32) << 16) | opcode as u32;
        let mut bytes = [0u8; 8];
        bytes[..4].copy_from_slice(&object_id.to_ne_bytes());
        bytes[4..].copy_from_slice(&word.to_ne_bytes());

        assert!(validate_client_handshake_prefix(&bytes));
    }

    #[test_case]
    fn validate_wayland_handshake_rejects_non_display_object() {
        let object_id = 5u32;
        let opcode = 1u16;
        let byte_len = 12u16;
        let word = ((byte_len as u32) << 16) | opcode as u32;
        let mut bytes = [0u8; 8];
        bytes[..4].copy_from_slice(&object_id.to_ne_bytes());
        bytes[4..].copy_from_slice(&word.to_ne_bytes());

        assert!(!validate_client_handshake_prefix(&bytes));
    }

    #[test_case]
    fn socket_preflight_accepts_runtime_display_name() {
        assert!(socket_preflight("wayland-0"));
    }

    #[test_case]
    fn connect_sockaddr_precheck_accepts_wayland_target() {
        let mut raw = [0u8; 48];
        raw[..2].copy_from_slice(&1u16.to_ne_bytes());
        let p = b"/run/user/1000/wayland-1\0";
        raw[2..2 + p.len()].copy_from_slice(p);
        assert!(connect_sockaddr_precheck(&raw));
    }
}
