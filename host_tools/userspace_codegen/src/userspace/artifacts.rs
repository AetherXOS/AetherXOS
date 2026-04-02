use std::collections::BTreeMap;

pub fn userspace_contract_files(
    startup_layout: &str,
    runtime_env_keys: &[&str],
    syscall_env_keys: &[&str],
    auxv_env_keys: &[&str],
    abi_paths: &[&str],
) -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "runtime-contract.txt".to_string(),
            [
                "[aethercore-runtime-contract]".to_string(),
                format!("startup_layout={startup_layout}"),
                format!("runtime_env_keys={}", runtime_env_keys.join(",")),
                format!("syscall_env_keys={}", syscall_env_keys.join(",")),
                format!("auxv_env_keys={}", auxv_env_keys.join(",")),
                format!("abi_paths={}", abi_paths.join(",")),
                String::new(),
            ]
            .join("\n"),
        ),
        (
            "runtime-env-keys.txt".to_string(),
            format!("{}\n", runtime_env_keys.join("\n")),
        ),
        (
            "syscall-env-keys.txt".to_string(),
            format!("{}\n", syscall_env_keys.join("\n")),
        ),
        (
            "auxv-env-keys.txt".to_string(),
            format!("{}\n", auxv_env_keys.join("\n")),
        ),
    ])
}

pub fn runtime_files(
    helpers: &[&str],
    memory_helpers: &[&str],
    string_helpers: &[&str],
    auxv_helpers: &[&str],
    env_helpers: &[&str],
    errno_features: &[&str],
    entrypoints: &[&str],
    source_units: &[&str],
    wrappers: &[&str],
    startup_features: &[&str],
    exported_symbols: &[&str],
    public_header: &str,
    state_header: &str,
) -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "runtime-core.txt".to_string(),
            [
                "[aethercore-runtime-core]".to_string(),
                format!("helpers={}", helpers.join(",")),
                format!("memory_helpers={}", memory_helpers.join(",")),
                format!("string_helpers={}", string_helpers.join(",")),
                format!("auxv_helpers={}", auxv_helpers.join(",")),
                format!("env_helpers={}", env_helpers.join(",")),
                format!("errno_features={}", errno_features.join(",")),
                format!("entrypoints={}", entrypoints.join(",")),
                format!("source_units={}", source_units.join(",")),
                format!("wrappers={}", wrappers.join(",")),
                format!("startup_features={}", startup_features.join(",")),
                String::new(),
            ]
            .join("\n"),
        ),
        ("runtime-helpers.txt".to_string(), format!("{}\n", helpers.join("\n"))),
        (
            "runtime-memory-helpers.txt".to_string(),
            format!("{}\n", memory_helpers.join("\n")),
        ),
        (
            "runtime-string-helpers.txt".to_string(),
            format!("{}\n", string_helpers.join("\n")),
        ),
        (
            "runtime-auxv-helpers.txt".to_string(),
            format!("{}\n", auxv_helpers.join("\n")),
        ),
        (
            "runtime-env-helpers.txt".to_string(),
            format!("{}\n", env_helpers.join("\n")),
        ),
        (
            "runtime-errno-features.txt".to_string(),
            format!("{}\n", errno_features.join("\n")),
        ),
        (
            "runtime-entrypoints.txt".to_string(),
            format!("{}\n", entrypoints.join("\n")),
        ),
        (
            "runtime-exported-symbols.txt".to_string(),
            format!("{}\n", exported_symbols.join("\n")),
        ),
        (
            "runtime-source-units.txt".to_string(),
            format!("{}\n", source_units.join("\n")),
        ),
        (
            "runtime-wrappers.txt".to_string(),
            format!("{}\n", wrappers.join("\n")),
        ),
        (
            "startup-features.txt".to_string(),
            format!("{}\n", startup_features.join("\n")),
        ),
        ("aethercore_runtime.h".to_string(), format!("{public_header}\n")),
        (
            "aethercore_runtime_state.h".to_string(),
            format!("{state_header}\n"),
        ),
    ])
}

pub fn elf_files(
    elf_class: &str,
    elf_endianness: &str,
    machine_targets: &[&str],
    loader_features: &[&str],
    relocation_families: &[&str],
    dynamic_tags: &[&str],
) -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "elf-contract.txt".to_string(),
            [
                "[aethercore-elf-runtime]".to_string(),
                format!("class={elf_class}"),
                format!("endianness={elf_endianness}"),
                format!("machine_targets={}", machine_targets.join(",")),
                format!("loader_features={}", loader_features.join(",")),
                format!("relocation_families={}", relocation_families.join(",")),
                format!("dynamic_tags={}", dynamic_tags.join(",")),
                String::new(),
            ]
            .join("\n"),
        ),
        (
            "elf-loader-features.txt".to_string(),
            format!("{}\n", loader_features.join("\n")),
        ),
        (
            "elf-relocations.txt".to_string(),
            format!("{}\n", relocation_families.join("\n")),
        ),
        (
            "elf-dynamic-tags.txt".to_string(),
            format!("{}\n", dynamic_tags.join("\n")),
        ),
    ])
}

