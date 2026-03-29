use super::status;
use super::transport;
mod protocol;

pub use self::protocol::*;

pub fn protocol_socket_supported() -> bool {
    status::wayland_runtime_enabled()
}

pub fn shm_path_supported() -> bool {
    // Memfd/inotify/eventfd/timerfd/signalfd substrate is available for userspace protocol flow.
    status::wayland_runtime_enabled()
}

pub fn wayland_protocol_semantics_supported() -> bool {
    status::wayland_runtime_enabled()
        && has_wire_header_parser()
        && protocol_socket_supported()
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

pub fn validate_registry_advertisement_path(frame: &[u8]) -> bool {
    let Some(header) = parse_wire_header(frame) else {
        return false;
    };

    // wl_display.get_registry is typically the first control message on object id 1.
    header.object_id == 1 && header.opcode == 1 && header.byte_len >= 12 && is_complete_frame(frame)
}

pub fn validate_registry_bind_prefix(frame: &[u8]) -> bool {
    let Some(header) = parse_wire_header(frame) else {
        return false;
    };

    // wl_registry.bind is opcode 0 on the advertised registry object.
    header.object_id > 1 && header.opcode == 0 && header.byte_len >= 16 && is_complete_frame(frame)
}

pub fn validate_surface_commit_prefix(frame: &[u8]) -> bool {
    let Some(header) = parse_wire_header(frame) else {
        return false;
    };

    // wl_surface.commit is opcode 6 on non-display objects and has an 8-byte frame.
    header.object_id > 1 && header.opcode == 6 && header.byte_len == 8 && is_complete_frame(frame)
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

    #[test_case]
    fn validate_registry_advertisement_accepts_get_registry_message() {
        let object_id = 1u32;
        let opcode = 1u16;
        let byte_len = 12u16;
        let word = ((byte_len as u32) << 16) | opcode as u32;
        let mut bytes = [0u8; 12];
        bytes[..4].copy_from_slice(&object_id.to_ne_bytes());
        bytes[4..8].copy_from_slice(&word.to_ne_bytes());
        bytes[8..12].copy_from_slice(&2u32.to_ne_bytes());

        assert!(validate_registry_advertisement_path(&bytes));
    }

    #[test_case]
    fn validate_registry_bind_accepts_registry_bind_message() {
        let object_id = 2u32;
        let opcode = 0u16;
        let byte_len = 16u16;
        let word = ((byte_len as u32) << 16) | opcode as u32;
        let mut bytes = [0u8; 16];
        bytes[..4].copy_from_slice(&object_id.to_ne_bytes());
        bytes[4..8].copy_from_slice(&word.to_ne_bytes());
        bytes[8..12].copy_from_slice(&3u32.to_ne_bytes());
        bytes[12..16].copy_from_slice(&4u32.to_ne_bytes());

        assert!(validate_registry_bind_prefix(&bytes));
    }

    #[test_case]
    fn validate_surface_commit_accepts_minimal_commit_message() {
        let object_id = 4u32;
        let opcode = 6u16;
        let byte_len = 8u16;
        let word = ((byte_len as u32) << 16) | opcode as u32;
        let mut bytes = [0u8; 8];
        bytes[..4].copy_from_slice(&object_id.to_ne_bytes());
        bytes[4..8].copy_from_slice(&word.to_ne_bytes());

        assert!(validate_surface_commit_prefix(&bytes));
    }
}
