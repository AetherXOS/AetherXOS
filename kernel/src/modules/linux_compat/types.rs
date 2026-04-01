// Reordered alphabetically for better maintenance

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxDirent64 {
    pub d_ino: u64,
    pub d_off: i64,
    pub d_reclen: u16,
    pub d_type: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxIoVec {
    pub iov_base: u64,
    pub iov_len: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxOpenHow {
    pub flags: u64,
    pub mode: u64,
    pub resolve: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxKSigAction {
    pub sa_handler: u64,
    pub sa_flags: u64,
    pub sa_restorer: u64,
    pub sa_mask: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxRusage {
    pub ru_utime: LinuxTimeval,
    pub ru_stime: LinuxTimeval,
    pub ru_maxrss: i64,
    pub ru_ixrss: i64,
    pub ru_idrss: i64,
    pub ru_isrss: i64,
    pub ru_minflt: i64,
    pub ru_majflt: i64,
    pub ru_nswap: i64,
    pub ru_inblock: i64,
    pub ru_oublock: i64,
    pub ru_msgsnd: i64,
    pub ru_msgrcv: i64,
    pub ru_nsignals: i64,
    pub ru_nvcsw: i64,
    pub ru_nivcsw: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxSiginfo {
    pub si_signo: i32,
    pub si_errno: i32,
    pub si_code: i32,
    pub si_pid: i32,
    pub si_uid: u32,
    pub si_status: i32,
    pub _pad: [i32; 26], // Standard size adjustment
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxSockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: [u8; 4],
    pub sin_zero: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxStat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub __pad0: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atime: i64,
    pub st_atime_nsec: i64,
    pub st_mtime: i64,
    pub st_mtime_nsec: i64,
    pub st_ctime: i64,
    pub st_ctime_nsec: i64,
    pub __unused: [i64; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxStatxTimestamp {
    pub tv_sec: i64,
    pub tv_nsec: u32,
    pub __reserved: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxStatx {
    pub stx_mask: u32,
    pub stx_blksize: u32,
    pub stx_attributes: u64,
    pub stx_nlink: u32,
    pub stx_uid: u32,
    pub stx_gid: u32,
    pub stx_mode: u16,
    pub __spare0: [u16; 1],
    pub stx_ino: u64,
    pub stx_size: u64,
    pub stx_blocks: u64,
    pub stx_attributes_mask: u64,
    pub stx_atime: LinuxStatxTimestamp,
    pub stx_btime: LinuxStatxTimestamp,
    pub stx_ctime: LinuxStatxTimestamp,
    pub stx_mtime: LinuxStatxTimestamp,
    pub stx_rdev_major: u32,
    pub stx_rdev_minor: u32,
    pub stx_dev_major: u32,
    pub stx_dev_minor: u32,
    pub __spare2: [u64; 14],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxMountAttr {
    pub attr_set: u64,
    pub attr_clr: u64,
    pub propagation: u64,
    pub userns_fd: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxStatfs {
    pub f_type: i64,
    pub f_bsize: i64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_fsid: [i32; 2],
    pub f_namelen: i64,
    pub f_frsize: i64,
    pub f_flags: i64,
    pub f_spare: [i64; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxSysinfo {
    pub uptime: i64,
    pub loads: [u64; 3],
    pub totalram: u64,
    pub freeram: u64,
    pub sharedram: u64,
    pub bufferram: u64,
    pub totalswap: u64,
    pub freeswap: u64,
    pub procs: u16,
    pub pad: u16,
    pub totalhigh: u64,
    pub freehigh: u64,
    pub mem_unit: u32,
    pub _f: [u8; 0], // Flexible array or padding
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct LinuxMContext {
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub rdx: u64,
    pub rax: u64,
    pub rcx: u64,
    pub rsp: u64,
    pub rip: u64,
    pub eflags: u64,
    pub cs: u16,
    pub gs: u16,
    pub fs: u16,
    pub ss: u16,
    pub err: u64,
    pub trapno: u64,
    pub oldmask: u64,
    pub cr2: u64,
    pub fpstate: u64, // Pointer
    pub __reserved1: [u64; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct LinuxUContext {
    pub flags: u64,
    pub link: u64,
    pub stack: LinuxStackT,
    pub mcontext: LinuxMContext,
    pub sigmask: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxUtimbuf {
    pub actime: i64,
    pub modtime: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct LinuxStackT {
    pub ss_sp: u64,
    pub ss_flags: i32,
    pub ss_size: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct LinuxSigaltstack {
    pub ss_sp: u64,
    pub ss_flags: i32,
    pub ss_size: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxTimes {
    pub tms_utime: i64,
    pub tms_stime: i64,
    pub tms_cutime: i64,
    pub tms_cstime: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxTimespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxTimeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxITimerVal {
    pub it_interval: LinuxTimeval,
    pub it_value: LinuxTimeval,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxMsgHdr {
    pub msg_name: u64,
    pub msg_namelen: u32,
    pub __pad1: u32,
    pub msg_iov: u64,
    pub msg_iovlen: u64,
    pub msg_control: u64,
    pub msg_controllen: u64,
    pub msg_flags: i32,
    pub __pad2: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxCmsghdr {
    pub cmsg_len: u64,
    pub cmsg_level: i32,
    pub cmsg_type: i32,
}
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct LinuxEpollEvent {
    pub events: u32,
    pub data: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxPollFd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

pub const LINUX_FD_SETSIZE: usize = 1024;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxFdSet {
    pub fds_bits: [u64; LINUX_FD_SETSIZE / 64],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxMmsghdr {
    pub msg_hdr: LinuxMsgHdr,
    pub msg_len: u32,
    pub __pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxFlock {
    pub l_type: i16,
    pub l_whence: i16,
    pub l_start: i64,
    pub l_len: i64,
    pub l_pid: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxFbFixScreeninfo {
    pub id: [u8; 16],
    pub smem_start: u64,
    pub smem_len: u32,
    pub type_: u32,
    pub type_aux: u32,
    pub visual: u32,
    pub xpanstep: u16,
    pub ypanstep: u16,
    pub ywrapstep: u16,
    pub line_length: u32,
    pub mmio_start: u64,
    pub mmio_len: u32,
    pub accel: u32,
    pub capabilities: u16,
    pub reserved: [u16; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxFbVarScreeninfo {
    pub xres: u32,
    pub yres: u32,
    pub xres_virtual: u32,
    pub yres_virtual: u32,
    pub xoffset: u32,
    pub yoffset: u32,
    pub bits_per_pixel: u32,
    pub grayscale: u32,
    pub red: [u16; 4],
    pub green: [u16; 4],
    pub blue: [u16; 4],
    pub transp: [u16; 4],
    pub nonstd: u32,
    pub activate: u32,
    pub height: u32,
    pub width: u32,
    pub accel_flags: u32,
    pub pixclock: u32,
    pub left_margin: u32,
    pub right_margin: u32,
    pub upper_margin: u32,
    pub lower_margin: u32,
    pub hsync_len: u32,
    pub vsync_len: u32,
    pub sync: u32,
    pub vmode: u32,
    pub rotate: u32,
    pub colorspace: u32,
    pub reserved: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxWinsize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxItimerspec {
    pub it_interval: LinuxTimespec,
    pub it_value: LinuxTimespec,
}