#[allow(clippy::too_many_arguments)]
pub fn libc_files(
    startup_capabilities: &[&str],
    thread_capabilities: &[&str],
    signal_capabilities: &[&str],
    time_capabilities: &[&str],
    fs_capabilities: &[&str],
    memory_capabilities: &[&str],
    string_capabilities: &[&str],
    errno_model: &[&str],
    planned_symbols: &[&str],
    source_modules: &[&str],
    syscall_surface: &[&str],
    exported_symbols: &[&str],
    public_header: &str,
    state_header: &str,
) -> BTreeMap<String, String> {
    let fini_telemetry = [
        "aethercore_runtime_fini_present",
        "aethercore_runtime_fini_hook_count",
        "aethercore_runtime_fini_attempt_count",
        "aethercore_runtime_fini_completed_count",
        "aethercore_runtime_fini_deferred_count",
        "aethercore_run_runtime_fini",
    ];
    let exported: std::collections::BTreeSet<&str> = exported_symbols.iter().copied().collect();
    let present: Vec<&str> = fini_telemetry
        .into_iter()
        .filter(|symbol| exported.contains(symbol))
        .collect();

    BTreeMap::from([
        (
            "libc-contract.txt".to_string(),
            [
                "[aethercore-libc-runtime]".to_string(),
                format!("startup_capabilities={}", startup_capabilities.join(",")),
                format!("thread_capabilities={}", thread_capabilities.join(",")),
                format!("signal_capabilities={}", signal_capabilities.join(",")),
                format!("time_capabilities={}", time_capabilities.join(",")),
                format!("fs_capabilities={}", fs_capabilities.join(",")),
                format!("memory_capabilities={}", memory_capabilities.join(",")),
                format!("string_capabilities={}", string_capabilities.join(",")),
                format!("errno_model={}", errno_model.join(",")),
                format!("planned_symbols={}", planned_symbols.join(",")),
                format!("source_modules={}", source_modules.join(",")),
                format!("syscall_surface={}", syscall_surface.join(",")),
                String::new(),
            ]
            .join("\n"),
        ),
        (
            "libc-startup-capabilities.txt".to_string(),
            format!("{}\n", startup_capabilities.join("\n")),
        ),
        (
            "libc-thread-capabilities.txt".to_string(),
            format!("{}\n", thread_capabilities.join("\n")),
        ),
        (
            "libc-signal-capabilities.txt".to_string(),
            format!("{}\n", signal_capabilities.join("\n")),
        ),
        (
            "libc-time-capabilities.txt".to_string(),
            format!("{}\n", time_capabilities.join("\n")),
        ),
        (
            "libc-fs-capabilities.txt".to_string(),
            format!("{}\n", fs_capabilities.join("\n")),
        ),
        (
            "libc-memory-capabilities.txt".to_string(),
            format!("{}\n", memory_capabilities.join("\n")),
        ),
        (
            "libc-string-capabilities.txt".to_string(),
            format!("{}\n", string_capabilities.join("\n")),
        ),
        (
            "libc-errno-model.txt".to_string(),
            format!("{}\n", errno_model.join("\n")),
        ),
        (
            "libc-planned-symbols.txt".to_string(),
            format!("{}\n", planned_symbols.join("\n")),
        ),
        (
            "libc-exported-symbols.txt".to_string(),
            format!("{}\n", exported_symbols.join("\n")),
        ),
        (
            "libc-source-modules.txt".to_string(),
            format!("{}\n", source_modules.join("\n")),
        ),
        (
            "libc-syscall-surface.txt".to_string(),
            format!("{}\n", syscall_surface.join("\n")),
        ),
        ("aethercore_libc.h".to_string(), public_header.to_string()),
        (
            "aethercore_libc_state.h".to_string(),
            format!("{state_header}\n"),
        ),
        (
            "runtime-fini-telemetry.txt".to_string(),
            [
                "[aethercore-runtime-fini-telemetry]".to_string(),
                format!("symbols={}", present.join(",")),
                format!(
                    "present={}",
                    if present.len() == 6 { "yes" } else { "partial" }
                ),
                "expected_runtime_flow=attempt_count>=completed_count and attempt_count>=deferred_count".to_string(),
                "success_signal=completed_count>0 when trampoline executes".to_string(),
                "deferred_signal=deferred_count>0 when hooks remain pending".to_string(),
                String::new(),
            ]
            .join("\n"),
        ),
    ])
}
