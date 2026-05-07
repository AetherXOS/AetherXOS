use super::types::*;

impl IntegrationHarness {
    pub fn setsockopt(
        &mut self,
        level: SocketLevel,
        opt: SocketOptName,
        value: bool,
    ) -> Result<(), IntegrationError> {
        match (level, opt) {
            (SocketLevel::SolSocket, SocketOptName::ReuseAddr) => {
                self.reuse_addr = value;
                Ok(())
            }
            (SocketLevel::SolSocket, SocketOptName::KeepAlive) => {
                self.keep_alive = value;
                Ok(())
            }
            (SocketLevel::IpProtoTcp, SocketOptName::TcpNoDelay) => {
                self.tcp_nodelay = value;
                Ok(())
            }
            _ => Err(IntegrationError::InvalidOption),
        }
    }

    pub fn getsockopt(&self, level: SocketLevel, opt: SocketOptName) -> Result<bool, IntegrationError> {
        match (level, opt) {
            (SocketLevel::SolSocket, SocketOptName::ReuseAddr) => Ok(self.reuse_addr),
            (SocketLevel::SolSocket, SocketOptName::KeepAlive) => Ok(self.keep_alive),
            (SocketLevel::IpProtoTcp, SocketOptName::TcpNoDelay) => Ok(self.tcp_nodelay),
            _ => Err(IntegrationError::InvalidOption),
        }
    }

    pub fn set_reuseport(&mut self, value: bool) {
        self.reuse_port = value;
    }

    pub fn reuseport_enabled(&self) -> bool {
        self.reuse_port
    }

    pub fn set_tcp_cork(&mut self, value: bool) {
        self.tcp_cork = value;
    }

    pub fn tcp_cork_enabled(&self) -> bool {
        self.tcp_cork
    }

    pub fn set_linger(&mut self, on: bool, secs: u32) {
        self.linger_on = on;
        self.linger_secs = secs;
    }

    pub fn linger_state(&self) -> (bool, u32) {
        (self.linger_on, self.linger_secs)
    }

    pub fn set_socket_buffers(&mut self, rcv: u32, snd: u32) -> Result<(), IntegrationError> {
        if rcv == 0 || snd == 0 {
            return Err(IntegrationError::InvalidOption);
        }
        self.rcvbuf = rcv;
        self.sndbuf = snd;
        Ok(())
    }

    pub fn socket_buffers(&self) -> (u32, u32) {
        (self.rcvbuf, self.sndbuf)
    }

    pub fn set_socket_timeouts(&mut self, rcv_ms: u32, snd_ms: u32) {
        self.rcvtimeo_ms = rcv_ms;
        self.sndtimeo_ms = snd_ms;
    }

    pub fn socket_timeouts(&self) -> (u32, u32) {
        (self.rcvtimeo_ms, self.sndtimeo_ms)
    }

    pub fn set_ip_ttl(&mut self, ttl: u8) -> Result<(), IntegrationError> {
        if ttl == 0 {
            return Err(IntegrationError::InvalidOption);
        }
        self.ip_ttl = ttl;
        Ok(())
    }

    pub fn ip_ttl(&self) -> u8 {
        self.ip_ttl
    }

    pub fn set_multicast_ttl(&mut self, ttl: u8) {
        self.mcast_ttl = ttl;
    }

    pub fn multicast_ttl(&self) -> u8 {
        self.mcast_ttl
    }

    pub fn set_multicast_loop(&mut self, enabled: bool) {
        self.mcast_loop = enabled;
    }

    pub fn multicast_loop_enabled(&self) -> bool {
        self.mcast_loop
    }

    pub fn join_multicast_group(&mut self, group: &str) -> Result<(), IntegrationError> {
        if !group.starts_with("224.") {
            return Err(IntegrationError::InvalidOption);
        }
        self.mcast_joined = true;
        Ok(())
    }

    pub fn leave_multicast_group(&mut self) -> Result<(), IntegrationError> {
        if !self.mcast_joined {
            return Err(IntegrationError::InvalidOption);
        }
        self.mcast_joined = false;
        Ok(())
    }

    pub fn multicast_joined(&self) -> bool {
        self.mcast_joined
    }

    pub fn set_broadcast(&mut self, enabled: bool) {
        self.broadcast = enabled;
    }

    pub fn broadcast_enabled(&self) -> bool {
        self.broadcast
    }

    pub fn socket_type_stream(&self) -> bool {
        self.socket_type_stream
    }

    pub fn boundary_mode_socket_valid(&self, mode: &str) -> bool {
        matches!(mode, "strict" | "balanced" | "compat")
    }
}
