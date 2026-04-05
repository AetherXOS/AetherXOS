use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NtSymbol {
    IoCreateDevice,
    IoDeleteDevice,
    IoCallDriver,
    IoCompleteRequest,
    KeAcquireSpinLock,
    KeReleaseSpinLock,
    ExAllocatePool2,
    ExFreePool,
    MmMapIoSpace,
    MmUnmapIoSpace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NtSymbolBinding {
    pub symbol: NtSymbol,
    pub address: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NtSymbolTable {
    pub bindings: Vec<NtSymbolBinding>,
}

impl NtSymbolTable {
    pub fn new() -> Self {
        Self { bindings: Vec::new() }
    }

    pub fn register(&mut self, symbol: NtSymbol, address: usize) {
        self.bindings.push(NtSymbolBinding { symbol, address });
    }

    pub fn resolve(&self, symbol: NtSymbol) -> Option<usize> {
        self.bindings
            .iter()
            .find(|binding| binding.symbol == symbol)
            .map(|binding| binding.address)
    }
}

impl Default for NtSymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Irql {
    Passive = 0,
    Apc = 1,
    Dispatch = 2,
    Device = 3,
    High = 4,
}

#[derive(Debug)]
pub struct NtIrqlGuard {
    previous: Irql,
    current: Irql,
}

impl NtIrqlGuard {
    pub fn raise(previous: Irql, to: Irql) -> Self {
        Self {
            previous,
            current: to,
        }
    }

    pub fn current(&self) -> Irql {
        self.current
    }

    pub fn previous(&self) -> Irql {
        self.previous
    }
}

#[derive(Debug)]
pub struct NtSpinLock {
    held: AtomicBool,
}

impl NtSpinLock {
    pub const fn new() -> Self {
        Self {
            held: AtomicBool::new(false),
        }
    }

    pub fn try_lock(&self) -> bool {
        self.held
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    pub fn unlock(&self) {
        self.held.store(false, Ordering::Release);
    }

    pub fn is_locked(&self) -> bool {
        self.held.load(Ordering::Relaxed)
    }
}
