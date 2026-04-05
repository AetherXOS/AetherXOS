use alloc::vec::Vec;

use super::{NtImportBinding, NtSymbol, NtSymbolTable, PeImportName};

pub fn bind_import_names(
    names: &[PeImportName],
    symbol_table: &NtSymbolTable,
) -> Vec<NtImportBinding> {
    let mut out = Vec::new();
    for name in names {
        let symbol = if contains_ascii_case_insensitive(&name.name, b"IoCreateDevice") {
            Some(NtSymbol::IoCreateDevice)
        } else if contains_ascii_case_insensitive(&name.name, b"IoDeleteDevice") {
            Some(NtSymbol::IoDeleteDevice)
        } else if contains_ascii_case_insensitive(&name.name, b"IoCallDriver") {
            Some(NtSymbol::IoCallDriver)
        } else if contains_ascii_case_insensitive(&name.name, b"IoCompleteRequest") {
            Some(NtSymbol::IoCompleteRequest)
        } else if contains_ascii_case_insensitive(&name.name, b"KeAcquireSpinLock") {
            Some(NtSymbol::KeAcquireSpinLock)
        } else if contains_ascii_case_insensitive(&name.name, b"KeReleaseSpinLock") {
            Some(NtSymbol::KeReleaseSpinLock)
        } else if contains_ascii_case_insensitive(&name.name, b"ExAllocatePool2") {
            Some(NtSymbol::ExAllocatePool2)
        } else if contains_ascii_case_insensitive(&name.name, b"ExFreePool") {
            Some(NtSymbol::ExFreePool)
        } else if contains_ascii_case_insensitive(&name.name, b"MmMapIoSpace") {
            Some(NtSymbol::MmMapIoSpace)
        } else if contains_ascii_case_insensitive(&name.name, b"MmUnmapIoSpace") {
            Some(NtSymbol::MmUnmapIoSpace)
        } else {
            None
        };

        if let Some(symbol) = symbol {
            if let Some(address) = symbol_table.resolve(symbol) {
                out.push(NtImportBinding {
                    descriptor_index: name.descriptor_index,
                    symbol,
                    address,
                });
            }
        }
    }
    out
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
