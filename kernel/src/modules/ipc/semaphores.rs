use crate::interfaces::{KernelError, KernelResult};
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use lazy_static::lazy_static;

pub type SemKey = i32;
pub type SemId = i32;

pub const IPC_PRIVATE: SemKey = 0;

#[derive(Debug, Clone)]
pub struct SemSet {
    pub id: SemId,
    pub key: SemKey,
    pub values: Vec<i16>,
}

struct SemState {
    sets: BTreeMap<SemId, SemSet>,
    key_to_id: BTreeMap<SemKey, SemId>,
    next_id: i32,
}

lazy_static! {
    static ref SEM_MANAGER: IrqSafeMutex<SemState> = IrqSafeMutex::new(SemState {
        sets: BTreeMap::new(),
        key_to_id: BTreeMap::new(),
        next_id: 3000000,
    });
}

pub fn semget(key: SemKey, nsems: i32, flags: u32) -> KernelResult<SemId> {
    let mut state = SEM_MANAGER.lock();

    if key != IPC_PRIVATE {
        if let Some(&id) = state.key_to_id.get(&key) {
            return Ok(id);
        }
    }

    let id = state.next_id;
    state.next_id += 1;

    let set = SemSet {
        id,
        key,
        values: alloc::vec![0; nsems as usize],
    };

    if key != IPC_PRIVATE {
        state.key_to_id.insert(key, id);
    }
    state.sets.insert(id, set);

    Ok(id)
}

pub fn semop(id: SemId, ops: &[(u16, i16, i16)]) -> KernelResult<()> {
    let mut state = SEM_MANAGER.lock();
    let set = state.sets.get_mut(&id).ok_or(KernelError::NotFound)?;

    for &(num, op, _flags) in ops {
        if num as usize >= set.values.len() {
            return Err(KernelError::Invalid);
        }
        // Simple synchronous op
        set.values[num as usize] = (set.values[num as usize] as i32 + op as i32) as i16;
    }
    Ok(())
}

pub fn semctl(id: SemId, semnum: i32, cmd: i32, arg: usize) -> KernelResult<usize> {
    let mut state = SEM_MANAGER.lock();
    let set = state.sets.get_mut(&id).ok_or(KernelError::NotFound)?;

    match cmd {
        0 => Ok(0), // IPC_RMID (hypothetical)
        _ => Err(KernelError::NotSupported),
    }
}
