mod libc;
mod libc_capabilities;
mod libc_surfaces;
mod artifacts;
mod emit;
mod headers;
mod program_build;
mod program_binaries;
mod program_sources;
mod smoke_boot;
mod smoke_detect;
mod smoke_proof;
mod smoke;
mod sources;
mod runtime;
mod runtime_core;
mod elf;

use crate::models::UserspaceSnapshot;
use std::path::Path;

pub fn userspace_snapshot() -> UserspaceSnapshot {
    let runtime_env_keys = vec![
        "AETHERCORE_RUNTIME_ENTRY",
        "AETHERCORE_RUNTIME_FINI_ENTRY",
        "AETHERCORE_IMAGE_ENTRY",
        "AETHERCORE_IMAGE_BASE",
        "AETHERCORE_PHDR",
        "AETHERCORE_VDSO_BASE",
        "AETHERCORE_VVAR_BASE",
        "AETHERCORE_INIT_HOOKS",
        "AETHERCORE_FINI_HOOKS",
        "AETHERCORE_EXEC_PATH",
        "AETHERCORE_SECURITY_PROFILE",
        "AETHERCORE_SYSCALL_ABI_VERSION",
        "AETHERCORE_SYSCALL_PLATFORM",
        "AETHERCORE_RUNTIME_VDSO",
        "AETHERCORE_RUNTIME_TLS",
        "AETHERCORE_RUNTIME_SIGNAL_ABI",
        "AETHERCORE_RUNTIME_INIT_HOOKS",
        "AETHERCORE_RUNTIME_FINI_HOOKS",
        "AETHERCORE_RUNTIME_FINI_TRAMPOLINE",
        "AETHERCORE_RUNTIME_MULTI_USER",
        "AETHERCORE_RUNTIME_CAPABILITY_MODEL",
    ];
    let syscall_env_keys = vec![
        "AETHERCORE_SYSCALL_READ",
        "AETHERCORE_SYSCALL_WRITE",
        "AETHERCORE_SYSCALL_OPENAT",
        "AETHERCORE_SYSCALL_CLOSE",
        "AETHERCORE_SYSCALL_EXECVE",
        "AETHERCORE_SYSCALL_EXIT",
        "AETHERCORE_SYSCALL_ARCH_PRCTL",
        "AETHERCORE_SYSCALL_GETPID",
        "AETHERCORE_SYSCALL_GETPPID",
        "AETHERCORE_SYSCALL_GETUID",
        "AETHERCORE_SYSCALL_GETEUID",
        "AETHERCORE_SYSCALL_GETGID",
        "AETHERCORE_SYSCALL_GETEGID",
        "AETHERCORE_SYSCALL_CLOCK_GETTIME",
        "AETHERCORE_SYSCALL_GETTIMEOFDAY",
        "AETHERCORE_SYSCALL_TIME",
        "AETHERCORE_SYSCALL_FUTEX",
        "AETHERCORE_SYSCALL_MMAP",
        "AETHERCORE_SYSCALL_MUNMAP",
        "AETHERCORE_SYSCALL_RT_SIGACTION",
        "AETHERCORE_SYSCALL_RT_SIGRETURN",
    ];
    let auxv_env_keys = vec![
        "AETHERCORE_AUXV_AT_BASE",
        "AETHERCORE_AUXV_AT_PHDR",
        "AETHERCORE_AUXV_AT_PHENT",
        "AETHERCORE_AUXV_AT_PHNUM",
        "AETHERCORE_AUXV_AT_PAGESZ",
        "AETHERCORE_AUXV_AT_ENTRY",
        "AETHERCORE_AUXV_AT_UID",
        "AETHERCORE_AUXV_AT_EUID",
        "AETHERCORE_AUXV_AT_GID",
        "AETHERCORE_AUXV_AT_EGID",
        "AETHERCORE_AUXV_AT_PLATFORM",
        "AETHERCORE_AUXV_AT_HWCAP",
        "AETHERCORE_AUXV_AT_CLKTCK",
        "AETHERCORE_AUXV_AT_SECURE",
        "AETHERCORE_AUXV_AT_RANDOM",
        "AETHERCORE_AUXV_AT_EXECFN",
        "AETHERCORE_AUXV_AT_SYSINFO_EHDR",
    ];
    let abi_paths = vec![
        "/proc/sys/aethercore/abi/startup_stack_layout",
        "/proc/sys/aethercore/abi/startup_runtime_contract_env_keys",
        "/proc/sys/aethercore/abi/startup_syscall_env_keys",
        "/proc/sys/aethercore/abi/startup_auxv_env_keys",
        "/proc/sys/aethercore/abi/platform",
        "/proc/sys/aethercore/abi/abi_version_major",
        "/proc/sys/aethercore/abi/abi_version_minor",
        "/proc/sys/aethercore/abi/abi_version_patch",
    ];
    UserspaceSnapshot {
        startup_layout: "argc|argv|null|envp|null|auxv",
        runtime_env_keys: runtime_env_keys.clone(),
        syscall_env_keys: syscall_env_keys.clone(),
        auxv_env_keys: auxv_env_keys.clone(),
        abi_paths: abi_paths.clone(),
        artifact_files: artifacts::userspace_contract_files(
            "argc|argv|null|envp|null|auxv",
            &runtime_env_keys,
            &syscall_env_keys,
            &auxv_env_keys,
            &abi_paths,
        ),
        programs: program_sources::programs(),
        build_files: program_build::build_files(&program_sources::programs()),
        runtime: runtime::runtime_snapshot(),
        elf: runtime::elf_snapshot(),
        libc: libc::libc_snapshot(),
    }
}

pub fn emit_userspace_dir(snapshot: &UserspaceSnapshot, dir: &Path) -> Result<(), String> {
    emit::emit_userspace_dir(snapshot, dir)
}

pub fn run_generated_userspace_smoke(repo_root: &Path, dir: &Path) -> Result<(), String> {
    smoke::run_generated_userspace_smoke(repo_root, dir)
}
