use crate::interfaces::{KernelError, KernelResult};
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;
use lazy_static::lazy_static;

pub type MsgKey = i32;
pub type MsgId = i32;

pub const IPC_PRIVATE: MsgKey = 0;

#[derive(Debug, Clone)]
pub struct Message {
    pub msg_type: i64,
    pub data: Vec<u8>,
}

pub struct MsgQueue {
    pub id: MsgId,
    pub key: MsgKey,
    pub messages: VecDeque<Message>,
}

struct MsgState {
    queues: BTreeMap<MsgId, MsgQueue>,
    key_to_id: BTreeMap<MsgKey, MsgId>,
    next_id: i32,
}

lazy_static! {
    static ref MSG_MANAGER: IrqSafeMutex<MsgState> = IrqSafeMutex::new(MsgState {
        queues: BTreeMap::new(),
        key_to_id: BTreeMap::new(),
        next_id: 4000000,
    });
}

pub fn msgget(key: MsgKey, flags: u32) -> KernelResult<MsgId> {
    let mut state = MSG_MANAGER.lock();

    if key != IPC_PRIVATE {
        if let Some(&id) = state.key_to_id.get(&key) {
            return Ok(id);
        }
    }

    let id = state.next_id;
    state.next_id += 1;

    let queue = MsgQueue {
        id,
        key,
        messages: VecDeque::new(),
    };

    if key != IPC_PRIVATE {
        state.key_to_id.insert(key, id);
    }
    state.queues.insert(id, queue);

    Ok(id)
}

pub fn msgsnd(id: MsgId, msg_type: i64, data: &[u8], _flags: i32) -> KernelResult<()> {
    let mut state = MSG_MANAGER.lock();
    let q = state.queues.get_mut(&id).ok_or(KernelError::NotFound)?;

    q.messages.push_back(Message {
        msg_type,
        data: data.to_vec(),
    });
    Ok(())
}

pub fn msgrcv(
    id: MsgId,
    buffer: &mut [u8],
    wanted_type: i64,
    _flags: i32,
) -> KernelResult<(usize, i64)> {
    let mut state = MSG_MANAGER.lock();
    let q = state.queues.get_mut(&id).ok_or(KernelError::NotFound)?;

    // Find matching message
    let idx = q
        .messages
        .iter()
        .position(|m| wanted_type == 0 || m.msg_type == wanted_type);
    if let Some(i) = idx {
        let msg = q.messages.remove(i).unwrap();
        let len = core::cmp::min(buffer.len(), msg.data.len());
        buffer[..len].copy_from_slice(&msg.data[..len]);
        Ok((len, msg.msg_type))
    } else {
        Err(KernelError::Again)
    }
}
