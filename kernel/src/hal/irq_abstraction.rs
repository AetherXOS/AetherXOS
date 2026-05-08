//! Interrupt abstraction layer
//!
//! Provides unified interrupt handling across x86_64 (IDT/APIC) and aarch64 (GIC).

use super::abstractions::{InterruptModel, InitResult};

/// Generic interrupt handler function pointer
pub type InterruptHandler = extern "C" fn(irq: u32);

/// Unified interrupt controller trait
pub trait InterruptController: Send + Sync {
    /// Get the interrupt model this controller implements
    fn model(&self) -> InterruptModel;

    /// Initialize the interrupt controller
    fn init(&self) -> InitResult;

    /// Register an interrupt handler
    fn register_handler(&self, irq: u32, handler: InterruptHandler) -> InitResult;

    /// Enable an interrupt
    fn enable(&self, irq: u32) -> InitResult;

    /// Disable an interrupt
    fn disable(&self, irq: u32) -> InitResult;

    /// Acknowledge/clear an interrupt
    fn acknowledge(&self, irq: u32) -> InitResult;

    /// Get pending interrupt status
    fn is_pending(&self, irq: u32) -> bool;

    /// Get total number of supported interrupts
    fn max_irqs(&self) -> u32;

    /// Mask an interrupt
    fn mask(&self, irq: u32) -> InitResult {
        self.disable(irq)
    }

    /// Unmask an interrupt
    fn unmask(&self, irq: u32) -> InitResult {
        self.enable(irq)
    }

    /// Set interrupt priority
    fn set_priority(&self, irq: u32, priority: u8) -> InitResult {
        let _ = (irq, priority);
        InitResult::Unavailable // Many platforms don't support this
    }

    /// Enable all interrupts
    fn enable_all(&self) -> InitResult;

    /// Disable all interrupts
    fn disable_all(&self) -> InitResult;
}

/// x86_64-specific IDT wrapper (hides complex interrupt handling)
#[cfg(target_arch = "x86_64")]
pub struct X86IDTController;

#[cfg(target_arch = "x86_64")]
impl InterruptController for X86IDTController {
    fn model(&self) -> InterruptModel {
        InterruptModel::Apic
    }

    fn init(&self) -> InitResult {
        // Would initialize IDT and APIC
        InitResult::Success
    }

    fn register_handler(&self, _irq: u32, _handler: InterruptHandler) -> InitResult {
        // Would register into IDT
        InitResult::Success
    }

    fn enable(&self, _irq: u32) -> InitResult {
        // Would enable in APIC or PIC
        InitResult::Success
    }

    fn disable(&self, _irq: u32) -> InitResult {
        // Would disable in APIC or PIC
        InitResult::Success
    }

    fn acknowledge(&self, _irq: u32) -> InitResult {
        // Would send EOI to APIC
        InitResult::Success
    }

    fn is_pending(&self, _irq: u32) -> bool {
        false
    }

    fn max_irqs(&self) -> u32 {
        256
    }

    fn enable_all(&self) -> InitResult {
        InitResult::Success
    }

    fn disable_all(&self) -> InitResult {
        InitResult::Success
    }
}

/// aarch64-specific GIC wrapper (hides complex interrupt handling)
#[cfg(target_arch = "aarch64")]
pub struct AArch64GICController;

#[cfg(target_arch = "aarch64")]
impl InterruptController for AArch64GICController {
    fn model(&self) -> InterruptModel {
        InterruptModel::Gic
    }

    fn init(&self) -> InitResult {
        // Would initialize GIC distributor and CPU interface
        InitResult::Success
    }

    fn register_handler(&self, _irq: u32, _handler: InterruptHandler) -> InitResult {
        // Would register into GIC
        InitResult::Success
    }

    fn enable(&self, _irq: u32) -> InitResult {
        // Would enable in GIC distributor
        InitResult::Success
    }

    fn disable(&self, _irq: u32) -> InitResult {
        // Would disable in GIC distributor
        InitResult::Success
    }

    fn acknowledge(&self, _irq: u32) -> InitResult {
        // Would read GICC_IAR in GIC CPU interface
        InitResult::Success
    }

    fn is_pending(&self, _irq: u32) -> bool {
        false
    }

    fn max_irqs(&self) -> u32 {
        1024 // GIC supports up to 1024 SPIs
    }

    fn enable_all(&self) -> InitResult {
        InitResult::Success
    }

    fn disable_all(&self) -> InitResult {
        InitResult::Success
    }

    fn set_priority(&self, _irq: u32, _priority: u8) -> InitResult {
        // GIC does support priority levels
        InitResult::Success
    }
}

/// Exception/Fault abstraction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionType {
    /// Divide by zero
    DivideByZero,
    /// Page fault
    PageFault,
    /// General protection fault
    GeneralProtectionFault,
    /// Stack overflow
    StackOverflow,
    /// Invalid opcode
    InvalidOpcode,
    /// Double fault
    DoubleFault,
    /// Unknown exception
    Unknown,
}

/// Exception handler
pub type ExceptionHandler = fn(ex: ExceptionType, code: u64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_model_detection() {
        #[cfg(target_arch = "x86_64")]
        {
            let ctrl = X86IDTController;
            assert_eq!(ctrl.model(), InterruptModel::Apic);
        }

        #[cfg(target_arch = "aarch64")]
        {
            let ctrl = AArch64GICController;
            assert_eq!(ctrl.model(), InterruptModel::Gic);
        }
    }

    #[test]
    fn test_irq_count() {
        #[cfg(target_arch = "x86_64")]
        {
            let ctrl = X86IDTController;
            assert_eq!(ctrl.max_irqs(), 256);
        }

        #[cfg(target_arch = "aarch64")]
        {
            let ctrl = AArch64GICController;
            assert!(ctrl.max_irqs() >= 256);
        }
    }
}
