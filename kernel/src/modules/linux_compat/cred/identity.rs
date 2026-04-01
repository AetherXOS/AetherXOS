use super::super::*;

pub fn sys_linux_getgid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::getgid() as usize
    }
    #[cfg(not(feature = "posix_process"))]
    {
        0
    }
}

pub fn sys_linux_getpid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::getpid()
    }
    #[cfg(not(feature = "posix_process"))]
    {
        1
    }
}

pub fn sys_linux_getuid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::getuid() as usize
    }
    #[cfg(not(feature = "posix_process"))]
    {
        0
    }
}

pub fn sys_linux_geteuid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::geteuid() as usize
    }
    #[cfg(not(feature = "posix_process"))]
    {
        0
    }
}

pub fn sys_linux_getegid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::getegid() as usize
    }
    #[cfg(not(feature = "posix_process"))]
    {
        0
    }
}

pub fn sys_linux_getppid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::getppid()
    }
    #[cfg(not(feature = "posix_process"))]
    {
        0
    }
}

pub fn sys_linux_getpgid(pid: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        match crate::modules::posix::process::getpgid(pid) {
            Ok(pgid) => pgid,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = pid;
        0
    }
}

pub fn sys_linux_gettid() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::gettid()
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
        cpu.current_task.load(core::sync::atomic::Ordering::Relaxed)
    }
}

pub fn sys_linux_set_tid_address(tidptr: usize) -> usize {
    let tid = sys_linux_gettid();
    if let Some(task_arc) = crate::kernel::task::get_task(crate::interfaces::task::TaskId(tid)) {
        task_arc.lock().clear_child_tid = tidptr;
    }
    tid
}

pub fn sys_linux_setpgid(pid: usize, pgid: usize) -> usize {
    crate::require_posix_process!((pid, pgid) => {
        match crate::modules::posix::process::setpgid(pid, pgid) { Ok(()) => 0, Err(e) => linux_errno(e.code()) }
    })
}

pub fn sys_linux_getpgrp() -> usize {
    #[cfg(feature = "posix_process")]
    {
        crate::modules::posix::process::getpgrp()
    }
    #[cfg(not(feature = "posix_process"))]
    {
        0
    }
}

pub fn sys_linux_setsid() -> usize {
    crate::require_posix_process!(() => {
        match crate::modules::posix::process::setsid() { Ok(sid) => sid, Err(e) => linux_errno(e.code()) }
    })
}

pub fn sys_linux_getsid(pid: usize) -> usize {
    crate::require_posix_process!((pid) => {
        match crate::modules::posix::process::getsid(pid) { Ok(sid) => sid, Err(e) => linux_errno(e.code()) }
    })
}

pub fn sys_linux_setuid(uid: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        match crate::modules::posix::process::setuid(uid as u32) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = uid;
        0
    }
}

pub fn sys_linux_setgid(gid: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        match crate::modules::posix::process::setgid(gid as u32) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = gid;
        0
    }
}

pub fn sys_linux_setresuid(ruid: usize, euid: usize, suid: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        match crate::modules::posix::process::setresuid(ruid as u32, euid as u32, suid as u32) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (ruid, euid, suid);
        0
    }
}

pub fn sys_linux_setresgid(rgid: usize, egid: usize, sgid: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        match crate::modules::posix::process::setresgid(rgid as u32, egid as u32, sgid as u32) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (rgid, egid, sgid);
        0
    }
}

pub fn sys_linux_getresuid(
    ruid_ptr: UserPtr<u32>,
    euid_ptr: UserPtr<u32>,
    suid_ptr: UserPtr<u32>,
) -> usize {
    #[cfg(feature = "posix_process")]
    {
        let (r, e, s) = crate::modules::posix::process::getresuid();
        if !ruid_ptr.is_null() {
            let _ = ruid_ptr.write(&r);
        }
        if !euid_ptr.is_null() {
            let _ = euid_ptr.write(&e);
        }
        if !suid_ptr.is_null() {
            let _ = suid_ptr.write(&s);
        }
        0
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (ruid_ptr, euid_ptr, suid_ptr);
        0
    }
}

pub fn sys_linux_getresgid(
    rgid_ptr: UserPtr<u32>,
    egid_ptr: UserPtr<u32>,
    sgid_ptr: UserPtr<u32>,
) -> usize {
    #[cfg(feature = "posix_process")]
    {
        let (r, e, s) = crate::modules::posix::process::getresgid();
        if !rgid_ptr.is_null() {
            let _ = rgid_ptr.write(&r);
        }
        if !egid_ptr.is_null() {
            let _ = egid_ptr.write(&e);
        }
        if !sgid_ptr.is_null() {
            let _ = sgid_ptr.write(&s);
        }
        0
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (rgid_ptr, egid_ptr, sgid_ptr);
        0
    }
}

pub fn sys_linux_personality(persona: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        if persona == !0 {
            crate::modules::posix::process::get_personality() as usize
        } else {
            crate::modules::posix::process::set_personality(persona as u32) as usize
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = persona;
        0
    }
}

pub fn sys_linux_getgroups(size: usize, list: UserPtr<u32>) -> usize {
    #[cfg(feature = "posix_process")]
    {
        if size == 0 {
            return crate::modules::posix::process::get_groups_len();
        }
        if list.is_null() {
            return linux_fault();
        }
        let groups = crate::modules::posix::process::get_groups_snapshot();
        if size < groups.len() {
            return linux_inval();
        }
        let mut written = 0;
        for &gid in &groups {
            if let Err(e) = list.offset(written).write(&gid) {
                return e;
            }
            written += 1;
        }
        written
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (size, list);
        0
    }
}

pub fn sys_linux_setgroups(size: usize, list: UserPtr<u32>) -> usize {
    #[cfg(feature = "posix_process")]
    {
        let mut groups = alloc::vec::Vec::with_capacity(size);
        for i in 0..size {
            match list.offset(i).read() {
                Ok(gid) => groups.push(gid),
                Err(e) => return e,
            }
        }
        match crate::modules::posix::process::setgroups(&groups) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (size, list);
        0
    }
}
