use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use xmas_elf::header::{Class, Data, Machine, Version};
use xmas_elf::program::Type;
use xmas_elf::ElfFile;
#[path = "module_loader_bootstrap.rs"]
mod module_loader_bootstrap;
#[path = "module_loader_runtime.rs"]
mod module_loader_runtime;
#[path = "module_loader_support.rs"]
mod module_loader_support;
#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
use module_loader_bootstrap::materialize_virtual_mapping_range;
#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
use module_loader_runtime::{
    encode_x86_64_runtime_fini_trampoline, encode_x86_64_runtime_init_trampoline,
    install_runtime_fini_trampoline, install_runtime_init_trampoline, materialize_write_bytes_at,
    materialize_write_u64_at,
};
use module_loader_support::{
    align_down, align_up, checked_table_end, current_target_elf_machine,
    elf_machine_matches_target, entry_in_segments, image_fingerprint, segment_range_fits_image,
    ELF_HEADER_MIN_BYTES, PAGE_SIZE,
};

#[derive(Debug, Clone, Copy)]
pub struct ModuleImageInfo {
    pub entry: u64,
    pub program_headers: u16,
    pub program_header_entry_size: u16,
    pub program_header_addr: u64,
    pub section_headers: u16,
    pub machine: Machine,
}

#[derive(Debug, Clone, Copy)]
pub struct LoadSegmentPlan {
    pub virtual_addr: u64,
    pub file_offset: u64,
    pub file_size: u64,
    pub mem_size: u64,
    pub align: u64,
}

#[derive(Debug, Clone)]
pub struct ModuleLoadPlan {
    pub entry: u64,
    pub segments: Vec<LoadSegmentPlan>,
    pub total_file_bytes: u64,
    pub total_mem_bytes: u64,
    pub aslr_base: u64,
    pub tls_virtual_addr: u64,
    pub tls_file_size: u64,
    pub tls_mem_size: u64,
    pub tls_align: u64,
    pub program_header_addr: u64,
    pub program_header_entry_size: u16,
    pub program_headers: u16,
}

#[cfg(feature = "process_abstraction")]
#[derive(Debug, Clone)]
pub struct PreparedProcessImage {
    pub info: ModuleImageInfo,
    pub load_plan: ModuleLoadPlan,
    pub mappings: Vec<VirtualMappingPlan>,
}

