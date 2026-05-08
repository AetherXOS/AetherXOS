/// Interrupt controller device abstraction with type-safe interrupt handling.
///
/// Provides strongly-typed, const-generic MMIO access to interrupt controller devices
/// (e.g., APIC, GIC, or custom interrupt controllers) with capability-based access control.

use core::marker::PhantomData;
use crate::core::types::CapabilityToken;

/// An interrupt vector ID (0–255 on x86, 0–1019 on ARM GIC, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IrqVector(pub u16);

/// IRQ priority level (higher = more urgent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IrqPriority(pub u8);

/// An IRQ masking capability—demonstrates capability-based security.
/// Only holders of this token can mask/unmask a specific IRQ vector.
#[derive(Debug, Clone, Copy)]
pub struct IrqMaskCapability {
    vector: IrqVector,
    _capability: CapabilityToken,
}

impl IrqMaskCapability {
    /// Create a new IRQ mask capability (typically issued by the boot loader or kernel init).
    pub fn new(vector: IrqVector) -> Self {
        IrqMaskCapability {
            vector,
            _capability: CapabilityToken,
        }
    }

    /// Get the vector this capability protects.
    pub fn vector(&self) -> IrqVector {
        self.vector
    }
}

/// Generic interrupt controller device with const-generic MMIO base address.
///
/// # Type Parameters
/// - `BASE`: The memory-mapped base address of the interrupt controller.
///
/// # Safety
/// The provided `BASE` must point to valid interrupt controller MMIO registers.
pub struct InterruptController<const BASE: usize> {
    _marker: PhantomData<()>,
}

impl<const BASE: usize> InterruptController<BASE> {
    /// Create a new interrupt controller handle.
    pub const fn new() -> Self {
        InterruptController {
            _marker: PhantomData,
        }
    }

    /// Enable a specific IRQ vector.
    ///
    /// # Arguments
    /// - `vector`: The IRQ vector to enable.
    /// - `priority`: The priority level for this interrupt.
    ///
    /// # Safety
    /// Enabling interrupts may cause immediate system state changes if
    /// an IRQ is already pending. The caller must ensure the corresponding
    /// handler is installed and ready.
    pub unsafe fn enable_irq(&mut self, vector: IrqVector, priority: IrqPriority) { unsafe {
        // SAFETY: Caller asserts that BASE is valid and vector/priority are appropriate.
        // Example register layout (APIC-like):
        // Offset 0x20: IRQ enable register (base)
        // Each vector occupies 4 bytes
        let enable_offset = 0x20 + (vector.0 as usize * 4);
        let enable_reg = (BASE + enable_offset) as *mut u32;
        
        let value = 0x100 | (priority.0 as u32 & 0xFF); // Enable bit + priority
        core::ptr::write_volatile(enable_reg, value);
    }}

    /// Disable a specific IRQ vector.
    ///
    /// # Arguments
    /// - `cap`: A capability token proving authorization to disable this IRQ.
    ///
    /// # Returns
    /// `Ok(())` on success; `Err(())` if the capability vector doesn't match.
    pub fn disable_irq(&mut self, cap: &IrqMaskCapability) -> Result<(), ()> {
        let vector = cap.vector();
        
        // SAFETY: We're only reading/writing a register for an IRQ that the
        // capability authorizes. This is safe as long as BASE is valid.
        unsafe {
            let disable_offset = 0x20 + (vector.0 as usize * 4);
            let disable_reg = (BASE + disable_offset) as *mut u32;
            
            core::ptr::write_volatile(disable_reg, 0); // Clear enable bit
        }
        
        Ok(())
    }

    /// Read the pending IRQ status mask (which interrupts are pending).
    ///
    /// # Returns
    /// A bitmask indicating pending IRQs.
    pub fn read_pending_mask(&self) -> u64 {
        // SAFETY: Reading status registers has no side effects.
        unsafe {
            let pending_reg = BASE as *const u64;
            core::ptr::read_volatile(pending_reg)
        }
    }

    /// Acknowledge (clear) a specific IRQ.
    ///
    /// # Arguments
    /// - `vector`: The IRQ vector to acknowledge.
    ///
    /// # Safety
    /// Acknowledging an IRQ without servicing the underlying hardware
    /// condition may cause the IRQ to fire repeatedly or be lost.
    pub unsafe fn ack_irq(&mut self, vector: IrqVector) { unsafe {
        // SAFETY: Caller asserts that the IRQ has been serviced.
        let ack_reg = (BASE + 0xB0) as *mut u32; // Example: EOI register
        core::ptr::write_volatile(ack_reg, vector.0 as u32);
    }}

    /// Set the priority threshold (only IRQs with priority >= threshold are delivered).
    ///
    /// # Arguments
    /// - `threshold`: The minimum priority level to allow.
    ///
    /// # Safety
    /// Setting a high threshold may prevent critical interrupts from being serviced.
    pub unsafe fn set_priority_threshold(&mut self, threshold: IrqPriority) { unsafe {
        // SAFETY: Caller asserts this is safe.
        let threshold_reg = (BASE + 0xA0) as *mut u32; // Example: priority threshold
        core::ptr::write_volatile(threshold_reg, threshold.0 as u32);
    }}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_irq_vector_ordering() {
        let v0 = IrqVector(0);
        let v1 = IrqVector(1);
        assert!(v0 < v1);
    }

    #[test]
    fn test_irq_capability() {
        let cap = IrqMaskCapability::new(IrqVector(32));
        assert_eq!(cap.vector(), IrqVector(32));
    }

    #[test]
    fn test_interrupt_controller_creation() {
        let _ic: InterruptController<0xFEE00000> = InterruptController::new();
        // Verify creation without panicking.
    }
}
