use super::super::*;
use crate::modules::posix_consts::errno;
use crate::klog_error;
#[cfg(not(feature = "ipc_shared_memory"))]
use alloc::collections::BTreeMap;
#[cfg(not(feature = "ipc_shared_memory"))]
use lazy_static::lazy_static;
#[cfg(not(feature = "ipc_shared_memory"))]
use spin::Mutex;

#[cfg(feature = "ipc_shared_memory")]
const IPC_RMID: i32 = 0;
#[cfg(feature = "ipc_shared_memory")]
const IPC_SET: i32 = 1;
#[cfg(feature = "ipc_shared_memory")]
const IPC_STAT: i32 = 2;
#[cfg(feature = "ipc_shared_memory")]
const SHM_RDONLY: i32 = 0o10000;
#[cfg(not(feature = "ipc_shared_memory"))]
const IPC_RMID: i32 = 0;
#[cfg(not(feature = "ipc_shared_memory"))]
const IPC_SET: i32 = 1;
#[cfg(not(feature = "ipc_shared_memory"))]
const IPC_STAT: i32 = 2;
#[cfg(not(feature = "ipc_shared_memory"))]
const SHM_RDONLY: i32 = 0o10000;
#[cfg(not(feature = "ipc_shared_memory"))]
const IPC_PRIVATE: i32 = 0;

#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxIpcPerm {
    key: u32,
    uid: u32,
    gid: u32,
    cuid: u32,
    cgid: u32,
    mode: u16,
    seq: u16,
    _pad1: u64,
    _pad2: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxShmidDs {
    shm_perm: LinuxIpcPerm,
    shm_segsz: usize,
    shm_atime: i64,
    shm_dtime: i64,
    shm_ctime: i64,
    shm_cpid: i32,
    shm_lpid: i32,
    shm_nattch: usize,
    _unused4: usize,
    _unused5: usize,
}

#[cfg(not(feature = "ipc_shared_memory"))]
#[derive(Clone, Copy)]
struct CompatShmSeg {
    key: i32,
    size: usize,
    permissions: i32,
}

#[cfg(not(feature = "ipc_shared_memory"))]
struct CompatShmState {
    next_id: i32,
    by_id: BTreeMap<i32, CompatShmSeg>,
    key_to_id: BTreeMap<i32, i32>,
}

#[cfg(not(feature = "ipc_shared_memory"))]
impl CompatShmState {
    fn new() -> Self {
        Self {
            next_id: 1,
            by_id: BTreeMap::new(),
            key_to_id: BTreeMap::new(),
        }
    }
}

#[cfg(not(feature = "ipc_shared_memory"))]
lazy_static! {
    static ref COMPAT_SYSV_SHM: Mutex<CompatShmState> = Mutex::new(CompatShmState::new());
}

/// `shmget(2)` — allocates a System V shared memory segment.
pub fn sys_linux_shmget(key: i32, size: usize, shmflg: i32) -> usize {
    #[cfg(not(feature = "ipc_shared_memory"))]
    {
        if size == 0 {
            return linux_inval();
        }
        let mut state = COMPAT_SYSV_SHM.lock();
        if key != IPC_PRIVATE {
            if let Some(id) = state.key_to_id.get(&key) {
                return *id as usize;
            }
        }
        let id = state.next_id;
        state.next_id = state.next_id.saturating_add(1);
        state.by_id.insert(
            id,
            CompatShmSeg {
                key,
                size,
                permissions: shmflg & 0o777,
            },
        );
        if key != IPC_PRIVATE {
            state.key_to_id.insert(key, id);
        }
        return id as usize;
    }
    #[cfg(feature = "ipc_shared_memory")]
    match crate::modules::ipc::shared_memory::shm_get(key, size, shmflg) {
        Ok(shmid) => shmid.0 as usize,
        Err(e) => linux_errno(e.code()),
    }
}

