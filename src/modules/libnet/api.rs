#[derive(Debug, Clone, Copy, Default)]
pub struct LibNetApi;

impl LibNetApi {
    pub const fn new() -> Self {
        Self
    }

    pub fn capabilities(&self) -> crate::modules::libnet::LibNetCapabilities {
        crate::modules::libnet::capabilities()
    }

    pub fn bridge_snapshot(&self) -> crate::modules::libnet::LibNetBridgeSnapshot {
        crate::modules::libnet::bridge_snapshot()
    }

    pub fn profile_compatibility(&self) -> crate::modules::libnet::LibNetProfileCompatibility {
        crate::modules::libnet::profile_compatibility()
    }

    pub fn pump_once(&self) -> crate::modules::libnet::LibNetPumpReport {
        crate::modules::libnet::pump_once_with_report()
    }

    pub fn apply_adaptive_profile(&self) -> crate::modules::libnet::LibNetBridgeSnapshot {
        crate::modules::libnet::apply_adaptive_profile()
    }

    pub fn apply_poll_profile(
        &self,
        profile: crate::modules::libnet::PollProfile,
    ) -> crate::modules::libnet::LibNetBridgeSnapshot {
        crate::modules::libnet::apply_poll_profile(profile)
    }

    pub fn run_fast_path_once(
        &self,
        config: crate::modules::libnet::FastPathConfig,
    ) -> crate::modules::libnet::FastPathReport {
        crate::modules::libnet::run_fast_path_once(config)
    }

    pub fn run_fast_path_default(&self) -> crate::modules::libnet::FastPathReport {
        crate::modules::libnet::run_default_cycle()
    }

    pub fn run_fast_path_with_budget(
        &self,
        l2_pump_budget: usize,
    ) -> crate::modules::libnet::FastPathReport {
        let mut config = crate::modules::libnet::FastPathConfig::default();
        config.l2_pump_budget = l2_pump_budget;
        crate::modules::libnet::run_fast_path_once(config)
    }

    pub fn run_service_fast_path_cycle(
        &self,
        preset: crate::modules::libnet::ServicePreset,
    ) -> crate::modules::libnet::FastPathReport {
        crate::modules::libnet::run_service_fast_path_cycle(preset)
    }
}

