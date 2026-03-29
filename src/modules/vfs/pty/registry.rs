use super::PtyPair;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use spin::Mutex;

static PTY_REGISTRY: Mutex<Option<PtyRegistry>> = Mutex::new(None);

struct PtyRegistry {
    next_index: u32,
    pairs: BTreeMap<u32, PtyPair>,
}

impl PtyRegistry {
    fn new() -> Self {
        Self {
            next_index: 0,
            pairs: BTreeMap::new(),
        }
    }
}

/// Initialize the PTY subsystem. Call during kernel init.
pub fn init_pty_subsystem() {
    let mut reg = PTY_REGISTRY.lock();
    if reg.is_none() {
        *reg = Some(PtyRegistry::new());
    }
}

pub(super) fn allocate_pty() -> Option<(u32, PtyPair)> {
    let mut reg = PTY_REGISTRY.lock();
    let registry = reg.as_mut()?;
    let index = registry.next_index;
    registry.next_index = registry.next_index.checked_add(1)?;
    let pair = PtyPair::new(index);
    registry.pairs.insert(index, pair.clone());
    Some((index, pair))
}

pub(super) fn get_pty_pair(index: u32) -> Option<PtyPair> {
    let reg = PTY_REGISTRY.lock();
    reg.as_ref()?.pairs.get(&index).cloned()
}

pub(super) fn remove_pty(index: u32) {
    let mut reg = PTY_REGISTRY.lock();
    if let Some(registry) = reg.as_mut() {
        registry.pairs.remove(&index);
    }
}

pub(super) fn list_ptys() -> Vec<u32> {
    let reg = PTY_REGISTRY.lock();
    match reg.as_ref() {
        Some(registry) => registry.pairs.keys().cloned().collect(),
        None => Vec::new(),
    }
}
