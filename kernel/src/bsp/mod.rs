//! Board support package bridge.
//!
//! The board layer will eventually own the static wiring of the selected platform.
//! For now it preserves access to the existing boot/platform constants.

pub mod common {
    //! Reexports of current boot-time platform data.

    pub use crate::hal::common::boot::{acpi_rsdp_addr, dtb_addr, framebuffer, hhdm_offset, mem_map};
}

pub mod macros;