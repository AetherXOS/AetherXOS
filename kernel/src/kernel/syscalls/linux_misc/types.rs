#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct LinuxTimespecCompat {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct LinuxItimerspecCompat {
    pub it_interval: LinuxTimespecCompat,
    pub it_value: LinuxTimespecCompat,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct TimerfdRuntimeState {
    pub spec: LinuxItimerspecCompat,
    pub armed_at_ns: u128,
}

#[derive(Clone, Debug)]
pub struct FanotifyMarkState {
    pub mask: usize,
    pub dirfd: isize,
    pub path: alloc::string::String,
}

#[derive(Clone, Debug)]
pub struct InotifyWatchState {
    pub wd: i32,
    pub path: alloc::string::String,
    pub mask: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct LinuxFutexWaitVCompat {
    pub val: u64,
    pub uaddr: u64,
    pub flags: u32,
    pub __reserved: u32,
}
