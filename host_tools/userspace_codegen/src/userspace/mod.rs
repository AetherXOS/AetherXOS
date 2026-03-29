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
        "HYPERCORE_RUNTIME_ENTRY",
        "HYPERCORE_RUNTIME_FINI_ENTRY",
        "HYPERCORE_IMAGE_ENTRY",
        "HYPERCORE_IMAGE_BASE",
        "HYPERCORE_PHDR",
        "HYPERCORE_VDSO_BASE",
        "HYPERCORE_VVAR_BASE",
        "HYPERCORE_INIT_HOOKS",
        "HYPERCORE_FINI_HOOKS",
        "HYPERCORE_EXEC_PATH",
        "HYPERCORE_SECURITY_PROFILE",
        "HYPERCORE_SYSCALL_ABI_VERSION",
        "HYPERCORE_SYSCALL_PLATFORM",
        "HYPERCORE_RUNTIME_VDSO",
        "HYPERCORE_RUNTIME_TLS",
        "HYPERCORE_RUNTIME_SIGNAL_ABI",
        "HYPERCORE_RUNTIME_INIT_HOOKS",
        "HYPERCORE_RUNTIME_FINI_HOOKS",
        "HYPERCORE_RUNTIME_FINI_TRAMPOLINE",
        "HYPERCORE_RUNTIME_MULTI_USER",
        "HYPERCORE_RUNTIME_CAPABILITY_MODEL",
    ];
    let syscall_env_keys = vec![
        "HYPERCORE_SYSCALL_READ",
        "HYPERCORE_SYSCALL_WRITE",
        "HYPERCORE_SYSCALL_OPENAT",
        "HYPERCORE_SYSCALL_CLOSE",
        "HYPERCORE_SYSCALL_EXECVE",
        "HYPERCORE_SYSCALL_EXIT",
        "HYPERCORE_SYSCALL_ARCH_PRCTL",
        "HYPERCORE_SYSCALL_GETPID",
        "HYPERCORE_SYSCALL_GETPPID",
        "HYPERCORE_SYSCALL_GETUID",
        "HYPERCORE_SYSCALL_GETEUID",
        "HYPERCORE_SYSCALL_GETGID",
        "HYPERCORE_SYSCALL_GETEGID",
        "HYPERCORE_SYSCALL_CLOCK_GETTIME",
        "HYPERCORE_SYSCALL_GETTIMEOFDAY",
        "HYPERCORE_SYSCALL_TIME",
        "HYPERCORE_SYSCALL_FUTEX",
        "HYPERCORE_SYSCALL_MMAP",
        "HYPERCORE_SYSCALL_MUNMAP",
        "HYPERCORE_SYSCALL_RT_SIGACTION",
        "HYPERCORE_SYSCALL_RT_SIGRETURN",
    ];
    let auxv_env_keys = vec![
        "HYPERCORE_AUXV_AT_BASE",
        "HYPERCORE_AUXV_AT_PHDR",
        "HYPERCORE_AUXV_AT_PHENT",
        "HYPERCORE_AUXV_AT_PHNUM",
        "HYPERCORE_AUXV_AT_PAGESZ",
        "HYPERCORE_AUXV_AT_ENTRY",
        "HYPERCORE_AUXV_AT_UID",
        "HYPERCORE_AUXV_AT_EUID",
        "HYPERCORE_AUXV_AT_GID",
        "HYPERCORE_AUXV_AT_EGID",
        "HYPERCORE_AUXV_AT_PLATFORM",
        "HYPERCORE_AUXV_AT_HWCAP",
        "HYPERCORE_AUXV_AT_CLKTCK",
        "HYPERCORE_AUXV_AT_SECURE",
        "HYPERCORE_AUXV_AT_RANDOM",
        "HYPERCORE_AUXV_AT_EXECFN",
        "HYPERCORE_AUXV_AT_SYSINFO_EHDR",
    ];
    let abi_paths = vec![
        "/proc/sys/hypercore/abi/startup_stack_layout",
        "/proc/sys/hypercore/abi/startup_runtime_contract_env_keys",
        "/proc/sys/hypercore/abi/startup_syscall_env_keys",
        "/proc/sys/hypercore/abi/startup_auxv_env_keys",
        "/proc/sys/hypercore/abi/platform",
        "/proc/sys/hypercore/abi/abi_version_major",
        "/proc/sys/hypercore/abi/abi_version_minor",
        "/proc/sys/hypercore/abi/abi_version_patch",
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
