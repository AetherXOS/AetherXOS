// --- PHASE 5: ADVANCED MEMORY EXTENSIONS ---
// Pressure handling, NUMA awareness, memory QoS

use crate::core::log;
use alloc::format;
use crate::interfaces::memory_ext::{
    MemoryPressure, MemoryPressureHandler, MemoryQoS, MemoryQoSManager, MemoryAccountant,
    NumaAwareAllocator, NumaNodeId, PageStats,
};
use alloc::collections::BTreeMap;
use crate::kernel::sync::IrqSafeMutex;
use core::sync::atomic::{AtomicU64, Ordering};

/// Memory pressure statistics
#[derive(Debug, Clone, Copy)]
pub struct PressureStats {
    pub total_pages: u64,
    pub free_pages: u64,
    pub cached_pages: u64,
    pub dirty_pages: u64,
}

impl PressureStats {
    /// Calculate current pressure level
    fn calculate_pressure(&self) -> MemoryPressure {
        if self.total_pages == 0 {
            return MemoryPressure::Low;
        }

        let free_ratio = self.free_pages as f64 / self.total_pages as f64;

        if free_ratio > 0.5 {
            MemoryPressure::Low
        } else if free_ratio > 0.25 {
            MemoryPressure::Medium
        } else if free_ratio > 0.05 {
            MemoryPressure::High
        } else {
            MemoryPressure::Critical
        }
    }
}

/// Concrete memory pressure handler
pub struct ConcreteMemoryPressureHandler {
    stats: IrqSafeMutex<PressureStats>,
    current_pressure: IrqSafeMutex<MemoryPressure>,
    callbacks: IrqSafeMutex<BTreeMap<u32, fn(MemoryPressure)>>,
    next_callback_id: IrqSafeMutex<u32>,
}

impl ConcreteMemoryPressureHandler {
    /// Create a new pressure handler
    pub const fn new() -> Self {
        Self {
            stats: IrqSafeMutex::new(PressureStats {
                total_pages: 1_000_000,
                free_pages: 500_000,
                cached_pages: 200_000,
                dirty_pages: 10_000,
            }),
            current_pressure: IrqSafeMutex::new(MemoryPressure::Low),
            callbacks: IrqSafeMutex::new(BTreeMap::new()),
            next_callback_id: IrqSafeMutex::new(1),
        }
    }

    /// Update memory stats (called by page allocator)
    pub fn update_stats(&self, total: u64, free: u64, cached: u64, dirty: u64) {
        let stats = PressureStats {
            total_pages: total,
            free_pages: free,
            cached_pages: cached,
            dirty_pages: dirty,
        };

        let new_pressure = stats.calculate_pressure();
        let old_pressure = *self.current_pressure.lock();

        *self.stats.lock() = stats;
        *self.current_pressure.lock() = new_pressure;

        // Trigger callbacks if pressure changed
        if new_pressure != old_pressure {
            log::info(&format!(
                "Memory pressure changed: {:?} -> {:?}",
                old_pressure, new_pressure
            ));
            self.notify_callbacks(new_pressure);
        }
    }

    pub fn register_pressure_callback(&self, callback: fn(MemoryPressure)) -> u32 {
        let mut next_id = self.next_callback_id.lock();
        let id = *next_id;
        self.callbacks.lock().insert(id, callback);
        *next_id += 1;
        id
    }

    fn notify_callbacks(&self, pressure: MemoryPressure) {
        for callback in self.callbacks.lock().values() {
            callback(pressure);
        }
    }

    pub fn notify_pressure(&self, pressure: MemoryPressure) {
        *self.current_pressure.lock() = pressure;
        self.notify_callbacks(pressure);
    }
}

