use super::{
    NtExecutionPolicy, NtImportBinding, NtImportDomain, NtImportDomainCounts,
    NtImportResolutionReport, NtSymbol, NtSymbolTable, PeImportName,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NtImportReadinessLevel {
    Ready,
    Partial,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NtImportReadinessReport {
    pub total_imports: usize,
    pub resolved_imports: usize,
    pub unknown_domain_imports: usize,
    pub win32_imports: usize,
    pub readiness: NtImportReadinessLevel,
    pub binding_ratio_pct: u8,
}

pub fn classify_import_dll(dll: &[u8]) -> NtImportDomain {
    if contains_ascii_case_insensitive(dll, b"ntoskrnl") {
        NtImportDomain::Kernel
    } else if contains_ascii_case_insensitive(dll, b"hal") {
        NtImportDomain::Hal
    } else if contains_ascii_case_insensitive(dll, b"win32k")
        || contains_ascii_case_insensitive(dll, b"user32")
        || contains_ascii_case_insensitive(dll, b"gdi32")
    {
        NtImportDomain::Win32
    } else {
        NtImportDomain::Unknown
    }
}

pub fn recommended_policy_for_imports(names: &[PeImportName]) -> NtExecutionPolicy {
    let mut saw_kernelish = false;
    for name in names {
        match classify_import_dll(&name.dll) {
            NtImportDomain::Win32 => return NtExecutionPolicy::wine_bridge(),
            NtImportDomain::Kernel | NtImportDomain::Hal => saw_kernelish = true,
            NtImportDomain::Unknown => {}
        }
    }

    if saw_kernelish {
        NtExecutionPolicy::native()
    } else {
        NtExecutionPolicy::wine_bridge()
    }
}

pub fn summarize_import_domains(names: &[PeImportName]) -> NtImportDomainCounts {
    let mut counts = NtImportDomainCounts {
        kernel: 0,
        hal: 0,
        win32: 0,
        unknown: 0,
    };
    for name in names {
        match classify_import_dll(&name.dll) {
            NtImportDomain::Kernel => counts.kernel += 1,
            NtImportDomain::Hal => counts.hal += 1,
            NtImportDomain::Win32 => counts.win32 += 1,
            NtImportDomain::Unknown => counts.unknown += 1,
        }
    }
    counts
}

pub fn bind_import_names_with_domain_tables(
    names: &[PeImportName],
    symbol_table: &NtSymbolTable,
) -> Vec<super::NtDomainImportBinding> {
    let mut out = Vec::new();
    for name in names {
        let domain = classify_import_dll(&name.dll);
        let candidates: &[NtSymbol] = match domain {
            NtImportDomain::Kernel => &[
                NtSymbol::IoCreateDevice,
                NtSymbol::IoDeleteDevice,
                NtSymbol::IoCallDriver,
                NtSymbol::IoCompleteRequest,
                NtSymbol::KeAcquireSpinLock,
                NtSymbol::KeReleaseSpinLock,
            ],
            NtImportDomain::Hal => &[
                NtSymbol::MmMapIoSpace,
                NtSymbol::MmUnmapIoSpace,
                NtSymbol::KeAcquireSpinLock,
                NtSymbol::KeReleaseSpinLock,
            ],
            NtImportDomain::Win32 => &[
                NtSymbol::IoCallDriver,
                NtSymbol::IoCompleteRequest,
                NtSymbol::MmMapIoSpace,
                NtSymbol::MmUnmapIoSpace,
            ],
            NtImportDomain::Unknown => &[
                NtSymbol::IoCallDriver,
                NtSymbol::IoCompleteRequest,
                NtSymbol::MmMapIoSpace,
                NtSymbol::MmUnmapIoSpace,
            ],
        };

        let mut bound = None;
        for candidate in candidates {
            if let Some(address) = symbol_table.resolve(*candidate) {
                bound = Some(NtImportBinding {
                    descriptor_index: name.descriptor_index,
                    symbol: *candidate,
                    address,
                });
                break;
            }
        }

        if let Some(binding) = bound {
            out.push(super::NtDomainImportBinding { domain, binding });
        }
    }
    out
}

pub fn build_import_resolution_report(
    names: &[PeImportName],
    symbol_table: &NtSymbolTable,
) -> NtImportResolutionReport {
    let policy = recommended_policy_for_imports(names);
    let counts = summarize_import_domains(names);
    let bindings = bind_import_names_with_domain_tables(names, symbol_table);
    NtImportResolutionReport {
        policy,
        counts,
        bindings,
    }
}

pub fn import_readiness_report(
    names: &[PeImportName],
    symbol_table: &NtSymbolTable,
) -> NtImportReadinessReport {
    if names.is_empty() {
        return NtImportReadinessReport {
            total_imports: 0,
            resolved_imports: 0,
            unknown_domain_imports: 0,
            win32_imports: 0,
            readiness: NtImportReadinessLevel::Ready,
            binding_ratio_pct: 100,
        };
    }

    let counts = summarize_import_domains(names);
    let bindings = bind_import_names_with_domain_tables(names, symbol_table);
    let resolved_imports = bindings.len().min(names.len());
    let binding_ratio_pct = ((resolved_imports * 100) / names.len()) as u8;

    let readiness = if binding_ratio_pct >= 80 && counts.unknown == 0 {
        NtImportReadinessLevel::Ready
    } else if binding_ratio_pct >= 50 {
        NtImportReadinessLevel::Partial
    } else {
        NtImportReadinessLevel::Blocked
    };

    NtImportReadinessReport {
        total_imports: names.len(),
        resolved_imports,
        unknown_domain_imports: counts.unknown,
        win32_imports: counts.win32,
        readiness,
        binding_ratio_pct,
    }
}

fn contains_ascii_case_insensitive(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.len() < needle.len() {
        return false;
    }
    let mut i = 0usize;
    while i + needle.len() <= haystack.len() {
        let mut j = 0usize;
        let mut ok = true;
        while j < needle.len() {
            let a = haystack[i + j].to_ascii_lowercase();
            let b = needle[j].to_ascii_lowercase();
            if a != b {
                ok = false;
                break;
            }
            j += 1;
        }
        if ok {
            return true;
        }
        i += 1;
    }
    false
}
