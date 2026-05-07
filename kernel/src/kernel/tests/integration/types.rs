pub const STATUS_EXITED_FLAG: u32 = 0x0100;
pub const MAX_PROCESSES: usize = 32;
pub const PAGE_SIZE: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationError {
    InvalidSignal,
    InvalidAlignment,
    InvalidPid,
    InvalidOption,
    InvalidFormat,
    PermissionDenied,
    InvalidPtraceRequest,
    BufferTooSmall,
    ConsistencyMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegisterState {
    pub rip: usize,
    pub rsp: usize,
    pub rax: usize,
    pub rbx: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignalFrame {
    pub frame_addr: usize,
    pub restorer_addr: usize,
    pub regs: RegisterState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessRecord {
    pub pid: u32,
    pub parent_pid: u32,
    pub cow_pages: u16,
    pub shared_fd_count: u16,
    pub signal_handler_count: u16,
    pub exited: bool,
    pub zombie: bool,
    pub exit_code: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitFlags;

impl WaitFlags {
    pub const NONE: u32 = 0;
    pub const WNOHANG: u32 = 1 << 0;
    pub const WUNTRACED: u32 = 1 << 1;
    pub const WCONTINUED: u32 = 1 << 2;
    pub const ALLOWED_MASK: u32 = Self::WNOHANG | Self::WUNTRACED | Self::WCONTINUED;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitOutcome {
    Running,
    Reaped { pid: u32, status: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatRecord {
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: usize,
    pub inode: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SocketLevel {
    SolSocket,
    IpProtoTcp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SocketOptName {
    ReuseAddr,
    KeepAlive,
    TcpNoDelay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PtraceRequest {
    Attach,
    GetRegs,
    SingleStep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntegrationHarness {
    pub next_pid: u32,
    pub proc_count: usize,
    pub processes: [Option<ProcessRecord>; MAX_PROCESSES],
    pub sigchld_delivered: bool,
    pub proc_status_threads: u32,
    pub proc_pid_max: u32,
    pub sysctl_pid_max: u32,
    pub uptime_seconds: u64,
    pub reuse_addr: bool,
    pub reuse_port: bool,
    pub keep_alive: bool,
    pub tcp_nodelay: bool,
    pub tcp_cork: bool,
    pub linger_on: bool,
    pub linger_secs: u32,
    pub rcvbuf: u32,
    pub sndbuf: u32,
    pub rcvtimeo_ms: u32,
    pub sndtimeo_ms: u32,
    pub ip_ttl: u8,
    pub mcast_ttl: u8,
    pub mcast_loop: bool,
    pub mcast_joined: bool,
    pub broadcast: bool,
    pub socket_type_stream: bool,
    pub ptrace_attached_pid: Option<u32>,
}
