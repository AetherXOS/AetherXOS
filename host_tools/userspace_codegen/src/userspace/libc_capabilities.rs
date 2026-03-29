pub fn startup_capabilities() -> Vec<&'static str> {
    vec![
        "argv_envp_stack_layout",
        "auxv_delivery",
        "exec_runtime_env",
        "pt_interp",
        "init_hooks",
        "vdso_contract",
        "tls_contract",
    ]
}

pub fn thread_capabilities() -> Vec<&'static str> {
    vec![
        "arch_prctl_fs",
        "set_tid_address",
        "robust_list_tracking",
        "rseq_registration",
        "membarrier_queries",
        "futex_wait_wake",
        "clear_child_tid_wakeup",
        "pt_tls_template",
        "sigset_helper_surface",
    ]
}

pub fn signal_capabilities() -> Vec<&'static str> {
    vec![
        "rt_sigaction",
        "rt_sigreturn",
        "signal_frame_delivery",
        "sa_restorer",
        "signal_mask_tracking",
        "signal_wrapper_surface",
    ]
}

pub fn time_capabilities() -> Vec<&'static str> {
    vec![
        "clock_gettime",
        "clock_getres",
        "gettimeofday",
        "time",
        "vdso_time_fastpath",
    ]
}

pub fn fs_capabilities() -> Vec<&'static str> {
    vec![
        "read",
        "write",
        "pread64",
        "pwrite64",
        "readv",
        "writev",
        "access",
        "faccessat",
        "ioctl",
        "fstat",
        "fstat64",
        "statx",
        "getdents64",
        "directory_stream_surface",
        "stdio_file_surface",
        "metadata_sync_surface",
    ]
}

pub fn memory_capabilities() -> Vec<&'static str> {
    vec![
        "mmap",
        "munmap",
        "mprotect",
        "brk",
        "malloc_contract",
        "calloc_contract",
        "realloc_contract",
        "free_contract",
        "aligned_alloc_contract",
        "posix_memalign_contract",
        "mremap",
        "madvise",
        "msync",
    ]
}

pub fn string_capabilities() -> Vec<&'static str> {
    vec![
        "strlen",
        "strnlen",
        "strcmp",
        "strcasecmp",
        "strncmp",
        "strncasecmp",
        "strcpy",
        "stpcpy",
        "strncpy",
        "strchr",
        "strrchr",
        "strspn",
        "strcspn",
        "strstr",
        "atoi",
        "atol",
        "strtol",
        "strtoul",
        "strtoull",
        "strtok",
        "strdup",
        "basename_dirname",
        "strerror_surface",
        "strsignal_surface",
    ]
}

pub fn errno_model() -> Vec<&'static str> {
    vec![
        "__errno_location",
        "global_errno_storage",
        "negative_syscall_translation",
        "strerror",
        "strerror_r",
    ]
}
