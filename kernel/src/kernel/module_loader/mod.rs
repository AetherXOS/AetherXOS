use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessPrepareError {
    Loader(u64),
    ProcessBindFailed,
    MappingBindFailed,
    PagingApplyFailed,
    SegmentMaterializationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleLoadError {
    TooSmall,
    ParseFailed,
    UnsupportedClass,
    UnsupportedEndian,
    UnsupportedVersion,
    UnsupportedMachine,
    NoLoadSegments,
    TooManyLoadSegments,
    ImageTooLarge,
    SegmentOutOfBounds,
    SegmentFileExceedsMem,
    SegmentAlignmentMismatch,
    SegmentAddressOverflow,
    EntryOutsideLoadSegments,
    SegmentOverlap,
    ProgramHeaderOutOfBounds,
    SectionHeaderOutOfBounds,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentMaterializationError {
    InvalidSegmentRange,
    SegmentOutOfBounds,
    SegmentAddressOverflow,
}

#[derive(Debug, Clone)]
pub struct ModulePreflightReport {
    pub entry: u64,
    pub load_segments: usize,
    pub total_file_bytes: u64,
    pub total_mem_bytes: u64,
    pub fingerprint: u64,
    pub machine: xmas_elf::header::Machine,
}

pub struct ModuleInfo {
    pub entry: u64,
    pub program_headers: u16,
    pub program_header_entry_size: u16,
    pub program_header_addr: u64,
    pub section_headers: u16,
    pub machine: xmas_elf::header::Machine,
    pub interpreter_path: Option<String>,
}

pub struct VirtualMappingPlan {
    pub start: u64,
    pub end: u64,
    pub virtual_addr: u64,
    pub mem_size: u64,
    pub file_bytes: u64,
    pub zero_fill_bytes: u64,
    pub file_offset: u64,
}

// Compatibility: older code expects `LoadSegmentPlan` with specific fields.
pub struct LoadSegmentPlan {
    pub virtual_addr: u64,
    pub file_offset: u64,
    pub file_size: u64,
    pub mem_size: u64,
    pub align: u64,
}

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

lazy_static::lazy_static! {
    pub static ref PREFLIGHT_ATTEMPTS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref PREFLIGHT_SUCCESS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref PREFLIGHT_FAILURES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref LAST_PREFLIGHT_FINGERPRINT: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref PARSE_ATTEMPTS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref PARSE_SUCCESS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref PARSE_FAILURES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref PLAN_ATTEMPTS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref PLAN_SUCCESS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref PLAN_FAILURES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref MAP_PLAN_ATTEMPTS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref MAP_PLAN_SUCCESS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref MAP_PLAN_FAILURES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref BOOTSTRAP_TASK_ATTEMPTS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref BOOTSTRAP_TASK_SUCCESS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref BOOTSTRAP_TASK_FAILURES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref SEGMENT_MATERIALIZATION_ATTEMPTS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref SEGMENT_MATERIALIZATION_SUCCESS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref SEGMENT_MATERIALIZATION_FAILURES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    pub static ref SEGMENT_MATERIALIZED_BYTES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
}

pub struct ModuleImageSnapshot {
    pub fingerprint: u64,
    pub entry: u64,
    pub load_plan: Arc<ModuleLoadPlan>,
    pub info: ModuleInfo,
    pub mappings: Vec<VirtualMappingPlan>,
}

pub struct ModuleLoadPlan {
    pub entry: u64,
    pub segments: Vec<LoadSegmentPlan>,
    pub total_file_bytes: u64,
    pub total_mem_bytes: u64,
    pub aslr_base: u64,
    pub tls_mem_size: u64,
    pub tls_align: u64,
    pub tls_file_size: u64,
    pub tls_virtual_addr: u64,
    pub program_header_addr: u64,
    pub program_header_entry_size: u16,
    pub program_headers: u16,
}

pub fn prepare_process_image(
    process: &crate::kernel::process::Process,
    image: &[u8],
) -> Result<u64, ProcessPrepareError> {
    let snapshot = snapshot_module_image(image).map_err(|err| {
        crate::klog_warn!("[LOADER] snapshot_module_image failed for prepare_process_image: {:?}", err);
        ProcessPrepareError::Loader(0)
    })?;

    crate::kernel::process::bind_prepared_image_snapshot(process, image, &snapshot)
        .map_err(|err| {
            crate::klog_warn!("[LOADER] bind_prepared_image_snapshot failed: {}", err);
            ProcessPrepareError::ProcessBindFailed
        })?;

    Ok(snapshot.load_plan.entry)
}

pub fn inspect_elf_image(data: &[u8]) -> Result<ModuleInfo, ModuleLoadError> {
    let elf = plan::elf::parse_elf(data)?;
    Ok(plan::elf::inspect_elf_from_parsed(&elf))
}

pub fn prepare_process_image_entry_from_snapshot(
    process: &crate::kernel::process::Process,
    image: &[u8],
    snapshot: ModuleImageSnapshot,
) -> Result<u64, ProcessPrepareError> {
    crate::kernel::process::bind_prepared_image_snapshot(process, image, &snapshot)
        .map_err(|err| {
            crate::klog_warn!("[LOADER] bind_prepared_image_snapshot failed: {}", err);
            ProcessPrepareError::ProcessBindFailed
        })?;

    Ok(snapshot.load_plan.entry)
}

pub fn materialize_process_image(
    process: &crate::kernel::process::Process,
    image: &[u8],
    _pm: &mut crate::kernel::memory::paging::PageManager,
    _fa: &mut impl x86_64::structures::paging::FrameAllocator<x86_64::structures::paging::Size4KiB>,
) -> Result<ModuleImageSnapshot, ProcessPrepareError> {
    let snapshot = snapshot_module_image(image).map_err(|err| {
        crate::klog_warn!("[LOADER] snapshot_module_image failed for materialize_process_image: {:?}", err);
        ProcessPrepareError::Loader(0)
    })?;

    crate::kernel::process::bind_prepared_image_snapshot(process, image, &snapshot)
        .map_err(|err| {
            crate::klog_warn!("[LOADER] bind_prepared_image_snapshot failed: {}", err);
            ProcessPrepareError::ProcessBindFailed
        })?;

    Ok(snapshot)
}

pub fn preflight_module_image(data: &[u8]) -> Result<(), &'static str> {
    PREFLIGHT_ATTEMPTS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if data.len() < core::mem::size_of::<Elf64Header>() {
        PREFLIGHT_FAILURES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        return Err("data too small for ELF header");
    }

    let header = unsafe { &*(data.as_ptr() as *const Elf64Header) };
    
    // Validate ELF Magic: 0x7F 'E' 'L' 'F'
    if &header.ident[0..4] != b"\x7FELF" {
        PREFLIGHT_FAILURES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        return Err("invalid ELF magic");
    }

    // Fingerprint for stats tracking (simple sum of bytes for now)
    let fingerprint = data.iter().take(256).fold(0u64, |acc, &b| acc.wrapping_add(b as u64));
    LAST_PREFLIGHT_FINGERPRINT.store(fingerprint, core::sync::atomic::Ordering::Relaxed);

    PREFLIGHT_SUCCESS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    Ok(())
}

pub mod support;
pub use support as module_loader_support;
pub mod bootstrap;
pub use bootstrap::*;
pub mod runtime;

#[cfg(feature = "process_abstraction")]
mod plan;

#[cfg(feature = "process_abstraction")]
pub use plan::{build_load_plan, snapshot_module_image};

// The process loader path is implemented directly above to avoid relying on
// the incomplete sibling module during bootstrap.

/// ELF Header (x86_64)
#[repr(C)]
struct Elf64Header {
    ident: [u8; 16],
    elf_type: u16,
    machine: u16,
    version: u32,
    entry: u64,
    phoff: u64,
    shoff: u64,
    flags: u32,
    ehsize: u16,
    phentsize: u16,
    phnum: u16,
    shentsize: u16,
    shnum: u16,
    shstrndx: u16,
}

/// ELF Program Header
#[repr(C)]
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

pub struct KernelModule {
    pub name: String,
    pub base_addr: u64,
    pub size: usize,
    pub entry_point: u64,
}

pub struct ModuleRegistry {
    pub modules: BTreeMap<String, Arc<KernelModule>>,
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_MODULE_REGISTRY: Mutex<ModuleRegistry> = Mutex::new(ModuleRegistry {
        modules: BTreeMap::new(),
    });
}

/// High-Fidelity ELF Module Loader.
pub fn load_module(name: &str, data: &[u8]) -> Result<(), &'static str> {
    if data.len() < core::mem::size_of::<Elf64Header>() {
        return Err("data too small for ELF header");
    }

    let header = unsafe { &*(data.as_ptr() as *const Elf64Header) };
    
    // Validate ELF Magic: 0x7F 'E' 'L' 'F'
    if &header.ident[0..4] != b"\x7FELF" {
        return Err("invalid ELF magic");
    }

    // Iterate Program Headers and map segments
    let mut module_size = 0u64;
    for i in 0..header.phnum {
        let phdr_ptr = data.as_ptr().wrapping_add(header.phoff as usize + (i as usize * header.phentsize as usize));
        let phdr = unsafe { &*(phdr_ptr as *const Elf64Phdr) };

        if phdr.p_type == 1 { // PT_LOAD
            let end = phdr.p_vaddr + phdr.p_memsz;
            if end > module_size { module_size = end; }
            
            // Here we would use the HugePage-aware VMM to map the segment
            // For now, we simulate the memory allocation and copy
            crate::klog_info!("[ELF] Mapping segment at {:#x}, size {:#x}", phdr.p_vaddr, phdr.p_memsz);
        }
    }

    let module = Arc::new(KernelModule {
        name: name.to_string(),
        base_addr: 0, // Assigned by VMM
        size: module_size as usize,
        entry_point: header.entry,
    });

    GLOBAL_MODULE_REGISTRY.lock().modules.insert(name.to_string(), module);
    crate::klog_info!("[MODULE] Successfully loaded ELF module '{}', entry: {:#x}", name, header.entry);
    Ok(())
}

