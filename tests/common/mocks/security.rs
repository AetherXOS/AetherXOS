use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct MockSecurityMonitor {
    pub enabled: AtomicBool,
    pub violation_count: AtomicUsize,
    pub access_denied_count: AtomicUsize,
    pub audit_log_count: AtomicUsize,
}

impl MockSecurityMonitor {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
            violation_count: AtomicUsize::new(0),
            access_denied_count: AtomicUsize::new(0),
            audit_log_count: AtomicUsize::new(0),
        }
    }

    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn check_access(&self, permitted: bool) -> Result<(), &'static str> {
        if !self.is_enabled() {
            return Ok(());
        }
        
        if permitted {
            self.audit_log_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        } else {
            self.access_denied_count.fetch_add(1, Ordering::SeqCst);
            Err("Access denied")
        }
    }

    pub fn record_violation(&self) {
        self.violation_count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get_violation_count(&self) -> usize {
        self.violation_count.load(Ordering::SeqCst)
    }

    pub fn get_access_denied_count(&self) -> usize {
        self.access_denied_count.load(Ordering::SeqCst)
    }

    pub fn get_audit_log_count(&self) -> usize {
        self.audit_log_count.load(Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.enabled.store(true, Ordering::SeqCst);
        self.violation_count.store(0, Ordering::SeqCst);
        self.access_denied_count.store(0, Ordering::SeqCst);
        self.audit_log_count.store(0, Ordering::SeqCst);
    }
}

impl Default for MockSecurityMonitor {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MockCapability {
    pub bits: u64,
}

impl MockCapability {
    pub const NONE: Self = Self { bits: 0 };
    
    pub const READ: Self = Self { bits: 1 << 0 };
    pub const WRITE: Self = Self { bits: 1 << 1 };
    pub const EXECUTE: Self = Self { bits: 1 << 2 };
    pub const ADMIN: Self = Self { bits: 1 << 3 };

    pub fn has(&self, other: MockCapability) -> bool {
        (self.bits & other.bits) == other.bits
    }

    pub fn add(&mut self, other: MockCapability) {
        self.bits |= other.bits;
    }

    pub fn remove(&mut self, other: MockCapability) {
        self.bits &= !other.bits;
    }
}