impl MemoryPressureHandler for ConcreteMemoryPressureHandler {
    /// Called when pressure increases
    fn on_pressure_increased(&self, new_pressure: MemoryPressure) -> crate::interfaces::KernelResult<()> {
        log::warn(&format!("Memory pressure increased to: {:?}", new_pressure));

        match new_pressure {
            MemoryPressure::High => {
                // Trigger cache shrinking
                log::debug("Requesting cache shrink");
            }
            MemoryPressure::Critical => {
                // Emergency measures
                log::error("Critical memory pressure - emergency shrinking");
            }
            _ => {}
        }
        Ok(())
    }

    /// Called when pressure decreases
    fn on_pressure_decreased(&self, new_pressure: MemoryPressure) -> crate::interfaces::KernelResult<()> {
        log::info(&format!("Memory pressure decreased to: {:?}", new_pressure));
        Ok(())
    }

    /// Get current pressure level
    fn current_pressure(&self) -> MemoryPressure {
        *self.current_pressure.lock()
    }

    /// Attempt to shrink memory usage
    fn shrink_memory(&self, _target_pages: usize) -> crate::interfaces::KernelResult<usize> {
        // In real implementation: trigger cache/slab shrinking
        // For now: simulate freeing some pages
        let mut stats = self.stats.lock();
        let freed = stats.cached_pages.min(100);
        stats.free_pages += freed;
        stats.cached_pages -= freed;
        Ok(freed as usize)
    }
}

/// Concrete NUMA-aware allocator
pub struct ConcreteNumaAllocator {
    /// Per-NUMA-node free pages
    node_free_pages: IrqSafeMutex<BTreeMap<NumaNodeId, u64>>,

    /// Per-NUMA-node allocated pages
    node_allocated_pages: IrqSafeMutex<BTreeMap<NumaNodeId, u64>>,

    /// Local node for current CPU
    local_node: AtomicU64,
}

impl ConcreteNumaAllocator {
    /// Create a new NUMA allocator
    pub fn new(num_nodes: u32) -> Self {
        let mut free_pages = BTreeMap::new();
        for i in 0..num_nodes {
            free_pages.insert(NumaNodeId(i), 100_000);
        }

        Self {
            node_free_pages: IrqSafeMutex::new(free_pages),
            node_allocated_pages: IrqSafeMutex::new(BTreeMap::new()),
            local_node: AtomicU64::new(0),
        }
    }
}

impl NumaAwareAllocator for ConcreteNumaAllocator {
    /// Allocate memory on a specific NUMA node
    fn allocate_on_node(&self, size: usize, node: NumaNodeId) -> crate::interfaces::KernelResult<*mut u8> {
        let pages = (size + 4095) / 4096;
        let mut free = self.node_free_pages.lock();
        let mut alloc = self.node_allocated_pages.lock();

        if let Some(available) = free.get_mut(&node) {
            if *available >= pages as u64 {
                *available -= pages as u64;
                *alloc.entry(node).or_insert(0) += pages as u64;
                log::debug(&format!(
                    "Allocated {} bytes ({} pages) on node {:?}",
                    size, pages, node
                ));
                // Simulate returning a physical address as a pointer (not for actual use)
                Ok(0x1000000usize as *mut u8) 
            } else {
                Err(crate::interfaces::KernelError::NoMemory)
            }
        } else {
            Err(crate::interfaces::KernelError::NotFound)
        }
    }

    /// Migrate pages between NUMA nodes
    fn migrate_pages(&self, _ptr: *mut u8, _size: usize, _target_node: NumaNodeId) -> crate::interfaces::KernelResult<()> {
        // In real implementation: update page mappings
        log::debug("Page migration not yet implemented");
        Ok(())
    }

    /// Get NUMA node for a physical address
    fn get_node_for_address(&self, _ptr: *const u8) -> crate::interfaces::KernelResult<NumaNodeId> {
        // In real implementation: consult NUMA memory map
        Ok(NumaNodeId(0))
    }

    /// Get list of available NUMA nodes
    fn available_nodes(&self) -> alloc::vec::Vec<NumaNodeId> {
        self.node_free_pages.lock().keys().copied().collect()
    }

