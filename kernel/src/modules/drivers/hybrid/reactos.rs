use alloc::vec::Vec;

pub mod domain;
pub mod imports;
pub mod parser;
pub mod runtime;
pub mod reloc;

pub use runtime::{Irql, NtIrqlGuard, NtSpinLock, NtSymbol, NtSymbolBinding, NtSymbolTable};
pub use domain::{NtImportReadinessLevel, NtImportReadinessReport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeLoadError {
    InvalidDosSignature,
    InvalidPeSignature,
    Truncated,
    UnsupportedMachine,
    InvalidOptionalHeader,
    InvalidSectionTable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NtBinaryExecutionMode {
    NativeKernel,
    WineHostBridge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NtExecutionPolicy {
    pub mode: NtBinaryExecutionMode,
    pub isolate_address_space: bool,
    pub allow_user_mode_callbacks: bool,
}

impl NtExecutionPolicy {
    pub const fn native() -> Self {
        Self {
            mode: NtBinaryExecutionMode::NativeKernel,
            isolate_address_space: true,
            allow_user_mode_callbacks: false,
        }
    }

    pub const fn wine_bridge() -> Self {
        Self {
            mode: NtBinaryExecutionMode::WineHostBridge,
            isolate_address_space: true,
            allow_user_mode_callbacks: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeSectionInfo {
    pub virtual_address: u32,
    pub virtual_size: u32,
    pub raw_data_ptr: u32,
    pub raw_data_size: u32,
    pub characteristics: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeImageInfo {
    pub machine: u16,
    pub image_base: u64,
    pub entry_rva: u32,
    pub size_of_image: u32,
    pub size_of_headers: u32,
    pub number_of_sections: u16,
    pub sections: Vec<PeSectionInfo>,
    pub import_directory_rva: u32,
    pub import_directory_size: u32,
    pub relocation_directory_rva: u32,
    pub relocation_directory_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeImportDescriptor {
    pub name_rva: u32,
    pub first_thunk_rva: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeRelocationBlock {
    pub page_rva: u32,
    pub block_size: u32,
    pub entry_count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NtImportBinding {
    pub descriptor_index: usize,
    pub symbol: NtSymbol,
    pub address: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NtDomainImportBinding {
    pub domain: NtImportDomain,
    pub binding: NtImportBinding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NtImportDomainCounts {
    pub kernel: usize,
    pub hal: usize,
    pub win32: usize,
    pub unknown: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NtImportResolutionReport {
    pub policy: NtExecutionPolicy,
    pub counts: NtImportDomainCounts,
    pub bindings: Vec<NtDomainImportBinding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NtImportDomain {
    Kernel,
    Hal,
    Win32,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeImportName {
    pub dll: Vec<u8>,
    pub name: Vec<u8>,
    pub descriptor_index: usize,
    pub thunk_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelocationPatch {
    pub target_rva: u32,
    pub old_value: u64,
    pub new_value: u64,
}

const DOS_SIGNATURE: u16 = 0x5A4D;
const PE_SIGNATURE: u32 = 0x0000_4550;
const MACHINE_AMD64: u16 = 0x8664;

pub fn parse_pe_image(image: &[u8]) -> Result<PeImageInfo, PeLoadError> {
    parser::parse_pe_image(image)
}

pub fn parse_import_directory(
    image: &[u8],
    info: &PeImageInfo,
) -> Result<Vec<PeImportDescriptor>, PeLoadError> {
    parser::parse_import_directory(image, info)
}

pub fn parse_relocation_blocks(
    image: &[u8],
    info: &PeImageInfo,
) -> Result<Vec<PeRelocationBlock>, PeLoadError> {
    parser::parse_relocation_blocks(image, info)
}

pub fn bind_imports_with_symbol_table(
    imports: &[PeImportDescriptor],
    symbol_table: &NtSymbolTable,
) -> Vec<NtImportBinding> {
    parser::bind_imports_with_symbol_table(imports, symbol_table)
}

pub fn parse_import_names(
    image: &[u8],
    info: &PeImageInfo,
    imports: &[PeImportDescriptor],
) -> Vec<PeImportName> {
    parser::parse_import_names(image, info, imports)
}

pub fn bind_import_names(
    names: &[PeImportName],
    symbol_table: &NtSymbolTable,
) -> Vec<NtImportBinding> {
    imports::bind_import_names(names, symbol_table)
}

pub fn bind_import_names_with_domain_tables(
    names: &[PeImportName],
    symbol_table: &NtSymbolTable,
) -> Vec<NtDomainImportBinding> {
    domain::bind_import_names_with_domain_tables(names, symbol_table)
}

pub fn classify_import_dll(dll: &[u8]) -> NtImportDomain {
    domain::classify_import_dll(dll)
}

pub fn recommended_policy_for_imports(names: &[PeImportName]) -> NtExecutionPolicy {
    domain::recommended_policy_for_imports(names)
}

pub fn summarize_import_domains(names: &[PeImportName]) -> NtImportDomainCounts {
    domain::summarize_import_domains(names)
}

pub fn build_import_resolution_report(
    names: &[PeImportName],
    symbol_table: &NtSymbolTable,
) -> NtImportResolutionReport {
    domain::build_import_resolution_report(names, symbol_table)
}

pub fn import_readiness_report(
    names: &[PeImportName],
    symbol_table: &NtSymbolTable,
) -> NtImportReadinessReport {
    domain::import_readiness_report(names, symbol_table)
}

pub fn plan_relocation_patches(
    blocks: &[PeRelocationBlock],
    old_image_base: u64,
    new_image_base: u64,
) -> Vec<RelocationPatch> {
    reloc::plan_relocation_patches(blocks, old_image_base, new_image_base)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_minimal_amd64_pe() -> Vec<u8> {
        let mut image = vec![0u8; 0x400];

        image[0] = 0x4D;
        image[1] = 0x5A;
        image[0x3C..0x40].copy_from_slice(&(0x80u32).to_le_bytes());

        image[0x80..0x84].copy_from_slice(&PE_SIGNATURE.to_le_bytes());

        let file_header = 0x84;
        image[file_header..file_header + 2].copy_from_slice(&MACHINE_AMD64.to_le_bytes());
        image[file_header + 2..file_header + 4].copy_from_slice(&(1u16).to_le_bytes());
        image[file_header + 16..file_header + 18].copy_from_slice(&(0xF0u16).to_le_bytes());

        let optional = file_header + 20;
        image[optional..optional + 2].copy_from_slice(&(0x20Bu16).to_le_bytes());
        image[optional + 16..optional + 20].copy_from_slice(&(0x1000u32).to_le_bytes());
        image[optional + 24..optional + 32].copy_from_slice(&(0x140000000u64).to_le_bytes());
        image[optional + 56..optional + 60].copy_from_slice(&(0x4000u32).to_le_bytes());
        image[optional + 60..optional + 64].copy_from_slice(&(0x400u32).to_le_bytes());
        image[optional + 120..optional + 124].copy_from_slice(&(0x1800u32).to_le_bytes());
        image[optional + 124..optional + 128].copy_from_slice(&(0x28u32).to_le_bytes());
        image[optional + 152..optional + 156].copy_from_slice(&(0x1A00u32).to_le_bytes());
        image[optional + 156..optional + 160].copy_from_slice(&(0x10u32).to_le_bytes());

        let section = optional + 0xF0;
        image[section + 8..section + 12].copy_from_slice(&(0x200u32).to_le_bytes());
        image[section + 12..section + 16].copy_from_slice(&(0x1000u32).to_le_bytes());
        image[section + 16..section + 20].copy_from_slice(&(0x200u32).to_le_bytes());
        image[section + 20..section + 24].copy_from_slice(&(0x200u32).to_le_bytes());
        image[section + 36..section + 40].copy_from_slice(&(0x60000020u32).to_le_bytes());

        let import_offset = 0x200 + 0x800;
        image[import_offset..import_offset + 4].copy_from_slice(&(0x1810u32).to_le_bytes());
        image[import_offset + 12..import_offset + 16].copy_from_slice(&(0x1820u32).to_le_bytes());
        image[import_offset + 16..import_offset + 20].copy_from_slice(&(0x1830u32).to_le_bytes());
        image[0x200 + 0x820..0x200 + 0x82E].copy_from_slice(b"ntoskrnl.exe\0");
        image[0x200 + 0x830..0x200 + 0x838].copy_from_slice(&(0x1840u64).to_le_bytes());
        image[0x200 + 0x840..0x200 + 0x842].copy_from_slice(&(0u16).to_le_bytes());
        image[0x200 + 0x842..0x200 + 0x852].copy_from_slice(b"IoCallDriver\0");

        let reloc_offset = 0x200 + 0xA00;
        image[reloc_offset..reloc_offset + 4].copy_from_slice(&(0x3000u32).to_le_bytes());
        image[reloc_offset + 4..reloc_offset + 8].copy_from_slice(&(0x000Cu32).to_le_bytes());

        image
    }

    #[test_case]
    fn parses_minimal_amd64_pe() {
        let image = build_minimal_amd64_pe();
        let info = parse_pe_image(&image).expect("valid image should parse");
        assert_eq!(info.machine, MACHINE_AMD64);
        assert_eq!(info.number_of_sections, 1);
        assert_eq!(info.sections.len(), 1);
        assert_eq!(info.import_directory_rva, 0x1800);
        assert_eq!(info.relocation_directory_rva, 0x1A00);
    }

    #[test_case]
    fn spin_lock_transitions_state() {
        let lock = NtSpinLock::new();
        assert!(lock.try_lock());
        assert!(lock.is_locked());
        lock.unlock();
        assert!(!lock.is_locked());
    }

    #[test_case]
    fn execution_policy_supports_native_and_wine_modes() {
        let native = NtExecutionPolicy::native();
        let wine = NtExecutionPolicy::wine_bridge();
        assert_eq!(native.mode, NtBinaryExecutionMode::NativeKernel);
        assert_eq!(wine.mode, NtBinaryExecutionMode::WineHostBridge);
        assert!(wine.allow_user_mode_callbacks);
    }

    #[test_case]
    fn parses_import_and_relocation_directories() {
        let image = build_minimal_amd64_pe();
        let info = parse_pe_image(&image).expect("image should parse");
        let imports = parse_import_directory(&image, &info).expect("imports should parse");
        let relocs = parse_relocation_blocks(&image, &info).expect("relocs should parse");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name_rva, 0x1820);
        assert_eq!(relocs.len(), 1);
        assert_eq!(relocs[0].entry_count, 2);
    }

    #[test_case]
    fn import_binding_uses_symbol_table() {
        let imports = vec![PeImportDescriptor {
            name_rva: 0x1111,
            first_thunk_rva: 3,
        }];
        let mut table = NtSymbolTable::new();
        table.register(NtSymbol::IoCompleteRequest, 0xFEED_BEEF);

        let bindings = bind_imports_with_symbol_table(&imports, &table);
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].address, 0xFEED_BEEF);
    }

    #[test_case]
    fn relocation_plan_applies_base_delta() {
        let blocks = vec![PeRelocationBlock {
            page_rva: 0x2000,
            block_size: 0x10,
            entry_count: 2,
        }];

        let patches = plan_relocation_patches(&blocks, 0x1400_0000, 0x1500_0000);
        assert_eq!(patches.len(), 2);
        assert_eq!(patches[0].target_rva, 0x2000);
        assert!(patches[0].new_value > patches[0].old_value);
    }

    #[test_case]
    fn parses_import_names_and_binds_symbols() {
        let image = build_minimal_amd64_pe();
        let info = parse_pe_image(&image).expect("image should parse");
        let imports = parse_import_directory(&image, &info).expect("imports should parse");
        let names = parse_import_names(&image, &info, &imports);
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].thunk_index, 0);

        let mut table = NtSymbolTable::new();
        table.register(NtSymbol::IoCallDriver, 0xABCD_EF01);
        let bindings = bind_import_names(&names, &table);
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].address, 0xABCD_EF01);
    }

    #[test_case]
    fn classify_import_domain_detects_kernel_and_win32() {
        assert_eq!(classify_import_dll(b"ntoskrnl.exe"), NtImportDomain::Kernel);
        assert_eq!(classify_import_dll(b"user32.dll"), NtImportDomain::Win32);
        assert_eq!(classify_import_dll(b"weird.dll"), NtImportDomain::Unknown);
    }

    #[test_case]
    fn recommended_policy_prefers_wine_for_win32_imports() {
        let names = vec![
            PeImportName {
                dll: b"ntoskrnl.exe".to_vec(),
                name: b"IoCallDriver".to_vec(),
                descriptor_index: 0,
                thunk_index: 0,
            },
            PeImportName {
                dll: b"user32.dll".to_vec(),
                name: b"MessageBoxA".to_vec(),
                descriptor_index: 1,
                thunk_index: 0,
            },
        ];
        let policy = recommended_policy_for_imports(&names);
        assert_eq!(policy.mode, NtBinaryExecutionMode::WineHostBridge);
    }

    #[test_case]
    fn domain_binding_prefers_kernel_tables_for_ntoskrnl() {
        let names = vec![PeImportName {
            dll: b"ntoskrnl.exe".to_vec(),
            name: b"IoCallDriver".to_vec(),
            descriptor_index: 0,
            thunk_index: 0,
        }];
        let mut table = NtSymbolTable::new();
        table.register(NtSymbol::IoCallDriver, 0x1111_2222);
        let bindings = bind_import_names_with_domain_tables(&names, &table);
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].domain, NtImportDomain::Kernel);
        assert_eq!(bindings[0].binding.address, 0x1111_2222);
    }

    #[test_case]
    fn summarize_import_domains_counts_win32() {
        let names = vec![
            PeImportName {
                dll: b"ntoskrnl.exe".to_vec(),
                name: b"IoCallDriver".to_vec(),
                descriptor_index: 0,
                thunk_index: 0,
            },
            PeImportName {
                dll: b"user32.dll".to_vec(),
                name: b"MessageBoxA".to_vec(),
                descriptor_index: 1,
                thunk_index: 0,
            },
        ];

        let counts = summarize_import_domains(&names);
        assert_eq!(counts.kernel, 1);
        assert_eq!(counts.win32, 1);
    }

    #[test_case]
    fn import_readiness_report_flags_partial_when_bindings_missing() {
        let names = vec![
            PeImportName {
                dll: b"ntoskrnl.exe".to_vec(),
                name: b"IoCallDriver".to_vec(),
                descriptor_index: 0,
                thunk_index: 0,
            },
            PeImportName {
                dll: b"user32.dll".to_vec(),
                name: b"MessageBoxA".to_vec(),
                descriptor_index: 1,
                thunk_index: 0,
            },
        ];
        let mut table = NtSymbolTable::new();
        table.register(NtSymbol::IoCallDriver, 0x1111_2222);

        let report = import_readiness_report(&names, &table);
        assert_eq!(report.total_imports, 2);
        assert_eq!(report.resolved_imports, 1);
        assert!(matches!(report.readiness, NtImportReadinessLevel::Partial));
    }

    #[test_case]
    fn parses_multiple_import_thunks_from_single_descriptor() {
        let mut image = build_minimal_amd64_pe();
        image[0x200 + 0x838..0x200 + 0x840].copy_from_slice(&(0x1850u64).to_le_bytes());
        image[0x200 + 0x840..0x200 + 0x848].copy_from_slice(&(0u64).to_le_bytes());
        image[0x200 + 0x850..0x200 + 0x852].copy_from_slice(&(0u16).to_le_bytes());
        image[0x200 + 0x852..0x200 + 0x861].copy_from_slice(b"IoCreateDevice\0");

        let info = parse_pe_image(&image).expect("image should parse");
        let imports = parse_import_directory(&image, &info).expect("imports should parse");
        let names = parse_import_names(&image, &info, &imports);
        assert_eq!(names.len(), 2);
        assert_eq!(names[0].thunk_index, 0);
        assert_eq!(names[1].thunk_index, 1);
    }

    #[test_case]
    fn bind_import_names_supports_pool_symbols() {
        let names = vec![PeImportName {
            dll: b"ntoskrnl.exe".to_vec(),
            name: b"ExAllocatePool2".to_vec(),
            descriptor_index: 0,
            thunk_index: 0,
        }];

        let mut table = NtSymbolTable::new();
        table.register(NtSymbol::ExAllocatePool2, 0x2222_3333);
        let bindings = bind_import_names(&names, &table);
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].symbol, NtSymbol::ExAllocatePool2);
        assert_eq!(bindings[0].address, 0x2222_3333);
    }
}
