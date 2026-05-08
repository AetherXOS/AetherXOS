//! AetherView: Extension Trait System for Zero-Overhead Data Access.
//! Provides macros to inject strongly-typed methods directly into byte slices.

use super::{DataCursor, DataWriter};

pub trait PacketView {
    fn view_at(&self, offset: usize) -> &[u8];
    fn view_at_mut(&mut self, offset: usize) -> &mut [u8];
    fn cursor(&self) -> DataCursor<'_>;
    fn writer(&mut self) -> DataWriter<'_>;
}

impl PacketView for [u8] {
    #[inline(always)]
    fn view_at(&self, offset: usize) -> &[u8] {
        &self[offset..]
    }

    #[inline(always)]
    fn view_at_mut(&mut self, offset: usize) -> &mut [u8] {
        &mut self[offset..]
    }

    #[inline(always)]
    fn cursor(&self) -> DataCursor<'_> {
        DataCursor::new(self)
    }

    #[inline(always)]
    fn writer(&mut self) -> DataWriter<'_> {
        DataWriter::new(self)
    }
}



#[macro_export]
macro_rules! define_view {
    ($trait_name:ident) => {
        pub trait $trait_name {
            fn view_at(&self, offset: usize) -> &[u8];
        }

        impl $trait_name for [u8] {
            #[inline(always)]
            fn view_at(&self, offset: usize) -> &[u8] {
                &self[offset..]
            }
        }
    };
}
