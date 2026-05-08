//! Phase 7: System Service Integration
//!
//! Integrates system services (signals, networking, audit logging) with security policies
//! and kernel subsystems. Follows the same hook pattern as syscall_integration.rs.


/// Signal delivery hook - called when a signal is about to be sent to a process
///
/// # Arguments
/// - `pid`: Target process ID
/// - `signal`: Signal number (1-64)
/// - `sender_uid`: UID of sending process
///
/// # Returns
/// - `Ok(())` if signal delivery is allowed
/// - `Err(&str)` with message if denied by policy
#[cfg(feature = "posix_signal")]
pub fn on_signal_send(pid: usize, signal: i32, sender_uid: u32) -> Result<(), &'static str> {
    use crate::kernel_runtime::integration_utils;

    // Validate signal number
    if signal < 1 || signal > 64 {
        log::warn(&format!("Invalid signal {} to pid {}", signal, pid));
        return Err("invalid_signal");
    }

    // Log audit event
    #[cfg(feature = "audit_logging")]
    {
        let description = format!("Signal {} sent to pid {}", signal, pid);
        log::info(&description);
        integration_utils::audit_syscall_event(
            "signal_send",
            pid as u32,
            sender_uid,
            true,
            Some(&description),
        );
    }

    log::debug(&format!("Signal delivery allowed: sig={} pid={}", signal, pid));
    Ok(())
}

/// Signal receipt hook - called when a signal is delivered to a process
///
/// # Arguments
/// - `pid`: Recipient process ID
/// - `signal`: Signal number
/// - `handler_action`: Handler action description (e.g., "default", "ignore", "custom")
///
/// # Returns
/// - `Ok(())` if signal can be delivered
/// - `Err(&str)` if delivery should be blocked
#[cfg(feature = "posix_signal")]
pub fn on_signal_receive(pid: usize, signal: i32, handler_action: &str) -> Result<(), &'static str> {
    use crate::kernel_runtime::integration_utils;

    // Validate signal and action
    if signal < 1 || signal > 64 {
        return Err("invalid_signal");
    }

    #[cfg(feature = "audit_logging")]
    {
        let description = format!("Signal {} delivered to pid {} with action {}", signal, pid, handler_action);
        integration_utils::audit_syscall_event(
            "signal_receive",
            pid as u32,
            0,
            true,
            Some(&description),
        );
    }

    log::debug(&format!(
        "Signal delivery: sig={} pid={} action={}",
        signal, pid, handler_action
    ));
    Ok(())
}

/// Socket creation hook - called when a socket is created
///
/// # Arguments
/// - `domain`: Address family (AF_INET, AF_INET6, AF_UNIX, etc.)
/// - `socket_type`: Socket type (SOCK_STREAM, SOCK_DGRAM, etc.)
/// - `protocol`: Protocol number
/// - `uid`: UID of creating process
///
/// # Returns
/// - `Ok(())` if socket creation is allowed
/// - `Err(&str)` if denied by policy
#[cfg(feature = "posix_net")]
pub fn on_socket_create(domain: usize, socket_type: usize, protocol: usize, uid: u32) -> Result<(), &'static str> {
    use crate::kernel_runtime::integration_utils;

    // Validate socket domain
    const AF_INET: usize = 2;
    const AF_INET6: usize = 10;
    const AF_UNIX: usize = 1;

    match domain {
        AF_INET | AF_INET6 | AF_UNIX => {
            // Allowed domains
        }
        _ => {
            log::warn(&format!("Invalid socket domain {}", domain));
            return Err("invalid_domain");
        }
    }

    #[cfg(feature = "audit_logging")]
    {
        let description = format!(
            "Socket created: domain={} type={} protocol={}",
            domain, socket_type, protocol
        );
        integration_utils::audit_syscall_event("socket_create", 0, uid, true, Some(&description));
    }

    log::debug(&format!(
        "Socket creation allowed: domain={} type={} protocol={}",
        domain, socket_type, protocol
    ));
    Ok(())
}

