pub(crate) fn encode_signaled(sig: u8, core_dump: bool) -> u32 {
    let mut status = sig as u32;
    if core_dump {
        status |= 1 << 7;
    }
    status
}

pub(crate) fn is_signaled(status: u32, sig: u8) -> bool {
    (status & 0x7f) == sig as u32
}

pub(crate) fn did_core_dump(status: u32) -> bool {
    (status & (1 << 7)) != 0
}

pub(crate) fn selector_class(pid: i32) -> u8 {
    if pid > 0 {
        1
    } else if pid == 0 {
        2
    } else if pid == -1 {
        3
    } else {
        4
    }
}
