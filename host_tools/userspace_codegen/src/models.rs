use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
pub struct CodegenSnapshot {
    pub config: ConfigSnapshot,
    pub userspace: UserspaceSnapshot,
}

#[derive(Debug, Serialize)]
pub struct ConfigSnapshot {
    pub linux_compat: BTreeMap<String, String>,
    pub linux_os: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct UserspaceSnapshot {
    pub startup_layout: &'static str,
    pub runtime_env_keys: Vec<&'static str>,
    pub syscall_env_keys: Vec<&'static str>,
    pub auxv_env_keys: Vec<&'static str>,
    pub abi_paths: Vec<&'static str>,
    pub artifact_files: BTreeMap<String, String>,
    pub programs: Vec<ProgramSnapshot>,
    pub build_files: BTreeMap<String, String>,
    pub runtime: RuntimeSnapshot,
    pub elf: ElfSnapshot,
    pub libc: LibcSnapshot,
}

#[derive(Debug, Serialize)]
pub struct ProgramSnapshot {
    pub output_name: &'static str,
    pub messages: Vec<&'static str>,
    pub candidates: Vec<&'static str>,
    pub role: &'static str,
    pub probe_features: Vec<&'static str>,
    pub source_units: Vec<&'static str>,
    pub source_blobs: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct RuntimeSnapshot {
    pub helpers: Vec<&'static str>,
    pub memory_helpers: Vec<&'static str>,
    pub string_helpers: Vec<&'static str>,
    pub auxv_helpers: Vec<&'static str>,
    pub env_helpers: Vec<&'static str>,
    pub errno_features: Vec<&'static str>,
    pub entrypoints: Vec<&'static str>,
    pub source_units: Vec<&'static str>,
    pub wrappers: Vec<&'static str>,
    pub startup_features: Vec<&'static str>,
    pub exported_symbols: Vec<&'static str>,
    pub public_header: String,
    pub state_header: String,
    pub artifact_files: BTreeMap<String, String>,
    pub source_blobs: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct ElfSnapshot {
    pub elf_class: &'static str,
    pub elf_endianness: &'static str,
    pub machine_targets: Vec<&'static str>,
    pub loader_features: Vec<&'static str>,
    pub relocation_families: Vec<&'static str>,
    pub dynamic_tags: Vec<&'static str>,
    pub artifact_files: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct LibcSnapshot {
    pub startup_capabilities: Vec<&'static str>,
    pub thread_capabilities: Vec<&'static str>,
    pub signal_capabilities: Vec<&'static str>,
    pub time_capabilities: Vec<&'static str>,
    pub fs_capabilities: Vec<&'static str>,
    pub memory_capabilities: Vec<&'static str>,
    pub string_capabilities: Vec<&'static str>,
    pub errno_model: Vec<&'static str>,
    pub planned_symbols: Vec<&'static str>,
    pub source_modules: Vec<&'static str>,
    pub syscall_surface: Vec<&'static str>,
    pub exported_symbols: Vec<&'static str>,
    pub public_header: String,
    pub state_header: String,
    pub artifact_files: BTreeMap<String, String>,
    pub source_blobs: BTreeMap<String, String>,
}