#[derive(Debug, Clone, Copy)]
pub struct VirtualMappingPlan {
    pub start: u64,
    pub end: u64,
    pub file_bytes: u64,
    pub zero_fill_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleLoadError {
    TooSmall,
    ParseFailed,
    UnsupportedClass,
    UnsupportedEndian,
    UnsupportedVersion,
    UnsupportedMachine,
    ProgramHeaderOutOfBounds,
    SectionHeaderOutOfBounds,
    NoLoadSegments,
    SegmentOutOfBounds,
    SegmentFileExceedsMem,
    SegmentAddressOverflow,
    SegmentOverlap,
    SegmentAlignmentMismatch,
    TooManyLoadSegments,
    ImageTooLarge,
    EntryOutsideLoadSegments,
}

#[cfg(feature = "process_abstraction")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessPrepareError {
    Loader(ModuleLoadError),
    ProcessBindFailed,
    MappingBindFailed,
    PagingApplyFailed,
    SegmentMaterializationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentMaterializationError {
    SegmentOutOfBounds,
    SegmentAddressOverflow,
    InvalidSegmentRange,
}

#[derive(Debug, Clone, Copy)]
pub struct ModuleLoaderStats {
    pub preflight_attempts: u64,
    pub preflight_success: u64,
    pub preflight_failures: u64,
    pub last_preflight_fingerprint: u64,
    pub parse_attempts: u64,
    pub parse_success: u64,
    pub parse_failures: u64,
    pub plan_attempts: u64,
    pub plan_success: u64,
    pub plan_failures: u64,
    pub mapping_plan_attempts: u64,
    pub mapping_plan_success: u64,
    pub mapping_plan_failures: u64,
    pub bootstrap_task_attempts: u64,
    pub bootstrap_task_success: u64,
    pub bootstrap_task_failures: u64,
    pub segment_materialization_attempts: u64,
    pub segment_materialization_success: u64,
    pub segment_materialization_failures: u64,
    pub segment_materialized_bytes: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ModulePreflightReport {
    pub entry: u64,
    pub load_segments: usize,
    pub total_file_bytes: u64,
    pub total_mem_bytes: u64,
    pub fingerprint: u64,
    pub machine: Machine,
}

#[derive(Debug, Clone)]
pub struct ModuleImageSnapshot {
    pub info: ModuleImageInfo,
    pub load_plan: ModuleLoadPlan,
    pub mappings: Vec<VirtualMappingPlan>,
}

static PREFLIGHT_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static PREFLIGHT_SUCCESS: AtomicU64 = AtomicU64::new(0);
static PREFLIGHT_FAILURES: AtomicU64 = AtomicU64::new(0);
static LAST_PREFLIGHT_FINGERPRINT: AtomicU64 = AtomicU64::new(0);
static PARSE_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static PARSE_SUCCESS: AtomicU64 = AtomicU64::new(0);
static PARSE_FAILURES: AtomicU64 = AtomicU64::new(0);
static PLAN_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static PLAN_SUCCESS: AtomicU64 = AtomicU64::new(0);
static PLAN_FAILURES: AtomicU64 = AtomicU64::new(0);
static MAP_PLAN_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static MAP_PLAN_SUCCESS: AtomicU64 = AtomicU64::new(0);
static MAP_PLAN_FAILURES: AtomicU64 = AtomicU64::new(0);
static BOOTSTRAP_TASK_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static BOOTSTRAP_TASK_SUCCESS: AtomicU64 = AtomicU64::new(0);
static BOOTSTRAP_TASK_FAILURES: AtomicU64 = AtomicU64::new(0);
static SEGMENT_MATERIALIZATION_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static SEGMENT_MATERIALIZATION_SUCCESS: AtomicU64 = AtomicU64::new(0);
static SEGMENT_MATERIALIZATION_FAILURES: AtomicU64 = AtomicU64::new(0);
static SEGMENT_MATERIALIZED_BYTES: AtomicU64 = AtomicU64::new(0);
#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
static NEXT_RUNTIME_TRAMPOLINE_MAP_ID: AtomicU64 = AtomicU64::new(0x6F00_0000);
#[path = "module_loader_plan.rs"]
mod module_loader_plan;
#[path = "module_loader_process.rs"]
mod module_loader_process;
pub use module_loader_plan::{
    build_load_plan, build_virtual_mapping_plan, inspect_elf_image, materialize_load_segments,
    preflight_module_image, snapshot_module_image,
};
#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub use module_loader_process::materialize_process_image;
#[cfg(feature = "process_abstraction")]
pub use module_loader_process::prepare_process_image_entry;
#[cfg(feature = "process_abstraction")]
pub use module_loader_process::prepare_process_image_entry_from_snapshot;
#[cfg(feature = "process_abstraction")]
pub use module_loader_process::prepare_process_image;
#[cfg(feature = "process_abstraction")]
pub use module_loader_bootstrap::build_process_bootstrap_task;
#[cfg(feature = "process_abstraction")]
pub use module_loader_bootstrap::build_process_bootstrap_task_from_snapshot;
#[cfg(all(feature = "process_abstraction", feature = "paging_enable"))]
pub use module_loader_bootstrap::materialize_and_build_process_bootstrap_task;
pub fn stats() -> ModuleLoaderStats {
    module_loader_bootstrap::stats()
}

#[cfg(test)]
#[path = "module_loader_tests.rs"]
mod module_loader_tests;
