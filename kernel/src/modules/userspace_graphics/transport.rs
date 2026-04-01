use alloc::format;
use alloc::string::String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnixDisplayEndpoint {
    pub path: String,
    pub abstract_namespace: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnixSocketConnectProbe {
    pub endpoint: UnixDisplayEndpoint,
    pub is_display_socket: bool,
}

const AF_UNIX: u16 = 1;

fn parse_display_number(s: &str) -> Option<u32> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u32>().ok()
}

pub fn wayland_endpoint_from_env(display: &str) -> Option<UnixDisplayEndpoint> {
    let value = display.trim();
    if value.is_empty() {
        return None;
    }

    if let Some(rest) = value.strip_prefix("@").or_else(|| value.strip_prefix("unix:@")) {
        if rest.is_empty() {
            return None;
        }
        return Some(UnixDisplayEndpoint {
            path: format!("@{}", rest),
            abstract_namespace: true,
        });
    }

    if value.contains('/') {
        return Some(UnixDisplayEndpoint {
            path: String::from(value),
            abstract_namespace: false,
        });
    }

    // Canonical runtime location used by most compositors.
    Some(UnixDisplayEndpoint {
        path: format!("/run/user/1000/{}", value),
        abstract_namespace: false,
    })
}

pub fn x11_endpoint_from_display(display: &str) -> Option<UnixDisplayEndpoint> {
    let value = display.trim();
    if value.is_empty() {
        return None;
    }

    let local = value
        .strip_prefix("unix/")
        .or_else(|| value.strip_prefix("localhost"))
        .unwrap_or(value);

    let after_colon = local.strip_prefix(':')?;
    let display_id_text = after_colon.split('.').next().unwrap_or("");
    let display_id = parse_display_number(display_id_text)?;

    Some(UnixDisplayEndpoint {
        path: format!("/tmp/.X11-unix/X{}", display_id),
        abstract_namespace: false,
    })
}

fn is_display_socket_path(path: &str) -> bool {
    path.contains("/wayland-") || path.starts_with("/tmp/.X11-unix/X") || path.starts_with("@wayland-")
}

pub fn parse_sockaddr_un(bytes: &[u8]) -> Option<UnixDisplayEndpoint> {
    if bytes.len() < 3 {
        return None;
    }

    let family = u16::from_ne_bytes([bytes[0], bytes[1]]);
    if family != AF_UNIX {
        return None;
    }

    let path_bytes = &bytes[2..];
    if path_bytes.is_empty() {
        return None;
    }

    if path_bytes[0] == 0 {
        let end = path_bytes[1..]
            .iter()
            .position(|b| *b == 0)
            .map(|i| i + 1)
            .unwrap_or(path_bytes.len());
        if end <= 1 {
            return None;
        }
        let name = core::str::from_utf8(&path_bytes[1..end]).ok()?;
        return Some(UnixDisplayEndpoint {
            path: format!("@{}", name),
            abstract_namespace: true,
        });
    }

    let end = path_bytes
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(path_bytes.len());
    if end == 0 {
        return None;
    }
    let path = core::str::from_utf8(&path_bytes[..end]).ok()?;
    Some(UnixDisplayEndpoint {
        path: String::from(path),
        abstract_namespace: false,
    })
}

pub fn probe_sockaddr_un_display_target(bytes: &[u8]) -> Option<UnixSocketConnectProbe> {
    let endpoint = parse_sockaddr_un(bytes)?;
    Some(UnixSocketConnectProbe {
        is_display_socket: is_display_socket_path(&endpoint.path),
        endpoint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn wayland_env_name_maps_to_runtime_socket_path() {
        let ep = wayland_endpoint_from_env("wayland-0").expect("endpoint");
        assert_eq!(ep.path, "/run/user/1000/wayland-0");
        assert!(!ep.abstract_namespace);
    }

    #[test_case]
    fn x11_display_number_maps_to_tmp_socket() {
        let ep = x11_endpoint_from_display(":1").expect("endpoint");
        assert_eq!(ep.path, "/tmp/.X11-unix/X1");
    }

    #[test_case]
    fn x11_invalid_display_is_rejected() {
        assert!(x11_endpoint_from_display("invalid").is_none());
    }

    #[test_case]
    fn parse_sockaddr_un_extracts_filesystem_path() {
        let mut buf = [0u8; 32];
        buf[..2].copy_from_slice(&AF_UNIX.to_ne_bytes());
        let p = b"/tmp/.X11-unix/X0\0";
        buf[2..2 + p.len()].copy_from_slice(p);
        let ep = parse_sockaddr_un(&buf).expect("sockaddr parse");
        assert_eq!(ep.path, "/tmp/.X11-unix/X0");
        assert!(!ep.abstract_namespace);
    }

    #[test_case]
    fn probe_detects_wayland_display_socket() {
        let mut buf = [0u8; 48];
        buf[..2].copy_from_slice(&AF_UNIX.to_ne_bytes());
        let p = b"/run/user/1000/wayland-0\0";
        buf[2..2 + p.len()].copy_from_slice(p);
        let probe = probe_sockaddr_un_display_target(&buf).expect("probe");
        assert!(probe.is_display_socket);
    }
}
