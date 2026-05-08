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
    
    // Parse using xmas_elf
    let elf = match xmas_elf::ElfFile::new(data) {
        Ok(e) => e,
        Err(_) => {
            PREFLIGHT_FAILURES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            return Err("invalid ELF file or too small");
        }
    };

    // Validate ELF Magic & Class (64-bit)
    if elf.header.pt1.class() != xmas_elf::header::Class::SixtyFour {
        PREFLIGHT_FAILURES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        return Err("only 64-bit ELF images supported");
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

// ELF types are now handled via xmas_elf

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
    use xmas_elf::program::Type;
    let elf = xmas_elf::ElfFile::new(data).map_err(|_| "failed to parse ELF")?;
    
    // Validate ELF Magic & Machine
    if !crate::kernel::module_loader::support::elf_machine_matches_target(elf.header.pt2.machine().as_machine()) {
        return Err("unsupported machine type");
    }

    // Iterate Program Headers and calculate module size
    let mut module_size = 0u64;
    for phdr in elf.program_iter() {
        if phdr.get_type() == Ok(Type::Load) {
            let end = phdr.virtual_addr() + phdr.mem_size();
            if end > module_size { module_size = end; }
            
            crate::klog_info!("[ELF] Mapping segment at {:#x}, size {:#x}", phdr.virtual_addr(), phdr.mem_size());
        }
    }

    // In a real system, we'd allocate from the kernel heap or a dedicated module area.
    // For this implementation, we'll assume the module is loaded at its preferred address (or 0 if PIC)
    // for demonstration, or we'd have a VMM call here.
    let module_base = 0; // Placeholder for actual allocation

    let module = Arc::new(KernelModule {
        name: name.to_string(),
        base_addr: module_base,
        size: module_size as usize,
        entry_point: elf.header.pt2.entry_point(),
    });

    // Apply relocations if any
    apply_relocations(module_base, &elf)?;

    GLOBAL_MODULE_REGISTRY.lock().modules.insert(name.to_string(), module);
    crate::klog_info!("[MODULE] Successfully loaded ELF module '{}', entry: {:#x}", name, elf.header.pt2.entry_point());
    Ok(())
}

pub fn unload_module(name: &str) -> Result<(), &'static str> {
    crate::klog_info!("[MODULE] Unloading dynamic module {}", name);
    GLOBAL_MODULE_REGISTRY.lock().modules.remove(name).ok_or("module not in registry")?;
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
fn apply_relocations(module_base: u64, elf: &xmas_elf::ElfFile<'_>) -> Result<(), &'static str> {
    use xmas_elf::sections::{ShType, SectionData};
    use xmas_elf::symbol_table::Entry;

    for sect in elf.section_iter() {
        if sect.get_type() == Ok(ShType::Rela) {
            let data = sect.get_data(elf).map_err(|_| "failed to parse relocation")?;
            if let SectionData::Rela64(relas) = data {
                // The sh_link of a relocation section points to the symbol table
                let symtab_idx = sect.link();
                let symtab_sect = elf.section_header(symtab_idx as u16).map_err(|_| "invalid symtab index")?;
                let symtab_data = symtab_sect.get_data(elf).map_err(|_| "failed to parse symbols")?;
                


                if let SectionData::SymbolTable64(symbols) = symtab_data {
                    for rela in relas {
                        let sym = &symbols[rela.get_symbol_table_index() as usize];
                        let sym_name = sym.get_name(elf).map_err(|_| "failed to get symbol name")?;
                        
                        let sym_val = if sym.shndx() == 0 { // STN_UNDEF
                            crate::kernel::symbols::KSYMTAB.lock()
                                .resolve(sym_name)
                                .ok_or_else(|| {
                                    crate::klog_err!("[ELF] Unresolved symbol: {}", sym_name);
                                    "kernel symbol not resolved"
                                })?
                        } else {
                            module_base + sym.value()
                        };
                        
                        let target_addr = module_base + rela.get_offset();
                        unsafe {
                            match rela.get_type() {
                                1 => { // R_X86_64_64
                                    *(target_addr as *mut u64) = sym_val.wrapping_add(rela.get_addend() as u64);
                                }
                                2 => { // R_X86_64_PC32
                                    let val = (sym_val as i64 + rela.get_addend() as i64) - target_addr as i64;
                                    *(target_addr as *mut u32) = val as u32;
                                }
                                4 => { // R_X86_64_PLT32
                                    let val = (sym_val as i64 + rela.get_addend() as i64) - target_addr as i64;
                                    *(target_addr as *mut u32) = val as u32;
                                }
                                _ => {
                                    crate::klog_warn!("[ELF] Unsupported relocation type {}", rela.get_type());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // Elf64Rela and Elf64Shdr are now handled via xmas_elf
    Ok(())
}
