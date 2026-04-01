use super::status;
use super::transport;
mod protocol;

pub use self::protocol::*;

pub fn unix_display_socket_supported() -> bool {
    status::x11_runtime_enabled()
}

pub fn x11_core_protocol_supported() -> bool {
    // Core setup/request/reply packet validation path is enabled at runtime.
    status::x11_runtime_enabled()
}

pub fn x11_reply_event_semantics_supported() -> bool {
    status::x11_runtime_enabled() && has_reply_parser() && has_server_packet_parser()
}

pub fn has_setup_parser() -> bool {
    true
}

pub fn has_reply_parser() -> bool {
    true
}

pub fn has_server_packet_parser() -> bool {
    true
}

pub fn socket_preflight(display_env: &str) -> bool {
    transport::x11_endpoint_from_display(display_env).is_some()
}

pub fn connect_sockaddr_precheck(sockaddr_un: &[u8]) -> bool {
    let Some(probe) = transport::probe_sockaddr_un_display_target(sockaddr_un) else {
        return false;
    };
    probe.is_display_socket && probe.endpoint.path.starts_with("/tmp/.X11-unix/X")
}

pub fn validate_client_setup_request(bytes: &[u8]) -> bool {
    let Some(setup) = parse_setup_prefix(bytes) else {
        return false;
    };

    // Keep request validation strict but lightweight.
    setup.major_version == 11
        && setup.auth_proto_name_len <= 256
        && setup.auth_proto_data_len <= 4096
        && has_complete_setup_request(bytes)
}

pub fn validate_client_request_prefix(bytes: &[u8], setup: &X11SetupPrefix) -> bool {
    if !status::x11_runtime_enabled() {
        return false;
    }
    has_complete_request(bytes, setup.byte_order)
}

pub fn validate_server_reply_prefix(bytes: &[u8], setup: &X11SetupPrefix) -> bool {
    if !status::x11_runtime_enabled() {
        return false;
    }
    has_complete_server_packet(bytes, setup.byte_order)
}

pub fn validate_core_opcode_dispatch_prefix(bytes: &[u8], setup: &X11SetupPrefix) -> bool {
    if !status::x11_runtime_enabled() {
        return false;
    }

    let Some(prefix) = parse_request_prefix(bytes, setup.byte_order) else {
        return false;
    };

    // Early desktop bring-up targets CreateWindow(1) and MapWindow(8) first.
    matches!(prefix.opcode, 1 | 8) && has_complete_request(bytes, setup.byte_order)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn validate_x11_setup_accepts_reasonable_auth_lengths() {
        let bytes = [
            b'l', 0, // byte order + pad
            11, 0, // major
            0, 0, // minor
            18, 0, // auth name len
            16, 0, // auth data len
            0, 0, // pad
        ];
        assert!(validate_client_setup_request(&bytes));
    }

    #[test_case]
    fn validate_x11_setup_rejects_huge_auth_blob() {
        let bytes = [
            b'l', 0, // byte order + pad
            11, 0, // major
            0, 0, // minor
            1, 0, // auth name len
            0, 0x20, // auth data len = 8192 (le)
            0, 0, // pad
        ];
        assert!(!validate_client_setup_request(&bytes));
    }

    #[test_case]
    fn socket_preflight_accepts_local_display() {
        assert!(socket_preflight(":0"));
    }

    #[test_case]
    fn connect_sockaddr_precheck_accepts_x11_target() {
        let mut raw = [0u8; 40];
        raw[..2].copy_from_slice(&1u16.to_ne_bytes());
        let p = b"/tmp/.X11-unix/X0\0";
        raw[2..2 + p.len()].copy_from_slice(p);
        assert!(connect_sockaddr_precheck(&raw));
    }

    #[test_case]
    fn validate_client_request_prefix_accepts_minimal_core_request() {
        let setup_bytes = [
            b'l', 0, // byte order + pad
            11, 0, // major
            0, 0, // minor
            0, 0, // auth name len
            0, 0, // auth data len
            0, 0, // pad
        ];
        let setup = parse_setup_prefix(&setup_bytes).expect("x11 setup prefix");
        let req = [1u8, 0, 2, 0, 0, 0, 0, 0];
        assert!(validate_client_request_prefix(&req, &setup));
    }

    #[test_case]
    fn validate_server_reply_prefix_accepts_minimal_success_reply() {
        let setup_bytes = [
            b'l', 0, // byte order + pad
            11, 0, // major
            0, 0, // minor
            0, 0, // auth name len
            0, 0, // auth data len
            0, 0, // pad
        ];
        let setup = parse_setup_prefix(&setup_bytes).expect("x11 setup prefix");
        let mut reply = [0u8; 32];
        reply[0] = 1;
        reply[2..4].copy_from_slice(&1u16.to_le_bytes());
        assert!(validate_server_reply_prefix(&reply, &setup));
    }

    #[test_case]
    fn validate_server_reply_prefix_accepts_event_frame() {
        let setup_bytes = [
            b'l', 0, // byte order + pad
            11, 0, // major
            0, 0, // minor
            0, 0, // auth name len
            0, 0, // auth data len
            0, 0, // pad
        ];
        let setup = parse_setup_prefix(&setup_bytes).expect("x11 setup prefix");
        let mut event = [0u8; 32];
        event[0] = 2;
        event[2..4].copy_from_slice(&1u16.to_le_bytes());
        assert!(validate_server_reply_prefix(&event, &setup));
    }

    #[test_case]
    fn x11_reply_event_semantics_flag_is_true_when_runtime_enabled() {
        assert!(x11_reply_event_semantics_supported());
    }

    #[test_case]
    fn validate_core_opcode_dispatch_accepts_createwindow_request() {
        let setup_bytes = [
            b'l', 0, // byte order + pad
            11, 0, // major
            0, 0, // minor
            0, 0, // auth name len
            0, 0, // auth data len
            0, 0, // pad
        ];
        let setup = parse_setup_prefix(&setup_bytes).expect("x11 setup prefix");
        let req = [1u8, 0, 2, 0, 0, 0, 0, 0];
        assert!(validate_core_opcode_dispatch_prefix(&req, &setup));
    }

    #[test_case]
    fn validate_core_opcode_dispatch_rejects_non_target_opcode() {
        let setup_bytes = [
            b'l', 0, // byte order + pad
            11, 0, // major
            0, 0, // minor
            0, 0, // auth name len
            0, 0, // auth data len
            0, 0, // pad
        ];
        let setup = parse_setup_prefix(&setup_bytes).expect("x11 setup prefix");
        let req = [38u8, 0, 2, 0, 0, 0, 0, 0];
        assert!(!validate_core_opcode_dispatch_prefix(&req, &setup));
    }
}
