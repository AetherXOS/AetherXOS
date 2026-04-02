pub fn runtime_public_header(entrypoints: &[&'static str]) -> String {
    let mut lines = vec![
        "/* aethercore runtime public header */".to_string(),
        "#pragma once".to_string(),
        String::new(),
    ];
    for entrypoint in entrypoints {
        lines.push(format!("void {}(void);", entrypoint));
    }
    lines.extend(
        [
            String::new(),
            "unsigned long aethercore_auxv_execfn(void);".to_string(),
            "unsigned long aethercore_auxv_sysinfo_ehdr(void);".to_string(),
            "unsigned long aethercore_auxv_pagesz(void);".to_string(),
            "unsigned long aethercore_auxv_random(void);".to_string(),
            "int aethercore_auxv_ready(void);".to_string(),
            "const char **aethercore_runtime_env_keys(void);".to_string(),
            "const char *aethercore_last_env_name(void);".to_string(),
            "const char *aethercore_last_env_value(void);".to_string(),
            "char *aethercore_getenv(const char *name);".to_string(),
            "void aethercore_kernel_print(const char *message);".to_string(),
            "long aethercore_last_syscall_nr(void);".to_string(),
            "unsigned long aethercore_runtime_init_flags(void);".to_string(),
            "int aethercore_runtime_ready(void);".to_string(),
            "const char *aethercore_runtime_status_word(void);".to_string(),
            "unsigned long aethercore_runtime_probe_mask(void);".to_string(),
            "const char *aethercore_runtime_probe_status_word(void);".to_string(),
            "const char *aethercore_runtime_probe_summary(void);".to_string(),
            String::new(),
        ],
    );
    lines.join("\n")
}

pub fn runtime_state_header() -> String {
    [
        "/* aethercore runtime shared state */",
        "#pragma once",
        "",
        "struct aethercore_auxv_view {",
        "    unsigned long execfn;",
        "    unsigned long sysinfo_ehdr;",
        "    unsigned long pagesz;",
        "    unsigned long random;",
        "};",
        "",
        "struct aethercore_env_view {",
        "    const char **runtime_env_keys;",
        "    const char *last_env_name;",
        "    const char *last_env_value;",
        "};",
        "",
        "extern struct aethercore_auxv_view g_aethercore_auxv_view;",
        "extern struct aethercore_env_view g_aethercore_env_view;",
        "extern long g_aethercore_last_syscall_nr;",
        "extern unsigned long g_aethercore_runtime_init_flags;",
        "",
        "#define AETHERCORE_RUNTIME_INIT_AUXV 0x1u",
        "#define AETHERCORE_RUNTIME_INIT_ENV 0x2u",
        "#define AETHERCORE_RUNTIME_INIT_SYSCALL 0x4u",
        "#define AETHERCORE_RUNTIME_PROBE_READY 0x100u",
        "#define AETHERCORE_RUNTIME_PROBE_AUXV 0x200u",
        "#define AETHERCORE_RUNTIME_PROBE_ENV 0x400u",
        "#define AETHERCORE_RUNTIME_PROBE_STATUS_READY 0x800u",
        "",
    ]
    .join("\n")
}

pub fn libc_state_header() -> String {
    [
        "/* aethercore libc shared state */",
        "#pragma once",
        "#include \"aethercore_libc.h\"",
        "",
        "extern int g_aethercore_errno;",
        "extern const char *g_aethercore_last_getenv_name;",
        "extern const char *g_aethercore_last_getenv_value;",
        "extern const char *g_aethercore_builtin_env[];",
        "extern void *__dso_handle;",
        "extern char *program_invocation_name;",
        "extern char *program_invocation_short_name;",
        "extern char *g_aethercore_environ_store[64];",
        "struct aethercore_cxa_slot {",
        "    aethercore_cxa_fn_t fn;",
        "    void *arg;",
        "    void *dso_handle;",
        "    int active;",
        "};",
        "struct aethercore_on_exit_slot {",
        "    aethercore_on_exit_fn_t fn;",
        "    void *arg;",
        "    int active;",
        "};",
        "struct aethercore_atfork_slot {",
        "    aethercore_atfork_fn_t prepare;",
        "    aethercore_atfork_fn_t parent;",
        "    aethercore_atfork_fn_t child;",
        "    void *dso_handle;",
        "    int active;",
        "};",
        "extern aethercore_atexit_fn_t g_aethercore_atexit_slots[AETHERCORE_ATEXIT_SLOTS];",
        "extern unsigned long g_aethercore_atexit_count;",
        "extern aethercore_atexit_fn_t g_aethercore_quick_exit_slots[AETHERCORE_ATEXIT_SLOTS];",
        "extern unsigned long g_aethercore_quick_exit_count;",
        "extern struct aethercore_on_exit_slot g_aethercore_on_exit_slots[AETHERCORE_ATEXIT_SLOTS];",
        "extern unsigned long g_aethercore_on_exit_count;",
        "extern struct aethercore_atfork_slot g_aethercore_atfork_slots[AETHERCORE_ATEXIT_SLOTS];",
        "extern unsigned long g_aethercore_atfork_count;",
        "extern struct aethercore_cxa_slot g_aethercore_cxa_slots[AETHERCORE_ATEXIT_SLOTS];",
        "extern unsigned long g_aethercore_cxa_count;",
        "extern unsigned long g_aethercore_runtime_fini_attempts;",
        "extern unsigned long g_aethercore_runtime_fini_completed;",
        "extern unsigned long g_aethercore_runtime_fini_deferred;",
        "",
    ]
    .join("\n")
}

pub fn libc_public_header() -> String {
    include_str!("libc_public_header.txt").to_string()
}
