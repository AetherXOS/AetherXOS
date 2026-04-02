use crate::models::ProgramSnapshot;
use std::collections::BTreeMap;

fn init_source_blobs() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "init_main.c".to_string(),
            include_str!("templates/init_main.c.txt").to_string(),
        ),
        (
            "init_chain.c".to_string(),
            include_str!("templates/init_chain.c.txt").to_string(),
        ),
    ])
}

fn probe_source_blobs() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "probe_main.c".to_string(),
            include_str!("templates/probe_main.c.txt").to_string(),
        ),
        (
            "probe_report.c".to_string(),
            include_str!("templates/probe_report.c.txt").to_string(),
        ),
    ])
}

fn console_source_blobs() -> BTreeMap<String, String> {
    BTreeMap::from([(
        "console_main.c".to_string(),
        include_str!("templates/console_main.c.txt").to_string(),
    )])
}

pub fn programs() -> Vec<ProgramSnapshot> {
    vec![
        ProgramSnapshot {
            output_name: "init.elf",
            messages: vec![
                "[aethercore-init] staged userspace init ELF started\n",
                "[aethercore-init] wrappers: read,write,openat,close,getpid,getppid,getuid,geteuid,getgid,getegid,execve,exit\n",
                "[aethercore-init] contract: auxv+vdso+proc/sys abi surface expected\n",
                "[aethercore-init] loader: gnu-hash+runpath+init-hooks runtime: tls+signal-abi+vdso\n",
                "[aethercore-init] startup env: runtime+syscall+auxv key groups exported\n",
            ],
            candidates: vec![
                "/usr/lib/aethercore/probe.elf\0",
                "/usr/lib/aethercore/console.elf\0",
                "/bin/sh\0",
            ],
            role: "bootstrap_init",
            probe_features: vec![
                "startup_stack_scan",
                "auxv_scan",
                "vdso_probe",
                "abi_surface_dump",
                "exec_chain",
            ],
            source_units: vec!["init_main.c", "init_chain.c"],
            source_blobs: init_source_blobs(),
        },
        ProgramSnapshot {
            output_name: "probe.elf",
            messages: vec![
                "[aethercore-probe] runtime probe ELF started\n",
                "[aethercore-probe] goal: validate startup+auxv+vdso abi path\n",
                "[aethercore-probe] expected runtime api: probe_mask+probe_status+probe_summary\n",
            ],
            candidates: vec!["/usr/lib/aethercore/console.elf\0", "/bin/sh\0"],
            role: "runtime_probe",
            probe_features: vec![
                "runtime_probe_mask",
                "runtime_probe_status_word",
                "runtime_probe_summary",
                "auxv_ready_check",
                "env_tracking_check",
            ],
            source_units: vec!["probe_main.c", "probe_report.c"],
            source_blobs: probe_source_blobs(),
        },
        ProgramSnapshot {
            output_name: "console.elf",
            messages: vec![
                "[aethercore-console] fallback console ELF started\n",
                "[aethercore-console] wrappers: read,write,openat,close,getpid,getuid,getgid,execve,exit\n",
                "[aethercore-console] handoff target: /bin/sh\n",
                "[aethercore-console] runtime surface: /proc/sys/aethercore/abi/*\n",
                "[aethercore-console] startup layout: argc|argv|null|envp|null|auxv\n",
            ],
            candidates: vec!["/bin/sh\0"],
            role: "console_fallback",
            probe_features: vec!["exec_chain", "shell_handoff"],
            source_units: vec!["console_main.c"],
            source_blobs: console_source_blobs(),
        },
    ]
}