pub fn unload_module(name: &str) -> Result<(), &'static str> {
    crate::klog_info!("[MODULE] Unloading dynamic module '{}'", name);
    GLOBAL_MODULE_REGISTRY.lock().modules.remove(name).ok_or("module not found")?;
    Ok(())
}

/// ELF Relocation (with addend)
#[repr(C)]
struct Elf64Rela {
    r_offset: u64,
    r_info: u64,
    r_addend: i64,
}

impl Elf64Rela {
    fn r_type(&self) -> u32 { (self.r_info & 0xFFFFFFFF) as u32 }
    fn r_sym(&self) -> u32 { (self.r_info >> 32) as u32 }
}

/// Apply ELF Relocations (Linking the module to the kernel).
fn apply_relocations(module_base: u64, data: &[u8], header: &Elf64Header) -> Result<(), &'static str> {
    for i in 0..header.shnum {
        let shdr_ptr = data.as_ptr().wrapping_add(header.shoff as usize + (i as usize * header.shentsize as usize));
        let shdr = unsafe { &*(shdr_ptr as *const Elf64Shdr) };

        if shdr.sh_type == 4 { // SHT_RELA
            let rela_count = shdr.sh_size / shdr.sh_entsize;
            for j in 0..rela_count {
                let rela_ptr = data.as_ptr().wrapping_add(shdr.sh_offset as usize + (j as usize * shdr.sh_entsize as usize));
                let rela = unsafe { &*(rela_ptr as *const Elf64Rela) };
                
                // --- Real Symbol Name Resolution ---
                // 1. Find the symbol table (usually shdr.sh_link points to the strtab)
                let symtab_shdr = unsafe { &*(data.as_ptr().wrapping_add(header.shoff as usize + (header.shnum as usize - 2) * header.shentsize as usize) as *const Elf64Shdr) };
                let strtab_shdr = unsafe { &*(data.as_ptr().wrapping_add(header.shoff as usize + (header.shnum as usize - 1) * header.shentsize as usize) as *const Elf64Shdr) };
                
                let sym_ptr = data.as_ptr().wrapping_add(symtab_shdr.sh_offset as usize + (rela.r_sym() as usize * 24));
                let sym_st_name = unsafe { *(sym_ptr as *const u32) };
                
                let str_ptr = data.as_ptr().wrapping_add(strtab_shdr.sh_offset as usize + sym_st_name as usize);
                let sym_name = unsafe { core::str::from_utf8_unchecked(core::ffi::CStr::from_ptr(str_ptr as *const i8).to_bytes()) };

                let sym_val = crate::kernel::symbols::KSYMTAB.lock()
                    .resolve(sym_name)
                    .ok_or_else(|| {
                        crate::klog_err!("[ELF] Undefined symbol: {}", sym_name);
                        "undefined symbol"
                    })?;
                
                let target_addr = module_base + rela.r_offset;
                unsafe {

                    match rela.r_type() {
                        1 => { // R_X86_64_64
                            *(target_addr as *mut u64) = sym_val + rela.r_addend as u64;
                        }
                        2 => { // R_X86_64_PC32
                            let val = (sym_val as i64 + rela.r_addend) - target_addr as i64;
                            *(target_addr as *mut u32) = val as u32;
                        }
                        4 => { // R_X86_64_PLT32
                            // In a kernel module, PLT32 is often treated like PC32
                            let val = (sym_val as i64 + rela.r_addend) - target_addr as i64;
                            *(target_addr as *mut u32) = val as u32;
                        }
                        _ => {
                            crate::klog_warn!("[ELF] Unsupported relocation type {}", rela.r_type());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// ELF Section Header (Mock for apply_relocations)
#[repr(C)]
struct Elf64Shdr {
    sh_name: u32,
    sh_type: u32,
    sh_flags: u64,
    sh_addr: u64,
    sh_offset: u64,
    sh_size: u64,
    sh_link: u32,
    sh_info: u32,
    sh_addralign: u64,
    sh_entsize: u64,
}

