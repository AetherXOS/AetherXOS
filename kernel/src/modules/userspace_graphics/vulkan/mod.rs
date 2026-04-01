use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static VULKAN_SWAPCHAIN_PATH_READY: AtomicBool = AtomicBool::new(false);
static VULKAN_API_VERSION: AtomicU32 = AtomicU32::new(0);
static VULKAN_QUEUE_MASK: AtomicU32 = AtomicU32::new(0);

pub const VULKAN_QUEUE_GRAPHICS: u32 = 1 << 0;
pub const VULKAN_QUEUE_COMPUTE: u32 = 1 << 1;
pub const VULKAN_QUEUE_TRANSFER: u32 = 1 << 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VulkanRuntimeSnapshot {
    pub swapchain_ready: bool,
    pub api_version: u32,
    pub queue_mask: u32,
    pub driver_accel_ready: bool,
}

pub fn mark_vulkan_swapchain_path_ready() {
    VULKAN_SWAPCHAIN_PATH_READY.store(true, Ordering::Relaxed);
}

pub fn register_vulkan_runtime(api_version: u32, queue_mask: u32) {
    VULKAN_API_VERSION.store(api_version, Ordering::Relaxed);
    VULKAN_QUEUE_MASK.store(queue_mask, Ordering::Relaxed);
}

pub fn is_vulkan_swapchain_path_ready() -> bool {
    VULKAN_SWAPCHAIN_PATH_READY.load(Ordering::Relaxed)
}

fn has_required_queues(mask: u32) -> bool {
    let required = VULKAN_QUEUE_GRAPHICS | VULKAN_QUEUE_TRANSFER;
    (mask & required) == required
}

fn api_version_meets_minimum(version: u32) -> bool {
    // VK_API_VERSION_1_1 equivalent threshold.
    version >= 0x00401000
}

fn driver_accel_ready() -> bool {
    #[cfg(feature = "drivers")]
    {
        let snapshot = crate::modules::drivers::gpu::gpu_stack_snapshot();
        snapshot.desktop_session_ready
            && !matches!(snapshot.backend, crate::modules::drivers::gpu::GpuBackend::None)
    }
    #[cfg(not(feature = "drivers"))]
    {
        true
    }
}

pub fn vulkan_runtime_snapshot() -> VulkanRuntimeSnapshot {
    VulkanRuntimeSnapshot {
        swapchain_ready: is_vulkan_swapchain_path_ready(),
        api_version: VULKAN_API_VERSION.load(Ordering::Relaxed),
        queue_mask: VULKAN_QUEUE_MASK.load(Ordering::Relaxed),
        driver_accel_ready: driver_accel_ready(),
    }
}

pub fn vulkan_runtime_contract_supported() -> bool {
    let snapshot = vulkan_runtime_snapshot();
    snapshot.swapchain_ready
        && snapshot.driver_accel_ready
        && api_version_meets_minimum(snapshot.api_version)
        && has_required_queues(snapshot.queue_mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn vulkan_contract_requires_swapchain_and_queues() {
        register_vulkan_runtime(0x00401000, VULKAN_QUEUE_GRAPHICS);
        assert!(!vulkan_runtime_contract_supported());

        mark_vulkan_swapchain_path_ready();
        register_vulkan_runtime(
            0x00401000,
            VULKAN_QUEUE_GRAPHICS | VULKAN_QUEUE_TRANSFER | VULKAN_QUEUE_COMPUTE,
        );
        assert!(vulkan_runtime_contract_supported());
    }
}