/// Socket connection hook - called when connecting to a remote address
///
/// # Arguments
/// - `domain`: Address family
/// - `remote_addr`: Remote address (as string for logging)
/// - `remote_port`: Remote port
/// - `uid`: UID of connecting process
///
/// # Returns
/// - `Ok(())` if connection is allowed
/// - `Err(&str)` if denied by policy
#[cfg(feature = "posix_net")]
pub fn on_socket_connect(domain: usize, remote_addr: &str, remote_port: u16, uid: u32) -> Result<(), &'static str> {
    use crate::kernel_runtime::integration_utils;

    // Policy: deny connections to localhost on high ports (example)
    if remote_addr == "127.0.0.1" && remote_port > 32768 {
        log::warn(&format!("Blocked connection to {}:{}", remote_addr, remote_port));
        #[cfg(feature = "audit_logging")]
        {
            let description = format!("Connection denied to {}:{}", remote_addr, remote_port);
            integration_utils::audit_syscall_event("socket_connect", 0, uid, false, Some(&description));
        }
        return Err("access_denied");
    }

    #[cfg(feature = "audit_logging")]
    {
        let description = format!("Socket connected to {}:{}", remote_addr, remote_port);
        integration_utils::audit_syscall_event("socket_connect", 0, uid, true, Some(&description));
    }

    log::debug(&format!("Connection allowed to {}:{}", remote_addr, remote_port));
    Ok(())
}

/// Socket send hook - called before sending data
///
/// # Arguments
/// - `fd`: Socket file descriptor
/// - `data_len`: Amount of data being sent
/// - `uid`: UID of sending process
///
/// # Returns
/// - `Ok(bytes_allowed)` with amount allowed (may be capped by policy)
/// - `Err(&str)` if send should be blocked
#[cfg(feature = "posix_net")]
pub fn on_socket_send(fd: usize, data_len: usize, uid: u32) -> Result<usize, &'static str> {
    use crate::kernel_runtime::integration_utils;

    // Policy: rate limit sends to 1MB per syscall
    const MAX_SEND_SIZE: usize = 1024 * 1024; // 1MB
    let allowed = core::cmp::min(data_len, MAX_SEND_SIZE);

    if data_len > MAX_SEND_SIZE {
        log::warn(&format!("Send capped: {} -> {} bytes", data_len, allowed));
    }

    #[cfg(feature = "audit_logging")]
    {
        let description = format!("Socket send: fd={} len={}", fd, allowed);
        integration_utils::audit_syscall_event("socket_send", 0, uid, true, Some(&description));
    }

    Ok(allowed)
}

/// Socket receive hook - called before receiving data
///
/// # Arguments
/// - `fd`: Socket file descriptor
/// - `buffer_len`: Size of receive buffer
/// - `uid`: UID of receiving process
///
/// # Returns
/// - `Ok(bytes_allowed)` with amount allowed
/// - `Err(&str)` if receive should be blocked
#[cfg(feature = "posix_net")]
pub fn on_socket_receive(fd: usize, buffer_len: usize, uid: u32) -> Result<usize, &'static str> {
    use crate::kernel_runtime::integration_utils;

    // Policy: rate limit receives to 1MB per syscall
    const MAX_RECV_SIZE: usize = 1024 * 1024; // 1MB
    let allowed = core::cmp::min(buffer_len, MAX_RECV_SIZE);

    #[cfg(feature = "audit_logging")]
    {
        let description = format!("Socket receive: fd={} len={}", fd, allowed);
        integration_utils::audit_syscall_event("socket_receive", 0, uid, true, Some(&description));
    }

    Ok(allowed)
}

/// Network bind hook - called when binding to a local address
///
/// # Arguments
/// - `port`: Local port to bind
/// - `uid`: UID of binding process
///
/// # Returns
/// - `Ok(())` if bind is allowed
/// - `Err(&str)` if denied by policy
#[cfg(feature = "posix_net")]
pub fn on_socket_bind(port: u16, uid: u32) -> Result<(), &'static str> {
    use crate::kernel_runtime::integration_utils;

    // Policy: only root (uid 0) can bind to ports < 1024
    if port < 1024 && uid != 0 {
        log::warn(&format!("Non-root uid {} tried to bind port {}", uid, port));
        #[cfg(feature = "audit_logging")]
        {
            let description = format!("Bind denied to port {}", port);
            integration_utils::audit_syscall_event("socket_bind", 0, uid, false, Some(&description));
        }
        return Err("permission_denied");
    }

    #[cfg(feature = "audit_logging")]
    {
        let description = format!("Socket bound to port {}", port);
        integration_utils::audit_syscall_event("socket_bind", 0, uid, true, Some(&description));
    }

    log::debug(&format!("Bind allowed: port={}", port));
    Ok(())
}

