use super::super::*;

#[cfg(not(feature = "ipc_sysv_msg"))]
use alloc::collections::{BTreeMap, VecDeque};
#[cfg(not(feature = "ipc_sysv_msg"))]
use alloc::vec::Vec;
#[cfg(not(feature = "ipc_sysv_msg"))]
use lazy_static::lazy_static;
#[cfg(not(feature = "ipc_sysv_msg"))]
use spin::Mutex;

#[cfg(not(feature = "ipc_sysv_msg"))]
const IPC_PRIVATE: i32 = 0;
#[cfg(not(feature = "ipc_sysv_msg"))]
const IPC_NOWAIT: i32 = 0o4000;
#[cfg(not(feature = "ipc_sysv_msg"))]
const MSG_NOERROR: i32 = 0o10000;

#[cfg(not(feature = "ipc_sysv_msg"))]
#[derive(Clone)]
struct CompatMsg {
    mtype: i64,
    data: Vec<u8>,
}

#[cfg(not(feature = "ipc_sysv_msg"))]
#[derive(Default)]
struct CompatQueue {
    entries: VecDeque<CompatMsg>,
}

#[cfg(not(feature = "ipc_sysv_msg"))]
struct CompatMsgState {
    next_id: i32,
    by_id: BTreeMap<i32, CompatQueue>,
    key_to_id: BTreeMap<i32, i32>,
}

#[cfg(not(feature = "ipc_sysv_msg"))]
impl CompatMsgState {
    fn new() -> Self {
        Self {
            next_id: 1,
            by_id: BTreeMap::new(),
            key_to_id: BTreeMap::new(),
        }
    }
}

#[cfg(not(feature = "ipc_sysv_msg"))]
lazy_static! {
    static ref COMPAT_SYSV_MSG: Mutex<CompatMsgState> = Mutex::new(CompatMsgState::new());
}

pub fn sys_linux_msgget(key: i32, msgflg: i32) -> usize {
    #[cfg(not(feature = "ipc_sysv_msg"))]
    {
        let _ = msgflg;
        let mut state = COMPAT_SYSV_MSG.lock();
        if key != IPC_PRIVATE {
            if let Some(id) = state.key_to_id.get(&key) {
                return *id as usize;
            }
        }
        let id = state.next_id;
        state.next_id = state.next_id.saturating_add(1);
        state.by_id.entry(id).or_default();
        if key != IPC_PRIVATE {
            state.key_to_id.insert(key, id);
        }
        return id as usize;
    }
    #[cfg(feature = "ipc_sysv_msg")]
    match crate::modules::ipc::sysv_msg::msgget(key, msgflg as u32) {
        Ok(id) => id as usize,
        Err(e) => linux_errno(e.code()),
    }
}

pub fn sys_linux_msgsnd(id: i32, msgp: UserPtr<u8>, msgsz: usize, msgflg: i32) -> usize {
    #[cfg(not(feature = "ipc_sysv_msg"))]
    {
        let _ = msgflg;
        let mut type_buf = [0u8; 8];
        if msgp.read_bytes(&mut type_buf).is_err() {
            return linux_fault();
        }
        let msg_type = i64::from_le_bytes(type_buf);
        if msg_type <= 0 {
            return linux_inval();
        }

        let mut data = alloc::vec![0u8; msgsz];
        if msgp.offset(8).read_bytes(&mut data).is_err() {
            return linux_fault();
        }

        let mut state = COMPAT_SYSV_MSG.lock();
        let Some(queue) = state.by_id.get_mut(&id) else {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        };
        queue.entries.push_back(CompatMsg {
            mtype: msg_type,
            data,
        });
        return 0;
    }
    #[cfg(feature = "ipc_sysv_msg")]
    {
        let mut type_buf = [0u8; 8];
        if msgp.read_bytes(&mut type_buf).is_err() {
            return linux_fault();
        }
        let msg_type = i64::from_le_bytes(type_buf);

        let mut data = alloc::vec![0u8; msgsz];
        if msgp.offset(8).read_bytes(&mut data).is_err() {
            return linux_fault();
        }

        match crate::modules::ipc::sysv_msg::msgsnd(id, msg_type, &data, msgflg) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    }
}

pub fn sys_linux_msgrcv(
    id: i32,
    msgp: UserPtr<u8>,
    msgsz: usize,
    msgtyp: i64,
    msgflg: i32,
) -> usize {
    #[cfg(not(feature = "ipc_sysv_msg"))]
    {
        let mut state = COMPAT_SYSV_MSG.lock();
        let Some(queue) = state.by_id.get_mut(&id) else {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        };

        let selected_idx = if msgtyp == 0 {
            if queue.entries.is_empty() {
                None
            } else {
                Some(0)
            }
        } else if msgtyp > 0 {
            queue.entries.iter().position(|m| m.mtype == msgtyp)
        } else {
            let max_type = -msgtyp;
            queue.entries.iter().position(|m| m.mtype <= max_type)
        };

        let Some(idx) = selected_idx else {
            if (msgflg & IPC_NOWAIT) != 0 {
                return linux_errno(crate::modules::posix_consts::errno::ENOMSG);
            }
            return linux_errno(crate::modules::posix_consts::errno::EAGAIN);
        };

        let msg = queue.entries.remove(idx).unwrap_or_else(|| CompatMsg {
            mtype: 1,
            data: Vec::new(),
        });

        if msg.data.len() > msgsz && (msgflg & MSG_NOERROR) == 0 {
            return linux_errno(crate::modules::posix_consts::errno::E2BIG);
        }

        let copy_len = core::cmp::min(msg.data.len(), msgsz);
        if msgp.write_bytes(&msg.mtype.to_le_bytes()).is_err() {
            return linux_fault();
        }
        if msgp.offset(8).write_bytes(&msg.data[..copy_len]).is_err() {
            return linux_fault();
        }
        return copy_len;
    }
    #[cfg(feature = "ipc_sysv_msg")]
    {
        let mut buf = alloc::vec![0u8; msgsz];
        match crate::modules::ipc::sysv_msg::msgrcv(id, &mut buf, msgtyp, msgflg) {
            Ok((len, msg_type)) => {
                let _ = msgp.write_bytes(&msg_type.to_le_bytes());
                let _ = msgp.offset(8).write_bytes(&buf[..len]);
                len
            }
            Err(e) => linux_errno(e.code()),
        }
    }
}
