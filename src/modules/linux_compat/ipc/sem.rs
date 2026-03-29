use super::super::*;

#[cfg(not(feature = "ipc_sysv_sem"))]
use alloc::collections::BTreeMap;
#[cfg(not(feature = "ipc_sysv_sem"))]
use alloc::vec::Vec;
#[cfg(not(feature = "ipc_sysv_sem"))]
use lazy_static::lazy_static;
#[cfg(not(feature = "ipc_sysv_sem"))]
use spin::Mutex;

#[cfg(not(feature = "ipc_sysv_sem"))]
const IPC_PRIVATE: i32 = 0;
#[cfg(not(feature = "ipc_sysv_sem"))]
const IPC_RMID: i32 = 0;
#[cfg(not(feature = "ipc_sysv_sem"))]
const GETVAL: i32 = 12;
#[cfg(not(feature = "ipc_sysv_sem"))]
const SETVAL: i32 = 16;

#[cfg(not(feature = "ipc_sysv_sem"))]
#[derive(Clone)]
struct CompatSemSet {
    values: Vec<i32>,
}

#[cfg(not(feature = "ipc_sysv_sem"))]
struct CompatSemState {
    next_id: i32,
    by_id: BTreeMap<i32, CompatSemSet>,
    key_to_id: BTreeMap<i32, i32>,
}

#[cfg(not(feature = "ipc_sysv_sem"))]
impl CompatSemState {
    fn new() -> Self {
        Self {
            next_id: 1,
            by_id: BTreeMap::new(),
            key_to_id: BTreeMap::new(),
        }
    }
}

#[cfg(not(feature = "ipc_sysv_sem"))]
lazy_static! {
    static ref COMPAT_SYSV_SEM: Mutex<CompatSemState> = Mutex::new(CompatSemState::new());
}

pub fn sys_linux_semget(key: i32, nsems: i32, semflg: i32) -> usize {
    #[cfg(not(feature = "ipc_sysv_sem"))]
    {
        let _ = semflg;
        if nsems <= 0 {
            return linux_inval();
        }
        let mut state = COMPAT_SYSV_SEM.lock();
        if key != IPC_PRIVATE {
            if let Some(id) = state.key_to_id.get(&key) {
                return *id as usize;
            }
        }

        let id = state.next_id;
        state.next_id = state.next_id.saturating_add(1);
        state.by_id.insert(
            id,
            CompatSemSet {
                values: alloc::vec![0; nsems as usize],
            },
        );
        if key != IPC_PRIVATE {
            state.key_to_id.insert(key, id);
        }
        return id as usize;
    }
    #[cfg(feature = "ipc_sysv_sem")]
    match crate::modules::ipc::semaphores::semget(key, nsems, semflg as u32) {
        Ok(id) => id as usize,
        Err(e) => linux_errno(e.code()),
    }
}

pub fn sys_linux_semop(id: i32, ops_ptr: UserPtr<u8>, nsops: usize) -> usize {
    #[cfg(not(feature = "ipc_sysv_sem"))]
    {
        let mut ops = alloc::vec![(0u16, 0i16, 0i16); nsops];
        for i in 0..nsops {
            let mut buf = [0u8; 6];
            if ops_ptr.offset(i * 6).read_bytes(&mut buf).is_err() {
                return linux_fault();
            }
            let num = u16::from_le_bytes([buf[0], buf[1]]);
            let op = i16::from_le_bytes([buf[2], buf[3]]);
            let flg = i16::from_le_bytes([buf[4], buf[5]]);
            ops[i] = (num, op, flg);
        }

        let mut state = COMPAT_SYSV_SEM.lock();
        let Some(set) = state.by_id.get_mut(&id) else {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        };

        let mut shadow = set.values.clone();
        for (sem_num, sem_op, _sem_flg) in &ops {
            let idx = *sem_num as usize;
            if idx >= shadow.len() {
                return linux_errno(crate::modules::posix_consts::errno::EINVAL);
            }
            let cur = shadow[idx];
            let delta = *sem_op as i32;
            if delta < 0 {
                if cur + delta < 0 {
                    return linux_errno(crate::modules::posix_consts::errno::EAGAIN);
                }
            } else if delta == 0 && cur != 0 {
                return linux_errno(crate::modules::posix_consts::errno::EAGAIN);
            }
            shadow[idx] = cur + delta;
        }

        set.values = shadow;
        return 0;
    }
    #[cfg(feature = "ipc_sysv_sem")]
    {
        let mut ops = alloc::vec![(0u16, 0i16, 0i16); nsops];
        // Each op is: sem_num (u16), sem_op (i16), sem_flg (i16) = 6 bytes
        for i in 0..nsops {
            let mut buf = [0u8; 6];
            if ops_ptr.offset(i * 6).read_bytes(&mut buf).is_err() {
                return linux_fault();
            }
            let num = u16::from_le_bytes([buf[0], buf[1]]);
            let op = i16::from_le_bytes([buf[2], buf[3]]);
            let flg = i16::from_le_bytes([buf[4], buf[5]]);
            ops[i] = (num, op, flg);
        }

        match crate::modules::ipc::semaphores::semop(id, &ops) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    }
}

pub fn sys_linux_semctl(id: i32, semnum: i32, cmd: i32, arg: usize) -> usize {
    #[cfg(not(feature = "ipc_sysv_sem"))]
    {
        let mut state = COMPAT_SYSV_SEM.lock();
        match cmd {
            IPC_RMID => {
                if state.by_id.remove(&id).is_none() {
                    return linux_errno(crate::modules::posix_consts::errno::EINVAL);
                }
                state.key_to_id.retain(|_, v| *v != id);
                0
            }
            GETVAL => {
                let Some(set) = state.by_id.get(&id) else {
                    return linux_errno(crate::modules::posix_consts::errno::EINVAL);
                };
                if semnum < 0 || semnum as usize >= set.values.len() {
                    return linux_errno(crate::modules::posix_consts::errno::EINVAL);
                }
                set.values[semnum as usize] as usize
            }
            SETVAL => {
                let Some(set) = state.by_id.get_mut(&id) else {
                    return linux_errno(crate::modules::posix_consts::errno::EINVAL);
                };
                if semnum < 0 || semnum as usize >= set.values.len() {
                    return linux_errno(crate::modules::posix_consts::errno::EINVAL);
                }
                set.values[semnum as usize] = arg as i32;
                0
            }
            _ => linux_inval(),
        }
    }
    #[cfg(feature = "ipc_sysv_sem")]
    match crate::modules::ipc::semaphores::semctl(id, semnum, cmd, arg) {
        Ok(res) => res,
        Err(e) => linux_errno(e.code()),
    }
}
