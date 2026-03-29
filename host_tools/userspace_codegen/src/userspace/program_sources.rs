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
                "[hypercore-init] staged userspace init ELF started\n",
                "[hypercore-init] wrappers: read,write,openat,close,getpid,getppid,getuid,geteuid,getgid,getegid,execve,exit\n",
                "[hypercore-init] contract: auxv+vdso+proc/sys abi surface expected\n",
                "[hypercore-init] loader: gnu-hash+runpath+init-hooks runtime: tls+signal-abi+vdso\n",
                "[hypercore-init] startup env: runtime+syscall+auxv key groups exported\n",
            ],
            candidates: vec![
                "/usr/lib/hypercore/probe.elf\0",
                "/usr/lib/hypercore/console.elf\0",
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
                "[hypercore-probe] runtime probe ELF started\n",
                "[hypercore-probe] goal: validate startup+auxv+vdso abi path\n",
                "[hypercore-probe] expected runtime api: probe_mask+probe_status+probe_summary\n",
            ],
            candidates: vec!["/usr/lib/hypercore/console.elf\0", "/bin/sh\0"],
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
                "[hypercore-console] fallback console ELF started\n",
                "[hypercore-console] wrappers: read,write,openat,close,getpid,getuid,getgid,execve,exit\n",
                "[hypercore-console] handoff target: /bin/sh\n",
                "[hypercore-console] runtime surface: /proc/sys/hypercore/abi/*\n",
                "[hypercore-console] startup layout: argc|argv|null|envp|null|auxv\n",
            ],
            candidates: vec!["/bin/sh\0"],
            role: "console_fallback",
            probe_features: vec!["exec_chain", "shell_handoff"],
            source_units: vec!["console_main.c"],
            source_blobs: console_source_blobs(),
        },
    ]
}
