use super::super::*;

pub fn sys_linux_rt_sigprocmask(
    how: usize,
    set: UserPtr<u64>,
    oldset: UserPtr<u64>,
    sigsetsize: usize,
) -> usize {
    crate::require_posix_signal!((how, set, oldset, sigsetsize) => {
        use crate::modules::posix::signal::{self, SigmaskHow};

        if sigsetsize != linux::SIGSET_SIZE {
            return linux_inval();
        }

        let how_enum = match how as i32 {
            crate::modules::posix_consts::signal::SIG_BLOCK => SigmaskHow::Block,
            crate::modules::posix_consts::signal::SIG_SETMASK => SigmaskHow::SetMask,
            crate::modules::posix_consts::signal::SIG_UNBLOCK => SigmaskHow::Unblock,
            _ => return linux_inval(),
        };

        // Read new set from userspace
        let new_set = if !set.is_null() {
            match set.read() {
                Ok(v) => Some(v),
                Err(e) => return e,
            }
        } else {
            None
        };

        match signal::sigprocmask(how_enum, new_set) {
            Ok(old_mask) => {
                if !oldset.is_null() {
                    if let Err(e) = oldset.write(&old_mask) { return e; }
                }
                0
            }
            Err(_) => linux_inval(),
        }
    })
}
pub fn sys_linux_rt_sigpending(set: UserPtr<u64>, sigsetsize: usize) -> usize {
    crate::require_posix_signal!((set, sigsetsize) => {
        if sigsetsize != linux::SIGSET_SIZE {
                    return linux_inval();
                }
                if !set.is_null() {
                    let pending = crate::modules::posix::signal::sigpending();
                    if let Err(e) = set.write(&pending) { return e; }
                }
                0
    })
}

pub fn sys_linux_rt_sigsuspend(unmask: UserPtr<u64>, sigsetsize: usize) -> usize {
    crate::require_posix_signal!((unmask, sigsetsize) => {
        if sigsetsize != linux::SIGSET_SIZE {
                    return linux_inval();
                }
                let mask = if !unmask.is_null() {
                    match unmask.read() {
                        Ok(v) => v,
                        Err(e) => return e,
                    }
                } else {
                    0
                };

                match crate::modules::posix::signal::sigsuspend(mask) {
                    Ok(_) => 0,
                    Err(e) => linux_errno(e.code()),
                }
    })
}

pub fn sys_linux_pause() -> usize {
    crate::require_posix_signal!(() => {
        match crate::modules::posix::signal::pause() {
                    Ok(_) => 0,
                    Err(e) => linux_errno(e.code()),
                }
    })
}
