use crate::models::LibcSnapshot;
use std::collections::BTreeMap;
use super::{artifacts, headers, libc_capabilities, libc_surfaces, sources};

pub fn libc_snapshot() -> LibcSnapshot {
    let startup_capabilities = libc_capabilities::startup_capabilities();
    let thread_capabilities = libc_capabilities::thread_capabilities();
    let signal_capabilities = libc_capabilities::signal_capabilities();
    let time_capabilities = libc_capabilities::time_capabilities();
    let fs_capabilities = libc_capabilities::fs_capabilities();
    let memory_capabilities = libc_capabilities::memory_capabilities();
    let string_capabilities = libc_capabilities::string_capabilities();
    let errno_model = libc_capabilities::errno_model();
    let planned_symbols = libc_surfaces::planned_symbols();
    let source_modules = libc_surfaces::source_modules();
    let syscall_surface = libc_surfaces::syscall_surface();
    let public_header = headers::libc_public_header();
    let state_header = headers::libc_state_header();
    LibcSnapshot {
        startup_capabilities: startup_capabilities.clone(),
        thread_capabilities: thread_capabilities.clone(),
        signal_capabilities: signal_capabilities.clone(),
        time_capabilities: time_capabilities.clone(),
        fs_capabilities: fs_capabilities.clone(),
        memory_capabilities: memory_capabilities.clone(),
        string_capabilities: string_capabilities.clone(),
        errno_model: errno_model.clone(),
        planned_symbols: planned_symbols.clone(),
        source_modules: source_modules.clone(),
        syscall_surface: syscall_surface.clone(),
        exported_symbols: planned_symbols.clone(),
        public_header: public_header.clone(),
        state_header: state_header.clone(),
        artifact_files: artifacts::libc_files(
            &startup_capabilities,
            &thread_capabilities,
            &signal_capabilities,
            &time_capabilities,
            &fs_capabilities,
            &memory_capabilities,
            &string_capabilities,
            &errno_model,
            &planned_symbols,
            &source_modules,
            &syscall_surface,
            &planned_symbols,
            &public_header,
            &state_header,
        ),
        source_blobs: BTreeMap::from([
            ("libc_state.c".to_string(), sources::libc_state_source()),
            ("startup_runtime.c".to_string(), sources::startup_runtime_source()),
            ("errno_runtime.c".to_string(), sources::errno_runtime_source()),
            ("libc_syscall.c".to_string(), sources::libc_syscall_source()),
            ("memory_runtime.c".to_string(), sources::memory_runtime_source()),
            ("string_runtime.c".to_string(), sources::string_runtime_source()),
        ]),
    }
}