#[cfg(feature = "network_transport")]
impl LibNetApi {
    pub fn posix_socket(
        &self,
        family: crate::modules::libnet::PosixAddressFamily,
        socket_type: crate::modules::libnet::PosixSocketType,
    ) -> Result<u32, &'static str> {
        crate::modules::libnet::posix_socket(family, socket_type)
    }

    pub fn posix_socket_errno(
        &self,
        family: crate::modules::libnet::PosixAddressFamily,
        socket_type: crate::modules::libnet::PosixSocketType,
    ) -> Result<u32, crate::modules::libnet::PosixErrno> {
        crate::modules::libnet::posix_socket_errno(family, socket_type)
    }

    pub fn posix_bind(
        &self,
        fd: u32,
        addr: crate::modules::libnet::PosixSocketAddrV4,
    ) -> Result<(), &'static str> {
        crate::modules::libnet::posix_bind(fd, addr)
    }

    pub fn posix_bind_errno(
        &self,
        fd: u32,
        addr: crate::modules::libnet::PosixSocketAddrV4,
    ) -> Result<(), crate::modules::libnet::PosixErrno> {
        crate::modules::libnet::posix_bind_errno(fd, addr)
    }

    pub fn posix_listen(&self, fd: u32, backlog: usize) -> Result<(), &'static str> {
        crate::modules::libnet::posix_listen(fd, backlog)
    }

    pub fn posix_connect(
        &self,
        fd: u32,
        addr: crate::modules::libnet::PosixSocketAddrV4,
    ) -> Result<(), &'static str> {
        crate::modules::libnet::posix_connect(fd, addr)
    }

    pub fn posix_accept(&self, fd: u32) -> Result<u32, &'static str> {
        crate::modules::libnet::posix_accept(fd)
    }

    pub fn posix_accept4(
        &self,
        fd: u32,
        flags: crate::modules::libnet::PosixFdFlags,
    ) -> Result<u32, &'static str> {
        crate::modules::libnet::posix_accept4(fd, flags)
    }

    pub fn posix_accept_errno(&self, fd: u32) -> Result<u32, crate::modules::libnet::PosixErrno> {
        crate::modules::libnet::posix_accept_errno(fd)
    }

    pub fn posix_send(&self, fd: u32, payload: &[u8]) -> Result<usize, &'static str> {
        crate::modules::libnet::posix_send(fd, payload)
    }

    pub fn posix_recv(&self, fd: u32) -> Result<alloc::vec::Vec<u8>, &'static str> {
        crate::modules::libnet::posix_recv(fd)
    }

    pub fn posix_sendto(
        &self,
        fd: u32,
        addr: crate::modules::libnet::PosixSocketAddrV4,
        payload: &[u8],
    ) -> Result<usize, &'static str> {
        crate::modules::libnet::posix_sendto(fd, addr, payload)
    }

    pub fn posix_recvfrom(
        &self,
        fd: u32,
    ) -> Result<crate::modules::libnet::PosixRecvFrom, &'static str> {
        crate::modules::libnet::posix_recvfrom(fd)
    }

    pub fn posix_poll(
        &self,
        fds: &mut [crate::modules::libnet::PosixPollFd],
        retries: usize,
    ) -> Result<usize, &'static str> {
        crate::modules::libnet::posix_poll(fds, retries)
    }

    pub fn posix_poll_errno(
        &self,
        fds: &mut [crate::modules::libnet::PosixPollFd],
        retries: usize,
    ) -> Result<usize, crate::modules::libnet::PosixErrno> {
        crate::modules::libnet::posix_poll_errno(fds, retries)
    }

    pub fn posix_select(
        &self,
        read_fds: &[u32],
        write_fds: &[u32],
        except_fds: &[u32],
        retries: usize,
    ) -> Result<crate::modules::libnet::PosixSelectResult, &'static str> {
        crate::modules::libnet::posix_select(read_fds, write_fds, except_fds, retries)
    }

    pub fn posix_select_errno(
        &self,
        read_fds: &[u32],
        write_fds: &[u32],
        except_fds: &[u32],
        retries: usize,
    ) -> Result<crate::modules::libnet::PosixSelectResult, crate::modules::libnet::PosixErrno> {
        crate::modules::libnet::posix_select_errno(read_fds, write_fds, except_fds, retries)
    }

    pub fn posix_getsockname(
        &self,
        fd: u32,
    ) -> Result<crate::modules::libnet::PosixSocketAddrV4, &'static str> {
        crate::modules::libnet::posix_getsockname(fd)
    }

    pub fn posix_getpeername(
        &self,
        fd: u32,
    ) -> Result<crate::modules::libnet::PosixSocketAddrV4, &'static str> {
        crate::modules::libnet::posix_getpeername(fd)
    }

    pub fn posix_set_nonblocking(&self, fd: u32, enabled: bool) -> Result<(), &'static str> {
        crate::modules::libnet::posix_set_nonblocking(fd, enabled)
    }

    pub fn posix_set_socket_option(
        &self,
        fd: u32,
        option: crate::modules::libnet::PosixSocketOption,
        enabled: bool,
    ) -> Result<(), &'static str> {
        crate::modules::libnet::posix_set_socket_option(fd, option, enabled)
    }

    pub fn posix_socket_options(
        &self,
        fd: u32,
    ) -> Result<crate::modules::libnet::PosixSocketOptions, &'static str> {
        crate::modules::libnet::posix_socket_options(fd)
    }

    pub fn posix_setsockopt(
        &self,
        fd: u32,
        option: crate::modules::libnet::PosixSockOpt,
        value: crate::modules::libnet::PosixSockOptVal,
    ) -> Result<(), &'static str> {
        crate::modules::libnet::posix_setsockopt(fd, option, value)
    }

    pub fn posix_getsockopt(
        &self,
        fd: u32,
        option: crate::modules::libnet::PosixSockOpt,
    ) -> Result<crate::modules::libnet::PosixSockOptVal, &'static str> {
        crate::modules::libnet::posix_getsockopt(fd, option)
    }

    pub fn posix_dup(&self, fd: u32) -> Result<u32, &'static str> {
        crate::modules::libnet::posix_dup(fd)
    }

    pub fn posix_dup2(&self, oldfd: u32, newfd: u32) -> Result<u32, &'static str> {
        crate::modules::libnet::posix_dup2(oldfd, newfd)
    }

    pub fn posix_ioctl(
        &self,
        fd: u32,
        cmd: crate::modules::libnet::PosixIoctlCmd,
    ) -> Result<usize, &'static str> {
        crate::modules::libnet::posix_ioctl(fd, cmd)
    }

    pub fn posix_fcntl(
        &self,
        fd: u32,
        cmd: crate::modules::libnet::PosixFcntlCmd,
    ) -> Result<crate::modules::libnet::PosixFdFlags, &'static str> {
        crate::modules::libnet::posix_fcntl(fd, cmd)
    }

    pub fn posix_fcntl_errno(
        &self,
        fd: u32,
        cmd: crate::modules::libnet::PosixFcntlCmd,
    ) -> Result<crate::modules::libnet::PosixFdFlags, crate::modules::libnet::PosixErrno> {
        crate::modules::libnet::posix_fcntl_errno(fd, cmd)
    }

    pub fn posix_setsockopt_errno(
        &self,
        fd: u32,
        option: crate::modules::libnet::PosixSockOpt,
        value: crate::modules::libnet::PosixSockOptVal,
    ) -> Result<(), crate::modules::libnet::PosixErrno> {
        crate::modules::libnet::posix_setsockopt_errno(fd, option, value)
    }

    pub fn posix_getsockopt_errno(
        &self,
        fd: u32,
        option: crate::modules::libnet::PosixSockOpt,
    ) -> Result<crate::modules::libnet::PosixSockOptVal, crate::modules::libnet::PosixErrno> {
        crate::modules::libnet::posix_getsockopt_errno(fd, option)
    }

    pub fn posix_dup_errno(&self, fd: u32) -> Result<u32, crate::modules::libnet::PosixErrno> {
        crate::modules::libnet::posix_dup_errno(fd)
    }

    pub fn posix_dup2_errno(
        &self,
        oldfd: u32,
        newfd: u32,
    ) -> Result<u32, crate::modules::libnet::PosixErrno> {
        crate::modules::libnet::posix_dup2_errno(oldfd, newfd)
    }

    pub fn posix_ioctl_errno(
        &self,
        fd: u32,
        cmd: crate::modules::libnet::PosixIoctlCmd,
    ) -> Result<usize, crate::modules::libnet::PosixErrno> {
        crate::modules::libnet::posix_ioctl_errno(fd, cmd)
    }

    pub fn posix_shutdown(
        &self,
        fd: u32,
        how: crate::modules::libnet::PosixShutdownHow,
    ) -> Result<(), &'static str> {
        crate::modules::libnet::posix_shutdown(fd, how)
    }

    pub fn posix_shutdown_errno(
        &self,
        fd: u32,
        how: crate::modules::libnet::PosixShutdownHow,
    ) -> Result<(), crate::modules::libnet::PosixErrno> {
        crate::modules::libnet::posix_shutdown_errno(fd, how)
    }

    pub fn posix_close(&self, fd: u32) -> Result<(), &'static str> {
        crate::modules::libnet::posix_close(fd)
    }

    pub fn udp_bind(
        &self,
        local_port: u16,
    ) -> Result<crate::modules::libnet::LibUdpSocket, &'static str> {
        crate::modules::libnet::udp_bind(local_port)
    }

    pub fn tcp_listen(
        &self,
        local_port: u16,
    ) -> Result<crate::modules::libnet::LibTcpListener, &'static str> {
        crate::modules::libnet::tcp_listen(local_port)
    }

    pub fn tcp_connect(
        &self,
        local_port: u16,
        remote_port: u16,
    ) -> Result<crate::modules::libnet::LibTcpStream, &'static str> {
        crate::modules::libnet::tcp_connect(local_port, remote_port)
    }

    pub fn dns_register(&self, name: &str, ipv4: [u8; 4]) -> Result<(), &'static str> {
        crate::modules::libnet::dns_register(name, ipv4)
    }

    pub fn dns_resolve(&self, name: &str) -> Option<[u8; 4]> {
        crate::modules::libnet::dns_resolve(name)
    }

    pub fn register_packet_filter(
        &self,
        protocol: crate::modules::libnet::FilterProtocol,
        src_port: Option<u16>,
        dst_port: Option<u16>,
        max_payload_len: Option<usize>,
        action: crate::modules::libnet::FilterAction,
    ) -> Result<u64, &'static str> {
        crate::modules::libnet::register_packet_filter(
            protocol,
            src_port,
            dst_port,
            max_payload_len,
            action,
        )
    }

    pub fn remove_packet_filter(&self, id: u64) -> bool {
        crate::modules::libnet::remove_packet_filter(id)
    }

    pub fn clear_packet_filters(&self) {
        crate::modules::libnet::clear_packet_filters();
    }

    pub fn transport_snapshot(&self) -> crate::modules::libnet::TransportSnapshot {
        crate::modules::libnet::transport_snapshot()
    }

    pub fn run_udp_relay_cycle(
        &self,
        socket: &crate::modules::libnet::LibUdpSocket,
        upstream_port: u16,
        max_packets: usize,
    ) -> crate::modules::libnet::ServiceRunReport {
        crate::modules::libnet::run_udp_relay_cycle(socket, upstream_port, max_packets)
    }

    pub fn run_udp_relay_cycle_with_preset(
        &self,
        socket: &crate::modules::libnet::LibUdpSocket,
        upstream_port: u16,
        max_packets: usize,
        preset: crate::modules::libnet::ServicePreset,
    ) -> crate::modules::libnet::ServiceRunReport {
        crate::modules::libnet::run_udp_relay_cycle_with_preset(
            socket,
            upstream_port,
            max_packets,
            preset,
        )
    }

    pub fn run_tcp_echo_cycle(
        &self,
        listener: &crate::modules::libnet::LibTcpListener,
        max_accepts: usize,
        max_chunks_per_stream: usize,
    ) -> crate::modules::libnet::ServiceRunReport {
        crate::modules::libnet::run_tcp_echo_cycle(listener, max_accepts, max_chunks_per_stream)
    }

    pub fn run_tcp_echo_cycle_with_preset(
        &self,
        listener: &crate::modules::libnet::LibTcpListener,
        max_accepts: usize,
        max_chunks_per_stream: usize,
        preset: crate::modules::libnet::ServicePreset,
    ) -> crate::modules::libnet::ServiceRunReport {
        crate::modules::libnet::run_tcp_echo_cycle_with_preset(
            listener,
            max_accepts,
            max_chunks_per_stream,
            preset,
        )
    }
}

