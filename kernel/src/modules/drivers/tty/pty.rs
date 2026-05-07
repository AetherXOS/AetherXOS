//! Pseudo-Terminal (PTY) Subsystem for AetherXOS.
//! Implements /dev/ptmx and /dev/pts/* hierarchy for POSIX shell support.

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use spin::Mutex;

pub struct PtyPair {
    pub master: Arc<Mutex<PtyEndpoint>>,
    pub slave: Arc<Mutex<PtyEndpoint>>,
}

pub struct PtyEndpoint {
    pub buffer: crate::kernel::sync::RingBuffer<u8>,
    pub winsize: WindowSize,
}

#[derive(Default, Clone, Copy)]
pub struct WindowSize {
    pub ws_row: u16,
    pub ws_col: u16,
}

pub struct PtyManager {
    pairs: BTreeMap<u32, Arc<PtyPair>>,
    next_id: u32,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            pairs: BTreeMap::new(),
            next_id: 0,
        }
    }

    pub fn create_pair(&mut self) -> (u32, Arc<PtyPair>) {
        let id = self.next_id;
        self.next_id += 1;
        
        let master = Arc::new(Mutex::new(PtyEndpoint {
            buffer: crate::kernel::sync::RingBuffer::new(4096),
            winsize: WindowSize::default(),
        }));
        let slave = Arc::new(Mutex::new(PtyEndpoint {
            buffer: crate::kernel::sync::RingBuffer::new(4096),
            winsize: WindowSize::default(),
        }));
        
        let pair = Arc::new(PtyPair { master, slave });
        self.pairs.insert(id, pair.clone());
        (id, pair)
    }
}

pub static PTY_MANAGER: Mutex<PtyManager> = Mutex::new(PtyManager {
    pairs: BTreeMap::new(),
    next_id: 0,
});
