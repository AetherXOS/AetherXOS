pub fn runtime_public_header(entrypoints: &[&'static str]) -> String {
    let mut lines = vec![
        "/* hypercore runtime public header */".to_string(),
        "#pragma once".to_string(),
        String::new(),
    ];
    for entrypoint in entrypoints {
        lines.push(format!("void {}(void);", entrypoint));
    }
    lines.extend(
        [
            String::new(),
            "unsigned long hypercore_auxv_execfn(void);".to_string(),
            "unsigned long hypercore_auxv_sysinfo_ehdr(void);".to_string(),
            "unsigned long hypercore_auxv_pagesz(void);".to_string(),
            "unsigned long hypercore_auxv_random(void);".to_string(),
            "int hypercore_auxv_ready(void);".to_string(),
            "const char **hypercore_runtime_env_keys(void);".to_string(),
            "const char *hypercore_last_env_name(void);".to_string(),
            "const char *hypercore_last_env_value(void);".to_string(),
            "char *hypercore_getenv(const char *name);".to_string(),
            "void hypercore_kernel_print(const char *message);".to_string(),
            "long hypercore_last_syscall_nr(void);".to_string(),
            "unsigned long hypercore_runtime_init_flags(void);".to_string(),
            "int hypercore_runtime_ready(void);".to_string(),
            "const char *hypercore_runtime_status_word(void);".to_string(),
            "unsigned long hypercore_runtime_probe_mask(void);".to_string(),
            "const char *hypercore_runtime_probe_status_word(void);".to_string(),
            "const char *hypercore_runtime_probe_summary(void);".to_string(),
            String::new(),
        ],
    );
    lines.join("\n")
}

pub fn runtime_state_header() -> String {
    [
        "/* hypercore runtime shared state */",
        "#pragma once",
        "",
        "struct hypercore_auxv_view {",
        "    unsigned long execfn;",
        "    unsigned long sysinfo_ehdr;",
        "    unsigned long pagesz;",
        "    unsigned long random;",
        "};",
        "",
        "struct hypercore_env_view {",
        "    const char **runtime_env_keys;",
        "    const char *last_env_name;",
        "    const char *last_env_value;",
        "};",
        "",
        "extern struct hypercore_auxv_view g_hypercore_auxv_view;",
        "extern struct hypercore_env_view g_hypercore_env_view;",
        "extern long g_hypercore_last_syscall_nr;",
        "extern unsigned long g_hypercore_runtime_init_flags;",
        "",
        "#define HYPERCORE_RUNTIME_INIT_AUXV 0x1u",
        "#define HYPERCORE_RUNTIME_INIT_ENV 0x2u",
        "#define HYPERCORE_RUNTIME_INIT_SYSCALL 0x4u",
        "#define HYPERCORE_RUNTIME_PROBE_READY 0x100u",
        "#define HYPERCORE_RUNTIME_PROBE_AUXV 0x200u",
        "#define HYPERCORE_RUNTIME_PROBE_ENV 0x400u",
        "#define HYPERCORE_RUNTIME_PROBE_STATUS_READY 0x800u",
        "",
    ]
    .join("\n")
}

pub fn libc_state_header() -> String {
    [
        "/* hypercore libc shared state */",
        "#pragma once",
        "#include \"hypercore_libc.h\"",
        "",
        "extern int g_hypercore_errno;",
        "extern const char *g_hypercore_last_getenv_name;",
        "extern const char *g_hypercore_last_getenv_value;",
        "extern const char *g_hypercore_builtin_env[];",
        "extern void *__dso_handle;",
        "extern char *program_invocation_name;",
        "extern char *program_invocation_short_name;",
        "extern char *g_hypercore_environ_store[64];",
        "struct hypercore_cxa_slot {",
        "    hypercore_cxa_fn_t fn;",
        "    void *arg;",
        "    void *dso_handle;",
        "    int active;",
        "};",
        "struct hypercore_on_exit_slot {",
        "    hypercore_on_exit_fn_t fn;",
        "    void *arg;",
        "    int active;",
        "};",
        "struct hypercore_atfork_slot {",
        "    hypercore_atfork_fn_t prepare;",
        "    hypercore_atfork_fn_t parent;",
        "    hypercore_atfork_fn_t child;",
        "    void *dso_handle;",
        "    int active;",
        "};",
        "extern hypercore_atexit_fn_t g_hypercore_atexit_slots[HYPERCORE_ATEXIT_SLOTS];",
        "extern unsigned long g_hypercore_atexit_count;",
        "extern hypercore_atexit_fn_t g_hypercore_quick_exit_slots[HYPERCORE_ATEXIT_SLOTS];",
        "extern unsigned long g_hypercore_quick_exit_count;",
        "extern struct hypercore_on_exit_slot g_hypercore_on_exit_slots[HYPERCORE_ATEXIT_SLOTS];",
        "extern unsigned long g_hypercore_on_exit_count;",
        "extern struct hypercore_atfork_slot g_hypercore_atfork_slots[HYPERCORE_ATEXIT_SLOTS];",
        "extern unsigned long g_hypercore_atfork_count;",
        "extern struct hypercore_cxa_slot g_hypercore_cxa_slots[HYPERCORE_ATEXIT_SLOTS];",
        "extern unsigned long g_hypercore_cxa_count;",
        "extern unsigned long g_hypercore_runtime_fini_attempts;",
        "extern unsigned long g_hypercore_runtime_fini_completed;",
        "extern unsigned long g_hypercore_runtime_fini_deferred;",
        "",
    ]
    .join("\n")
}

pub fn libc_public_header() -> String {
    include_str!("libc_public_header.txt").to_string()
}
