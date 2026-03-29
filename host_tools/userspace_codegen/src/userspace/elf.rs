pub fn machine_targets() -> Vec<&'static str> {
    vec!["x86_64", "aarch64"]
}

pub fn loader_features() -> Vec<&'static str> {
    vec![
        "pt_interp",
        "dt_needed",
        "dt_rpath",
        "dt_runpath",
        "dt_gnu_hash",
        "dt_hash",
        "dt_init",
        "dt_init_array",
        "dt_fini",
        "dt_fini_array_tracking",
        "pt_tls",
        "vdso",
    ]
}

pub fn relocation_families() -> Vec<&'static str> {
    vec![
        "relative",
        "glob_dat",
        "jmp_slot",
        "plt32",
        "pc32",
        "got32",
        "gotpcrel",
        "gotpcrelx",
        "tls_local_exec",
        "tls_dtpmod",
        "tls_dtpoff",
        "irelative_best_effort",
        "copy_best_effort",
    ]
}

pub fn dynamic_tags() -> Vec<&'static str> {
    vec![
        "DT_NEEDED",
        "DT_RPATH",
        "DT_RUNPATH",
        "DT_SONAME",
        "DT_STRTAB",
        "DT_STRSZ",
        "DT_SYMTAB",
        "DT_SYMENT",
        "DT_HASH",
        "DT_GNU_HASH",
        "DT_RELA",
        "DT_RELASZ",
        "DT_INIT",
        "DT_INIT_ARRAY",
        "DT_FINI",
        "DT_FINI_ARRAY",
        "DT_FLAGS_1",
        "DT_VERSYM",
    ]
}
