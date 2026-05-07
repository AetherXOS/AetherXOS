use alloc::string::String;
use alloc::collections::BTreeMap;
use spin::Mutex;

/// AetherXOS Kernel Symbol Table (KSymTab).
/// Maps function names to their actual physical/virtual addresses in the kernel.
pub struct SymbolTable {
    symbols: BTreeMap<String, u64>,
}

impl SymbolTable {
    pub const fn new() -> Self {
        Self {
            symbols: BTreeMap::new(),
        }
    }

    /// Register a kernel symbol (called during kernel boot).
    pub fn register(&mut self, name: &str, addr: u64) {
        self.symbols.insert(String::from(name), addr);
    }

    /// Resolve a symbol name to its address.
    pub fn resolve(&self, name: &str) -> Option<u64> {
        self.symbols.get(name).copied()
    }
}

/// Global Kernel Symbol Table.
pub static KSYMTAB: Mutex<SymbolTable> = Mutex::new(SymbolTable::new());

/// Initialize basic kernel symbols for dynamic modules.
pub fn init_ksymtab() {
    let mut syms = KSYMTAB.lock();
    // In a real system, these would be extracted from the kernel ELF at boot
    syms.register("klog_info", 0xFFFF_FFFF_8010_0000);
    syms.register("kalloc", 0xFFFF_FFFF_8010_1000);
    syms.register("kfree", 0xFFFF_FFFF_8010_2000);
    syms.register("sched_yield", 0xFFFF_FFFF_8010_3000);
    crate::klog_info!("[KSYMTAB] Initialized with core kernel symbols");
}
