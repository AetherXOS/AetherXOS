use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicI32, AtomicU64};
use lazy_static::lazy_static;
use crate::kernel::sync::IrqSafeMutex;
use super::set::NsSet;

pub static NEXT_NSSET_ID: AtomicU64 = AtomicU64::new(1);
pub static NEXT_NSFD: AtomicI32 = AtomicI32::new(1000);

lazy_static! {
    pub static ref NSSET_TABLE: IrqSafeMutex<BTreeMap<u32, NsSet>> = {
        let mut map = BTreeMap::new();
        map.insert(0, NsSet::init_root());
        IrqSafeMutex::new(map)
    };
    /// Maps namespace file-descriptor numbers to their `ns_set_id`.
    pub static ref NSFD_TABLE: IrqSafeMutex<BTreeMap<i32, u32>> =
        IrqSafeMutex::new(BTreeMap::new());
}
