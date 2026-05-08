/// Memory management extension interfaces.

use crate::interfaces::KernelResult;

/// NUMA node identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NumaNodeId(pub u32);

/// Memory pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryPressure {
    Low,
    Medium,
    High,
    Critical,
}

/// Memory page statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct PageStats {
    pub total_pages: usize,
    pub free_pages: usize,
    pub allocated_pages: usize,
    pub cached_pages: usize,
    pub dirty_pages: usize,
}

/// Trait for NUMA-aware memory allocation
pub trait NumaAwareAllocator {
    fn allocate_on_node(&self, size: usize, node: NumaNodeId) -> KernelResult<*mut u8>;
    fn migrate_pages(&self, ptr: *mut u8, size: usize, target_node: NumaNodeId) -> KernelResult<()>;
    fn get_node_for_address(&self, ptr: *const u8) -> KernelResult<NumaNodeId>;
    fn available_nodes(&self) -> alloc::vec::Vec<NumaNodeId>;
    fn local_node(&self) -> NumaNodeId;
}

/// Trait for memory pressure notification
pub trait MemoryPressureHandler {
    fn on_pressure_increased(&self, new_pressure: MemoryPressure) -> KernelResult<()>;
    fn on_pressure_decreased(&self, new_pressure: MemoryPressure) -> KernelResult<()>;
    fn current_pressure(&self) -> MemoryPressure;
    fn shrink_memory(&self, target_pages: usize) -> KernelResult<usize>;
}

/// Trait for memory accounting and limits
pub trait MemoryAccountant {
    fn page_stats(&self) -> PageStats;
    fn set_memory_limit(&self, pid: u32, limit_bytes: u64) -> KernelResult<()>;
    fn get_memory_usage(&self, pid: u32) -> KernelResult<u64>;
    fn can_allocate(&self, pid: u32, size_bytes: u64) -> bool;
    fn record_allocation(&self, pid: u32, size_bytes: u64) -> KernelResult<()>;
    fn record_deallocation(&self, pid: u32, size_bytes: u64);
}

/// QoS level for memory allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryQoS {
    KernelCritical,
    RealTime,
    Interactive,
    Background,
}

/// Trait for memory QoS management
pub trait MemoryQoSManager {
    fn allocate_with_qos(&self, size: usize, qos: MemoryQoS) -> KernelResult<*mut u8>;
    fn set_process_qos(&self, pid: u32, qos: MemoryQoS) -> KernelResult<()>;
    fn get_process_qos(&self, pid: u32) -> KernelResult<MemoryQoS>;
}