/// Listen hook - called when listening on a socket
///
/// # Arguments
/// - `fd`: File descriptor
/// - `backlog`: Listen backlog size
/// - `uid`: UID of listening process
///
/// # Returns
/// - `Ok(())` if listen is allowed
/// - `Err(&str)` if denied
#[cfg(feature = "posix_net")]
pub fn on_socket_listen(fd: usize, backlog: usize, uid: u32) -> Result<(), &'static str> {
    use crate::kernel_runtime::integration_utils;

    // Validate backlog
    if backlog == 0 {
        return Err("invalid_backlog");
    }

    // Policy: cap backlog to 4096
    const MAX_BACKLOG: usize = 4096;
    let effective_backlog = core::cmp::min(backlog, MAX_BACKLOG);

    #[cfg(feature = "audit_logging")]
    {
        let description = format!("Socket listen: fd={} backlog={}", fd, effective_backlog);
        integration_utils::audit_syscall_event("socket_listen", 0, uid, true, Some(&description));
    }

    log::debug(&format!("Listen allowed: fd={} backlog={}", fd, effective_backlog));
    Ok(())
}

/// Accept hook - called when accepting an incoming connection
///
/// # Arguments
/// - `fd`: Listening socket file descriptor
/// - `uid`: UID of accepting process
/// - `remote_addr`: Remote address (for logging)
///
/// # Returns
/// - `Ok(())` if accept is allowed
/// - `Err(&str)` if denied by policy
#[cfg(feature = "posix_net")]
pub fn on_socket_accept(fd: usize, uid: u32, remote_addr: &str) -> Result<(), &'static str> {
    use crate::kernel_runtime::integration_utils;

    #[cfg(feature = "audit_logging")]
    {
        let description = format!("Accept connection from {}", remote_addr);
        integration_utils::audit_syscall_event("socket_accept", 0, uid, true, Some(&description));
    }

    log::debug(&format!("Accept allowed: fd={} from={}", fd, remote_addr));
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "posix_signal")]
    fn test_signal_send_valid() {
        let result = on_signal_send(1234, 9, 1000); // SIGKILL to pid 1234
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "posix_signal")]
    fn test_signal_send_invalid_signal() {
        let result = on_signal_send(1234, 100, 1000); // Invalid signal
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "posix_signal")]
    fn test_signal_receive_valid() {
        let result = on_signal_receive(1234, 15, "custom"); // SIGTERM with custom handler
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "posix_net")]
    fn test_socket_create_inet() {
        const AF_INET: usize = 2;
        const SOCK_STREAM: usize = 1;
        let result = on_socket_create(AF_INET, SOCK_STREAM, 6, 1000);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "posix_net")]
    fn test_socket_create_unix() {
        const AF_UNIX: usize = 1;
        const SOCK_STREAM: usize = 1;
        let result = on_socket_create(AF_UNIX, SOCK_STREAM, 0, 1000);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "posix_net")]
    fn test_socket_create_invalid_domain() {
        let result = on_socket_create(999, 1, 0, 1000); // Invalid domain
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "posix_net")]
    fn test_socket_connect_allowed() {
        let result = on_socket_connect(2, "192.168.1.1", 80, 1000);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "posix_net")]
    fn test_socket_connect_denied() {
        let result = on_socket_connect(2, "127.0.0.1", 32769, 1000); // Blocked by policy
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "posix_net")]
    fn test_socket_send_rate_limit() {
        let result = on_socket_send(3, 2 * 1024 * 1024, 1000); // 2MB
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1024 * 1024); // Capped to 1MB
    }

    #[test]
    #[cfg(feature = "posix_net")]
    fn test_socket_bind_privileged_port() {
        // Root can bind to privileged port
        let result = on_socket_bind(80, 0);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "posix_net")]
    fn test_socket_bind_nonroot_denied() {
        // Non-root cannot bind to privileged port
        let result = on_socket_bind(80, 1000);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "posix_net")]
    fn test_socket_bind_unprivileged() {
        // Non-root can bind to unprivileged port
        let result = on_socket_bind(8080, 1000);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "posix_net")]
    fn test_socket_listen_valid() {
        let result = on_socket_listen(5, 128, 1000);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "posix_net")]
    fn test_socket_listen_zero_backlog() {
        let result = on_socket_listen(5, 0, 1000);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "posix_net")]
    fn test_socket_accept_valid() {
        let result = on_socket_accept(5, 1000, "192.168.1.100");
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "posix_net")]
    fn test_socket_receive_rate_limit() {
        let result = on_socket_receive(3, 2 * 1024 * 1024, 1000); // 2MB buffer
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1024 * 1024); // Capped to 1MB
    }
}
