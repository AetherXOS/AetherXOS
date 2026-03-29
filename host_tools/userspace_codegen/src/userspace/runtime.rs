use crate::models::{ElfSnapshot, RuntimeSnapshot};
use std::collections::BTreeMap;
use super::{artifacts, elf, headers, runtime_core, sources};

pub fn runtime_snapshot() -> RuntimeSnapshot {
    let entrypoints = runtime_core::entrypoints();
    let helpers = runtime_core::helpers();
    let memory_helpers = runtime_core::memory_helpers();
    let string_helpers = runtime_core::string_helpers();
    let auxv_helpers = runtime_core::auxv_helpers();
    let env_helpers = runtime_core::env_helpers();
    let errno_features = runtime_core::errno_features();
    let source_units = runtime_core::source_units();
    let wrappers = runtime_core::wrappers();
    let startup_features = runtime_core::startup_features();
    let exported_symbols: Vec<&'static str> = runtime_core::entrypoints()
        .into_iter()
        .chain(runtime_core::helpers())
        .chain(runtime_core::wrappers())
        .collect();
    let public_header = headers::runtime_public_header(&entrypoints);
    let state_header = headers::runtime_state_header();
    RuntimeSnapshot {
        helpers: helpers.clone(),
        memory_helpers: memory_helpers.clone(),
        string_helpers: string_helpers.clone(),
        auxv_helpers: auxv_helpers.clone(),
        env_helpers: env_helpers.clone(),
        errno_features: errno_features.clone(),
        entrypoints: entrypoints.clone(),
        source_units: source_units.clone(),
        wrappers: wrappers.clone(),
        startup_features: startup_features.clone(),
        exported_symbols: exported_symbols.clone(),
        public_header: public_header.clone(),
        state_header: state_header.clone(),
        artifact_files: artifacts::runtime_files(
            &helpers,
            &memory_helpers,
            &string_helpers,
            &auxv_helpers,
            &env_helpers,
            &errno_features,
            &entrypoints,
            &source_units,
            &wrappers,
            &startup_features,
            &exported_symbols,
            &public_header,
            &state_header,
        ),
        source_blobs: BTreeMap::from([
            ("crt0.S".to_string(), sources::crt0_source()),
            ("runtime_state.c".to_string(), sources::runtime_state_source()),
            ("auxv_runtime.c".to_string(), sources::auxv_runtime_source()),
            ("env_runtime.c".to_string(), sources::env_runtime_source()),
            ("runtime_syscall.c".to_string(), sources::runtime_syscall_source()),
            ("runtime_entry.c".to_string(), sources::runtime_entry_source()),
            ("runtime_probe.c".to_string(), sources::runtime_probe_source()),
            ("runtime_smoke.c".to_string(), sources::runtime_smoke_source()),
        ]),
    }
}

pub fn elf_snapshot() -> ElfSnapshot {
    let machine_targets = elf::machine_targets();
    let loader_features = elf::loader_features();
    let relocation_families = elf::relocation_families();
    let dynamic_tags = elf::dynamic_tags();
    ElfSnapshot {
        elf_class: "ELF64",
        elf_endianness: "little",
        machine_targets: machine_targets.clone(),
        loader_features: loader_features.clone(),
        relocation_families: relocation_families.clone(),
        dynamic_tags: dynamic_tags.clone(),
        artifact_files: artifacts::elf_files(
            "ELF64",
            "little",
            &machine_targets,
            &loader_features,
            &relocation_families,
            &dynamic_tags,
        ),
    }
}
