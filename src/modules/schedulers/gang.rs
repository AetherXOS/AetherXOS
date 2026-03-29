use crate::interfaces::task::TaskId;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

static GANG_CREATE_CALLS: AtomicU64 = AtomicU64::new(0);
static GANG_ASSIGN_CALLS: AtomicU64 = AtomicU64::new(0);
static GANG_PICK_CALLS: AtomicU64 = AtomicU64::new(0);
static GANG_NEXT_ID: AtomicU64 = AtomicU64::new(1);

lazy_static! {
    static ref GANGS: Mutex<BTreeMap<u64, Vec<TaskId>>> = Mutex::new(BTreeMap::new());
}

#[derive(Debug, Clone, Copy)]
pub struct GangStats {
    pub create_calls: u64,
    pub assign_calls: u64,
    pub pick_calls: u64,
    pub active_gangs: usize,
}

pub fn gang_create() -> u64 {
    GANG_CREATE_CALLS.fetch_add(1, Ordering::Relaxed);
    let id = GANG_NEXT_ID.fetch_add(1, Ordering::Relaxed);
    GANGS.lock().insert(id, Vec::new());
    id
}

pub fn gang_assign_task(gang_id: u64, task_id: TaskId) -> Result<(), &'static str> {
    GANG_ASSIGN_CALLS.fetch_add(1, Ordering::Relaxed);
    let mut gangs = GANGS.lock();
    let members = gangs.get_mut(&gang_id).ok_or("gang not found")?;
    if !members.contains(&task_id) {
        members.push(task_id);
    }
    Ok(())
}

pub fn gang_pick_members(gang_id: u64, max_members: usize) -> Result<Vec<TaskId>, &'static str> {
    GANG_PICK_CALLS.fetch_add(1, Ordering::Relaxed);
    let gangs = GANGS.lock();
    let members = gangs.get(&gang_id).ok_or("gang not found")?;
    let take = core::cmp::min(max_members, members.len());
    Ok(members[..take].to_vec())
}

pub fn gang_stats() -> GangStats {
    GangStats {
        create_calls: GANG_CREATE_CALLS.load(Ordering::Relaxed),
        assign_calls: GANG_ASSIGN_CALLS.load(Ordering::Relaxed),
        pick_calls: GANG_PICK_CALLS.load(Ordering::Relaxed),
        active_gangs: GANGS.lock().len(),
    }
}