    /// Get local NUMA node for current CPU
    fn local_node(&self) -> NumaNodeId {
        NumaNodeId(self.local_node.load(Ordering::Relaxed) as u32)
    }
}

/// Concrete memory accountant for quotas
pub struct ConcreteMemoryAccountant {
    /// Per-process memory usage
    usage: IrqSafeMutex<BTreeMap<u32, u64>>,

    /// Per-process memory limits
    limits: IrqSafeMutex<BTreeMap<u32, u64>>,
}

impl ConcreteMemoryAccountant {
    /// Create a new memory accountant
    pub const fn new() -> Self {
        Self {
            usage: IrqSafeMutex::new(BTreeMap::new()),
            limits: IrqSafeMutex::new(BTreeMap::new()),
        }
    }

    pub fn get_usage(&self, process_id: u32) -> u64 {
        self.usage.lock().get(&process_id).copied().unwrap_or(0)
    }
}

impl MemoryAccountant for ConcreteMemoryAccountant {
    /// Get page statistics
    fn page_stats(&self) -> PageStats {
        PageStats {
            total_pages: 1_000_000,
            free_pages: 500_000,
            allocated_pages: 300_000,
            cached_pages: 200_000,
            dirty_pages: 10_000,
        }
    }

    /// Set memory limit for process
    fn set_memory_limit(&self, process_id: u32, limit_bytes: u64) -> crate::interfaces::KernelResult<()> {
        self.limits
            .lock()
            .insert(process_id, limit_bytes);
        log::debug(&format!(
            "Process {} memory limit set to {} bytes",
            process_id, limit_bytes
        ));
        Ok(())
    }

    /// Get current memory usage of process
    fn get_memory_usage(&self, process_id: u32) -> crate::interfaces::KernelResult<u64> {
        self.usage.lock().get(&process_id).copied().ok_or(crate::interfaces::KernelError::NotFound)
    }

    /// Check if process can allocate more memory
    fn can_allocate(&self, process_id: u32, size_bytes: u64) -> bool {
        let current = self.usage.lock().get(&process_id).copied().unwrap_or(0);
        let limit = self
            .limits
            .lock()
            .get(&process_id)
            .copied()
            .unwrap_or(u64::MAX);

        current + size_bytes <= limit
    }

    /// Record memory allocation
    fn record_allocation(&self, process_id: u32, size_bytes: u64) -> crate::interfaces::KernelResult<()> {
        if !self.can_allocate(process_id, size_bytes) {
            return Err(crate::interfaces::KernelError::NoMemory);
        }

        *self
            .usage
            .lock()
            .entry(process_id)
            .or_insert(0) += size_bytes;
        Ok(())
    }

    /// Record memory deallocation
    fn record_deallocation(&self, process_id: u32, size_bytes: u64) {
        if let Some(usage) = self.usage.lock().get_mut(&process_id) {
            *usage = usage.saturating_sub(size_bytes);
        }
    }
}

/// Concrete memory QoS manager
pub struct ConcreteMemoryQoSManager {
    /// Per-process QoS level
    qos: IrqSafeMutex<BTreeMap<u32, MemoryQoS>>,
}

impl ConcreteMemoryQoSManager {
    /// Create a new QoS manager
    pub const fn new() -> Self {
        Self {
            qos: IrqSafeMutex::new(BTreeMap::new()),
        }
    }
}

impl MemoryQoSManager for ConcreteMemoryQoSManager {
    /// Allocate memory with QoS tier
    fn allocate_with_qos(&self, size: usize, _qos: MemoryQoS) -> crate::interfaces::KernelResult<*mut u8> {
        log::debug(&format!(
            "Allocating {} bytes with QoS",
            size
        ));
        // Simulate allocation
        Ok(0x2000000usize as *mut u8)
    }

