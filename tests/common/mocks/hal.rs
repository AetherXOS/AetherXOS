use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct MockHal {
    pub initialized: AtomicBool,
    pub interrupt_count: AtomicUsize,
    pub memory_regions: AtomicUsize,
}

impl MockHal {
    pub fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            interrupt_count: AtomicUsize::new(0),
            memory_regions: AtomicUsize::new(0),
        }
    }

    pub fn init(&self) -> Result<(), &'static str> {
        if self.initialized.swap(true, Ordering::SeqCst) {
            return Err("Already initialized");
        }
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    pub fn simulate_interrupt(&self) {
        self.interrupt_count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get_interrupt_count(&self) -> usize {
        self.interrupt_count.load(Ordering::SeqCst)
    }

    pub fn register_memory_region(&self) {
        self.memory_regions.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get_memory_region_count(&self) -> usize {
        self.memory_regions.load(Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.initialized.store(false, Ordering::SeqCst);
        self.interrupt_count.store(0, Ordering::SeqCst);
        self.memory_regions.store(0, Ordering::SeqCst);
    }
}

impl Default for MockHal {
    fn default() -> Self {
        Self::new()
    }
}