#[cfg(feature = "network_http")]
impl LibNetApi {
    pub fn register_static_asset(
        &self,
        path: &str,
        content_type: &str,
        body: alloc::vec::Vec<u8>,
    ) -> Result<(), &'static str> {
        crate::modules::libnet::register_static_asset(path, content_type, body)
    }

    pub fn handle_static_request(
        &self,
        method: &str,
        path: &str,
        if_none_match: Option<u64>,
    ) -> crate::modules::libnet::HttpResponse {
        crate::modules::libnet::handle_static_request(method, path, if_none_match)
    }

    pub fn run_http_static_cycle(
        &self,
        method: &str,
        path: &str,
        if_none_match: Option<u64>,
    ) -> crate::modules::libnet::HttpResponse {
        crate::modules::libnet::run_http_static_cycle(method, path, if_none_match)
    }
}

#[cfg(feature = "network_https")]
impl LibNetApi {
    pub fn install_tls_server_config(&self, config: alloc::sync::Arc<rustls::ServerConfig>) {
        crate::modules::libnet::install_tls_server_config(config);
    }

    pub fn terminate_tls_record(
        &self,
        record: &[u8],
        out: &mut [u8],
    ) -> Result<usize, &'static str> {
        crate::modules::libnet::terminate_tls_record(record, out)
    }

    pub fn run_https_terminate_cycle(
        &self,
        record: &[u8],
        out: &mut [u8],
    ) -> Result<(usize, crate::modules::libnet::LibNetPumpReport), &'static str> {
        crate::modules::libnet::run_https_terminate_cycle(record, out)
    }

    pub fn run_https_terminate_cycle_with_preset(
        &self,
        record: &[u8],
        out: &mut [u8],
        preset: crate::modules::libnet::ServicePreset,
    ) -> Result<(usize, crate::modules::libnet::LibNetPumpReport), &'static str> {
        crate::modules::libnet::run_https_terminate_cycle_with_preset(record, out, preset)
    }
}