/// `shmat(2)` — attach the shared memory segment into the address space.
pub fn sys_linux_shmat(shmid: i32, shmaddr: UserPtr<u8>, _shmflg: i32) -> usize {
    #[cfg(not(feature = "ipc_shared_memory"))]
    {
        let size = {
            let state = COMPAT_SYSV_SHM.lock();
            let Some(seg) = state.by_id.get(&shmid) else {
                return linux_errno(errno::EINVAL);
            };
            seg.size
        };

        let start = if shmaddr.is_null() {
            let pid = match current_process_id() {
                Some(p) => p,
                None => return linux_fault(),
            };
            let process = match crate::kernel::launch::process_arc_by_id(
                crate::interfaces::task::ProcessId(pid),
            ) {
                Some(p) => p,
                None => return linux_fault(),
            };
            match process.allocate_user_vaddr(size) {
                Ok(v) => v,
                Err(_) => return linux_errno(errno::ENOMEM),
            }
        } else {
            let addr = shmaddr.addr as u64;
            if (addr & 4095) != 0 {
                return linux_errno(errno::EINVAL);
            }
            addr
        };

        let map_id = 2_000_000 + shmid as u32;
        let pid = match current_process_id() {
            Some(p) => p,
            None => return linux_fault(),
        };
        let process =
            match crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid))
            {
                Some(p) => p,
                None => return linux_fault(),
            };
        let prot = if (_shmflg & SHM_RDONLY) != 0 {
            crate::modules::posix_consts::mman::PROT_READ
        } else {
            crate::modules::posix_consts::mman::PROT_READ
                | crate::modules::posix_consts::mman::PROT_WRITE
        };
        let end = start + size as u64;
        if let Err(e) = process.register_mapping(map_id, start, end, prot, 0) {
            klog_error!("shmat compat: failed to register mapping: {}", e);
            return linux_errno(errno::ENOMEM);
        }
        return start as usize;
    }
    #[cfg(feature = "ipc_shared_memory")]
    {
        // 1. Get region to verify it exists and get its size
        let shm = match crate::modules::ipc::shared_memory::shm_get_region(shmid) {
            Some(s) => s,
            None => return linux_errno(errno::EINVAL),
        };

        let size = shm.size as usize;

        // 2. Resolve target virtual address
        let start = if shmaddr.is_null() {
            // High-level "mmap-like" auto allocation
            let pid = match current_process_id() {
                Some(p) => p,
                None => return linux_fault(),
            };
            let process = match crate::kernel::launch::process_arc_by_id(
                crate::interfaces::task::ProcessId(pid),
            ) {
                Some(p) => p,
                None => return linux_fault(),
            };
            match process.allocate_user_vaddr(size) {
                Ok(v) => v,
                Err(_) => return linux_errno(errno::ENOMEM),
            }
        } else {
            // Fixed address attachment
            let addr = shmaddr.addr as u64;
            if (addr & 4095) != 0 {
                return linux_errno(errno::EINVAL);
            }
            addr
        };

        // 3. Register mapping in the process.
        // Map ID for SHM is 2,000,000 + ShmId to distinguish it in the VMM page fault handler.
        let map_id = 2_000_000 + shmid as u32;
        let pid = match current_process_id() {
            Some(p) => p,
            None => return linux_fault(),
        };
        let process =
            match crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid))
            {
                Some(p) => p,
                None => return linux_fault(),
            };

        // SHM_RDONLY maps read-only; default is read+write.
        let prot = if (_shmflg & SHM_RDONLY) != 0 {
            crate::modules::posix_consts::mman::PROT_READ
        } else {
            crate::modules::posix_consts::mman::PROT_READ
                | crate::modules::posix_consts::mman::PROT_WRITE
        };
        let end = start + size as u64;

        if let Err(e) = process.register_mapping(map_id, start, end, prot, 0) {
            klog_error!("shmat: failed to register mapping: {}", e);
            return linux_errno(errno::ENOMEM);
        }

        linux_trace!(
            "[IPC] shmat: id={}, vaddr={:#x}, size={}\n",
            shmid,
            start,
            size
        );
        start as usize
    }
}

