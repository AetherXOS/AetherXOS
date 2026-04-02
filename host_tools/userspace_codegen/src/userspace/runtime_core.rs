pub fn helpers() -> Vec<&'static str> {
    vec![
        "strlen",
        "print_hex_value",
        "print_u64_decimal",
        "argc_argv_envp_scan",
        "auxv_scan",
        "vdso_magic_check",
        "runtime_init_state",
        "kernel_print_log_bridge",
        "runtime_status_word",
        "runtime_probe_mask",
        "runtime_probe_status_word",
        "runtime_probe_summary",
    ]
}

pub fn memory_helpers() -> Vec<&'static str> {
    vec![
        "malloc_contract",
        "calloc_contract",
        "realloc_contract",
        "free_contract",
        "bump_allocator_pool",
        "memcpy_contract",
        "memset_contract",
        "bzero_contract",
        "memmove_contract_planned",
        "memcmp_contract",
        "memchr_contract",
        "strdup_pool_contract",
    ]
}

pub fn string_helpers() -> Vec<&'static str> {
    vec![
        "strlen_contract",
        "strnlen_contract",
        "strcmp_contract",
        "strcasecmp_contract",
        "strncmp_contract",
        "strncasecmp_contract",
        "strcpy_contract",
        "stpcpy_contract",
        "strncpy_contract",
        "strchr_contract",
        "strrchr_contract",
        "strspn_contract",
        "strcspn_contract",
        "strstr_contract",
        "atoi_contract",
        "atol_contract",
        "strtol_contract",
        "strtoul_contract",
        "strtoull_contract",
        "strtok_contract",
        "isspace_contract",
        "isdigit_contract",
        "isalpha_contract",
        "isalnum_contract",
        "argv_execfn_scan",
        "auxv_string_slots",
    ]
}

pub fn auxv_helpers() -> Vec<&'static str> {
    vec![
        "auxv_scan",
        "auxv_execfn_lookup",
        "auxv_sysinfo_ehdr_lookup",
        "auxv_pagesz_lookup",
        "auxv_random_lookup",
        "auxv_presence_checks",
    ]
}

pub fn env_helpers() -> Vec<&'static str> {
    vec![
        "runtime_env_contract_keys",
        "syscall_env_key_lookup",
        "auxv_env_key_lookup",
        "default_env_bootstrap",
        "last_env_name_tracking",
        "builtin_env_lookup",
        "last_env_value_tracking",
    ]
}

pub fn errno_features() -> Vec<&'static str> {
    vec![
        "negative_errno_syscall_return",
        "thread_local_errno_planned",
        "errno_wrapper_planned",
        "errno_state_storage",
        "errno_query_api",
    ]
}

pub fn entrypoints() -> Vec<&'static str> {
    vec![
        "_start",
        "__aethercore_crt0_start",
        "__aethercore_auxv_init",
        "__aethercore_env_init",
        "__aethercore_syscall_init",
    ]
}

pub fn source_units() -> Vec<&'static str> {
    vec![
        "crt0.S",
        "runtime_state.c",
        "auxv_runtime.c",
        "env_runtime.c",
        "runtime_syscall.c",
        "runtime_entry.c",
        "runtime_probe.c",
        "runtime_smoke.c",
    ]
}

pub fn wrappers() -> Vec<&'static str> {
    vec![
        "read",
        "write",
        "openat",
        "close",
        "arch_prctl",
        "getpid",
        "gettid",
        "getppid",
        "getuid",
        "geteuid",
        "getgid",
        "getegid",
        "set_tid_address",
        "clock_gettime",
        "clock_getres",
        "gettimeofday",
        "time",
        "futex",
        "mmap",
        "munmap",
        "mprotect",
        "brk",
        "rt_sigaction",
        "rt_sigreturn",
        "execve",
        "exit",
    ]
}

pub fn startup_features() -> Vec<&'static str> {
    vec![
        "argv0_print",
        "execfn_print",
        "argc_print",
        "pagesz_print",
        "random_print",
        "sysinfo_ehdr_print",
        "vdso_elf_probe",
        "startup_status_report",
        "runtime_status_word_report",
        "runtime_probe_mask_report",
        "runtime_probe_summary_report",
        "runtime_probe_kernel_log_report",
    ]
}
