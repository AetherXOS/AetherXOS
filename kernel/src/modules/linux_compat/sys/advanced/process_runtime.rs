use super::*;
use alloc::collections::BTreeMap;
use spin::Mutex;

static LINUX_RSEQ_REGISTRY: Mutex<BTreeMap<usize, LinuxRseqRegistration>> =
    Mutex::new(BTreeMap::new());

#[derive(Clone, Copy)]
struct LinuxRseqRegistration {
    addr: usize,
    len: usize,
    sig: usize,
}

pub fn sys_linux_execveat(
    frame: &mut SyscallFrame,
    dirfd: Fd,
    pathname_ptr: usize,
    argv_ptr: usize,
    envp_ptr: usize,
    flags: usize,
) -> usize {
    let path = match crate::modules::linux_compat::process::exec::resolve_linux_execveat_path(
        dirfd,
        pathname_ptr,
        flags,
    ) {
        Ok(v) => v,
        Err(e) => return e,
    };
    crate::modules::linux_compat::process::exec::execve_with_path(frame, path, argv_ptr, envp_ptr)
}

pub fn sys_linux_rseq(rseq: UserPtr<u8>, rseq_len: usize, flags: usize, sig: usize) -> usize {
    const RSEQ_FLAG_UNREGISTER: usize = 1;
    const RSEQ_MIN_LEN: usize = 32;

    if (flags & !RSEQ_FLAG_UNREGISTER) != 0 {
        return linux_inval();
    }

    crate::require_posix_process!((rseq, rseq_len, flags, sig) => {
        let tid = crate::modules::posix::process::gettid();
        if tid == 0 {
            return linux_esrch();
        }

        let unregister = (flags & RSEQ_FLAG_UNREGISTER) != 0;
        let mut table = LINUX_RSEQ_REGISTRY.lock();

        if unregister {
            if rseq.is_null() || rseq_len == 0 {
                table.remove(&tid);
                return 0;
            }
            match table.get(&tid) {
                Some(existing)
                    if existing.addr == rseq.addr
                        && existing.len == rseq_len
                        && existing.sig == sig =>
                {
                    table.remove(&tid);
                    0
                }
                _ => linux_inval(),
            }
        } else {
            if rseq.is_null() || rseq_len < RSEQ_MIN_LEN {
                return linux_inval();
            }
            match table.get(&tid) {
                Some(existing)
                    if existing.addr == rseq.addr
                        && existing.len == rseq_len
                        && existing.sig == sig =>
                {
                    0
                }
                Some(_) => linux_errno(crate::modules::posix_consts::errno::EBUSY),
                None => {
                    table.insert(
                        tid,
                        LinuxRseqRegistration {
                            addr: rseq.addr,
                            len: rseq_len,
                            sig,
                        },
                    );
                    0
                }
            }
        }
    })
}
