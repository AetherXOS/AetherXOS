#[inline(always)]
pub fn encode_wait_exit_status(code: u8) -> i32 {
    ((code as i32) & 0xff) << 8
}

#[inline(always)]
pub fn encode_wait_signal_status(signum: i32) -> i32 {
    signum & 0x7f
}

#[inline(always)]
pub fn wait_exited(status: i32) -> bool {
    (status & 0x7f) == 0
}

#[inline(always)]
pub fn wait_exit_code(status: i32) -> u8 {
    ((status >> 8) & 0xff) as u8
}

#[inline(always)]
pub fn wait_signaled(status: i32) -> bool {
    let sig = status & 0x7f;
    sig != 0 && sig != 0x7f
}

#[inline(always)]
pub fn wait_term_signal(status: i32) -> i32 {
    status & 0x7f
}

#[inline(always)]
pub(crate) fn wait_code_from_status(status: i32) -> i32 {
    if wait_exited(status) {
        crate::modules::posix_consts::process::CLD_EXITED
    } else if wait_signaled(status) {
        crate::modules::posix_consts::process::CLD_KILLED
    } else {
        crate::modules::posix_consts::process::CLD_CONTINUED
    }
}
