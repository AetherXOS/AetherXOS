use super::types::*;

impl IntegrationHarness {
    pub fn new() -> Self {
        Self {
            next_pid: 100,
            proc_count: 0,
            processes: [None; MAX_PROCESSES],
            sigchld_delivered: false,
            proc_status_threads: 1,
            proc_pid_max: 32768,
            sysctl_pid_max: 32768,
            uptime_seconds: 1,
            reuse_addr: false,
            reuse_port: false,
            keep_alive: false,
            tcp_nodelay: false,
            tcp_cork: false,
            linger_on: false,
            linger_secs: 0,
            rcvbuf: 128 * 1024,
            sndbuf: 128 * 1024,
            rcvtimeo_ms: 0,
            sndtimeo_ms: 0,
            ip_ttl: 64,
            mcast_ttl: 1,
            mcast_loop: true,
            mcast_joined: false,
            broadcast: false,
            socket_type_stream: true,
            ptrace_attached_pid: None,
        }
    }

    pub fn first_free_slot(&self) -> Option<usize> {
        self.processes.iter().position(|p| p.is_none())
    }

    pub fn find_index(&self, pid: u32) -> Option<usize> {
        self.processes
            .iter()
            .position(|p| p.is_some() && p.unwrap().pid == pid)
    }

    pub fn process_count(&self) -> usize {
        self.proc_count
    }

    pub fn sigchld_observed(&self) -> bool {
        self.sigchld_delivered
    }
}