/// `shmdt(2)` — detach the shared memory segment.
pub fn sys_linux_shmdt(shmaddr: UserPtr<u8>) -> usize {
    let pid = match current_process_id() {
        Some(p) => p,
        None => return linux_fault(),
    };
    let process =
        match crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid)) {
            Some(p) => p,
            None => return linux_fault(),
        };

    let vaddr = shmaddr.addr as u64;
    if let Some(record) = process.lookup_mapping(vaddr) {
        if record.map_id >= 2_000_000 {
            process.remove_mapping(record.map_id);
            // In a real Linux, we'd also unmap from the page tables here.
            // For now, we rely on the process being cleaned up or the record being gone.
            return 0;
        }
    }

    linux_errno(errno::EINVAL)
}

/// `shmctl(2)` — shared memory control.
pub fn sys_linux_shmctl(shmid: i32, cmd: i32, _buf: UserPtr<u8>) -> usize {
    #[cfg(not(feature = "ipc_shared_memory"))]
    {
        match cmd {
            IPC_RMID => {
                let mut state = COMPAT_SYSV_SHM.lock();
                if state.by_id.remove(&shmid).is_none() {
                    return linux_errno(errno::EINVAL);
                }
                state.key_to_id.retain(|_, v| *v != shmid);
                0
            }
            IPC_STAT => {
                if _buf.is_null() {
                    return linux_fault();
                }
                let seg = {
                    let state = COMPAT_SYSV_SHM.lock();
                    let Some(v) = state.by_id.get(&shmid) else {
                        return linux_errno(errno::EINVAL);
                    };
                    *v
                };
                let now = crate::modules::posix::time::monotonic_timespec().sec;
                let out = LinuxShmidDs {
                    shm_perm: LinuxIpcPerm {
                        key: seg.key as u32,
                        uid: 0,
                        gid: 0,
                        cuid: 0,
                        cgid: 0,
                        mode: (seg.permissions & 0o777) as u16,
                        seq: 0,
                        _pad1: 0,
                        _pad2: 0,
                    },
                    shm_segsz: seg.size,
                    shm_atime: now,
                    shm_dtime: 0,
                    shm_ctime: now,
                    shm_cpid: 0,
                    shm_lpid: 0,
                    shm_nattch: 0,
                    _unused4: 0,
                    _unused5: 0,
                };
                match _buf.cast::<LinuxShmidDs>().write(&out) {
                    Ok(()) => 0,
                    Err(e) => e,
                }
            }
            IPC_SET => {
                if _buf.is_null() {
                    return linux_fault();
                }
                0
            }
            _ => linux_inval(),
        }
    }
    #[cfg(feature = "ipc_shared_memory")]
    {
        match cmd {
            IPC_RMID => match crate::modules::ipc::shared_memory::shm_rmid(shmid) {
                Ok(()) => 0,
                Err(e) => linux_errno(e.code()),
            },
            IPC_STAT => {
                if _buf.is_null() {
                    return linux_fault();
                }
                let shm = match crate::modules::ipc::shared_memory::shm_get_region(shmid) {
                    Some(v) => v,
                    None => return linux_errno(errno::EINVAL),
                };
                let now = crate::modules::posix::time::monotonic_timespec().sec;
                let out = LinuxShmidDs {
                    shm_perm: LinuxIpcPerm {
                        key: shm.key as u32,
                        uid: 0,
                        gid: 0,
                        cuid: 0,
                        cgid: 0,
                        mode: (shm.permissions & 0o777) as u16,
                        seq: 0,
                        _pad1: 0,
                        _pad2: 0,
                    },
                    shm_segsz: shm.size,
                    shm_atime: now,
                    shm_dtime: 0,
                    shm_ctime: now,
                    shm_cpid: shm.owner.0 as i32,
                    shm_lpid: shm.creator_tid.0 as i32,
                    shm_nattch: 0,
                    _unused4: 0,
                    _unused5: 0,
                };
                match _buf.cast::<LinuxShmidDs>().write(&out) {
                    Ok(()) => 0,
                    Err(e) => e,
                }
            }
            IPC_SET => {
                // Accept and ignore permission updates for now.
                if _buf.is_null() {
                    return linux_fault();
                }
                0
            }
            _ => linux_inval(),
        }
    }
}
