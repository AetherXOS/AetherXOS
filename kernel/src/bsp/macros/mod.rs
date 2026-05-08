#[macro_export]
macro_rules! define_system_board {
    (
        $name:ident,
        mmio_base = $mmio:expr,
        irq_base = $irq:expr,
    ) => {
        pub struct $name;

        impl $name {
            /// MMIO base address for the board.
            pub const MMIO_BASE: usize = $mmio;

            /// IRQ base offset for the board.
            pub const IRQ_BASE: u32 = $irq;
        }
    };
}

pub use crate::define_system_board;
