use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ProcessLifecycleState {
    Created = 0,
    Runnable = 1,
    Running = 2,
    Exited = 3,
}

impl_enum_u8_default_conversions!(ProcessLifecycleState {
    Created,
    Runnable,
    Running,
    Exited,
}, default = Created);

impl ProcessLifecycleState {
    #[inline(always)]
    pub(crate) const fn from_raw(raw: u8) -> Self {
        Self::from_u8(raw)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MappingRecord {
    pub map_id: u32,
    pub start: u64,
    pub end: u64,
    pub prot: u32,
    pub flags: u32,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeLifecycleHooks {
    pub preinit_array: Vec<u64>,
    pub init: Option<u64>,
    pub init_array: Vec<u64>,
    pub deferred_fini: Vec<u64>,
    pub fini_array: Vec<u64>,
    pub fini: Option<u64>,
}

impl RuntimeLifecycleHooks {
    pub fn ordered_init_calls(&self) -> Vec<u64> {
        let mut ordered = Vec::new();
        ordered.extend(self.preinit_array.iter().copied().filter(|addr| *addr != 0));
        if let Some(init) = self.init.filter(|addr| *addr != 0) {
            ordered.push(init);
        }
        ordered.extend(self.init_array.iter().copied().filter(|addr| *addr != 0));
        ordered
    }

    pub fn ordered_fini_calls(&self) -> Vec<u64> {
        let mut seen = alloc::collections::BTreeSet::new();
        let mut ordered = Vec::new();
        for addr in self.deferred_fini.iter().copied().filter(|addr| *addr != 0) {
            if seen.insert(addr) {
                ordered.push(addr);
            }
        }
        for addr in self
            .fini_array
            .iter()
            .rev()
            .copied()
            .filter(|addr| *addr != 0)
        {
            if seen.insert(addr) {
                ordered.push(addr);
            }
        }
        if let Some(fini) = self.fini.filter(|addr| *addr != 0) {
            if seen.insert(fini) {
                ordered.push(fini);
            }
        }
        ordered
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRuntimeContractSnapshot {
    pub image_entry: usize,
    pub runtime_entry: usize,
    pub runtime_fini_entry: usize,
    pub image_base: usize,
    pub phdr_addr: usize,
    pub vdso_base: usize,
    pub vvar_base: usize,
    pub exec_path: String,
    pub init_calls: Vec<u64>,
    pub fini_calls: Vec<u64>,
}