    /// Set QoS tier for process
    fn set_process_qos(&self, process_id: u32, qos: MemoryQoS) -> crate::interfaces::KernelResult<()> {
        self.qos.lock().insert(process_id, qos);
        log::debug(&format!("Process {} QoS set to: {:?}", process_id, qos));
        Ok(())
    }

    /// Get current QoS tier for process
    fn get_process_qos(&self, process_id: u32) -> crate::interfaces::KernelResult<MemoryQoS> {
        self.qos.lock().get(&process_id).copied().ok_or(crate::interfaces::KernelError::NotFound)
    }
}

// Global instances
pub static PRESSURE_HANDLER: ConcreteMemoryPressureHandler = ConcreteMemoryPressureHandler::new();
lazy_static::lazy_static! {
    pub static ref NUMA_ALLOCATOR: ConcreteNumaAllocator = ConcreteNumaAllocator::new(4);
}
pub static MEMORY_ACCOUNTANT: ConcreteMemoryAccountant = ConcreteMemoryAccountant::new();
pub static QOS_MANAGER: ConcreteMemoryQoSManager = ConcreteMemoryQoSManager::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pressure_calculation() {
        let stats = PressureStats {
            total_pages: 1000,
            free_pages: 600,
            cached_pages: 200,
            dirty_pages: 50,
        };
        assert_eq!(stats.calculate_pressure(), MemoryPressure::Low);

        let stats = PressureStats {
            total_pages: 1000,
            free_pages: 200,
            cached_pages: 200,
            dirty_pages: 50,
        };
        assert_eq!(stats.calculate_pressure(), MemoryPressure::High);
    }

    #[test]
    fn test_pressure_handler_creation() {
        let handler = ConcreteMemoryPressureHandler::new();
        assert_eq!(handler.current_pressure(), MemoryPressure::Low);
    }

    #[test]
    fn test_pressure_callback_registration() {
        let handler = ConcreteMemoryPressureHandler::new();

        static CALLBACK_CALLED: AtomicU64 = AtomicU64::new(0);
        fn test_callback(_pressure: MemoryPressure) {
            CALLBACK_CALLED.store(1, Ordering::Relaxed);
        }

        handler.register_pressure_callback(test_callback);
        handler.update_stats(1000, 100, 100, 50);

        assert_eq!(CALLBACK_CALLED.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_numa_allocator_allocation() {
        let alloc = ConcreteNumaAllocator::new(2);
        let node0 = NumaNodeId(0);

        assert!(alloc.allocate_on_node(4096, node0).is_ok());
        assert!(alloc.allocate_on_node(1_000_000_000, node0).is_err()); // Exceed available
    }

    #[test]
    fn test_memory_accountant_limits() {
        let acct = ConcreteMemoryAccountant::new();
        let pid = 123;

        acct.set_memory_limit(pid, 1_000_000).ok();
        assert!(acct.can_allocate(pid, 500_000));
        assert!(!acct.can_allocate(pid, 600_000 * 2)); // Fixed to exceed
    }

    #[test]
    fn test_memory_accountant_tracking() {
        let acct = ConcreteMemoryAccountant::new();
        let pid = 123;

        acct.set_memory_limit(pid, 1_000_000).ok();
        acct.record_allocation(pid, 100_000).ok();

        assert_eq!(acct.get_memory_usage(pid).unwrap(), 100_000);

        acct.record_deallocation(pid, 50_000);
        assert_eq!(acct.get_memory_usage(pid).unwrap(), 50_000);
    }

    #[test]
    fn test_qos_manager() {
        let mgr = ConcreteMemoryQoSManager::new();
        let pid = 123;

        mgr.set_process_qos(pid, MemoryQoS::Interactive).ok();
        assert_eq!(
            mgr.get_process_qos(pid).unwrap(),
            MemoryQoS::Interactive
        );
    }

    #[test]
    fn test_shrink_memory() {
        let handler = ConcreteMemoryPressureHandler::new();
        let freed = handler.shrink_memory(1000).unwrap();
        assert!(freed > 0);
    }
}
