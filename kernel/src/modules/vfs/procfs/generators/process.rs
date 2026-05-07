use super::super::*;

const PROC_SELF_STATUS_FDSIZE: usize = 256;
const PROC_SELF_STATUS_VM_PEAK_KB: usize = 4096;
const PROC_SELF_STATUS_VM_SIZE_KB: usize = 4096;
const PROC_SELF_STATUS_VMRSS_KB: usize = 2048;
const PROC_SELF_STATUS_VMDATA_KB: usize = 1024;
const PROC_SELF_STATUS_VMSTK_KB: usize = 256;
const PROC_SELF_STATUS_VMEXE_KB: usize = 512;
const PROC_SELF_STATUS_VMLIB_KB: usize = 1024;

pub fn generate_self_status(tid: TaskId) -> String {
    let pid = tid.0;
    let seccomp_mode = crate::kernel::syscalls::linux_seccomp_mode_for_tid(pid as usize);
    let no_new_privs = if crate::kernel::syscalls::linux_no_new_privs_for_tid(pid as usize) {
        1
    } else {
        0
    };
    let mut result = String::new();
    result.push_str(&format!("Name:\taethercore\n"));
    result.push_str(&format!("Umask:\t0022\n"));
    result.push_str(&format!("State:\tR (running)\n"));
    result.push_str(&format!("Tgid:\t{}\n", pid));
    result.push_str(&format!("Ngid:\t0\n"));
    result.push_str(&format!("Pid:\t{}\n", pid));
    result.push_str(&format!("PPid:\t1\n"));
    result.push_str(&format!("TracerPid:\t0\n"));
    result.push_str(&format!("Uid:\t0\t0\t0\t0\n"));
    result.push_str(&format!("Gid:\t0\t0\t0\t0\n"));
    result.push_str(&format!("FDSize:\t{}\n", PROC_SELF_STATUS_FDSIZE));
    result.push_str(&format!("Groups:\t0\n"));

    if let Some(process) = crate::kernel::process_registry::get_process(crate::interfaces::task::ProcessId(pid as usize)) {
        let mappings = process.mappings.lock();
        let mut total_pages = 0;
        let mut data_pages = 0;
        let mut stack_pages = 0;
        let mut exe_pages = 0;

        for m in mappings.iter() {
            let pages = ((m.end - m.start) / 4096) as usize;
            total_pages += pages;
            if m.prot & 4 != 0 { exe_pages += pages; } // PROT_EXEC
            if m.prot & 2 != 0 { data_pages += pages; } // PROT_WRITE
            // Simple heuristic for stack
            if m.start >= 0x7000_0000_0000 { stack_pages += pages; }
        }

        result.push_str(&format!("VmPeak:\t    {} kB\n", total_pages * 4));
        result.push_str(&format!("VmSize:\t    {} kB\n", total_pages * 4));
        result.push_str(&format!("VmRSS:\t    {} kB\n", total_pages * 4)); // Simplified
        result.push_str(&format!("VmData:\t    {} kB\n", data_pages * 4));
        result.push_str(&format!("VmStk:\t     {} kB\n", stack_pages * 4));
        result.push_str(&format!("VmExe:\t     {} kB\n", exe_pages * 4));
        result.push_str(&format!("VmLib:\t    {} kB\n", PROC_SELF_STATUS_VMLIB_KB));

        let threads = process.threads.lock().len();
        result.push_str(&format!("Threads:\t{}\n", threads));
    } else {
        result.push_str(&format!("VmPeak:\t    {} kB\n", PROC_SELF_STATUS_VM_PEAK_KB));
        result.push_str(&format!("VmSize:\t    {} kB\n", PROC_SELF_STATUS_VM_SIZE_KB));
        result.push_str(&format!("VmRSS:\t    {} kB\n", PROC_SELF_STATUS_VMRSS_KB));
        result.push_str(&format!("VmData:\t    {} kB\n", PROC_SELF_STATUS_VMDATA_KB));
        result.push_str(&format!("VmStk:\t     {} kB\n", PROC_SELF_STATUS_VMSTK_KB));
        result.push_str(&format!("VmExe:\t     {} kB\n", PROC_SELF_STATUS_VMEXE_KB));
        result.push_str(&format!("VmLib:\t    {} kB\n", PROC_SELF_STATUS_VMLIB_KB));
        result.push_str(&format!("Threads:\t1\n"));
    }
    result.push_str(&format!("SigQ:\t0/31439\n"));
    result.push_str(&format!("SigPnd:\t0000000000000000\n"));
    result.push_str(&format!("ShdPnd:\t0000000000000000\n"));
    result.push_str(&format!("SigBlk:\t0000000000000000\n"));
    result.push_str(&format!("SigIgn:\t0000000000000000\n"));
    result.push_str(&format!("SigCgt:\t0000000000000000\n"));
    result.push_str(&format!("CapInh:\t0000000000000000\n"));
    result.push_str(&format!("CapPrm:\t000001ffffffffff\n"));
    result.push_str(&format!("CapEff:\t000001ffffffffff\n"));
    result.push_str(&format!("CapBnd:\t000001ffffffffff\n"));
    result.push_str(&format!("CapAmb:\t0000000000000000\n"));
    result.push_str(&format!("NoNewPrivs:\t{}\n", no_new_privs));
    result.push_str(&format!("Seccomp:\t{}\n", seccomp_mode));
    result.push_str(&format!("voluntary_ctxt_switches:\t0\n"));
    result.push_str(&format!("nonvoluntary_ctxt_switches:\t0\n"));
    result
}

pub fn generate_self_maps(tid: TaskId) -> String {
    let pid = crate::interfaces::task::ProcessId(tid.0);
    let mut result = String::new();
    if let Some(process) = crate::kernel::process_registry::get_process(pid) {
        let mappings = process.mappings.lock();
        let exec_path = process.exec_path_snapshot();
        for m in mappings.iter() {
            let r = if m.prot & 1 != 0 { "r" } else { "-" };
            let w = if m.prot & 2 != 0 { "w" } else { "-" };
            let x = if m.prot & 4 != 0 { "x" } else { "-" };
            let p = if m.flags & 2 != 0 { "p" } else { "s" }; // MAP_PRIVATE=2
            
            let path = if m.prot & 4 != 0 && !exec_path.is_empty() {
                &exec_path
            } else if m.start >= 0x7000_0000_0000 {
                "[stack]"
            } else {
                ""
            };

            result.push_str(&format!(
                "{:012x}-{:012x} {}{}{}{} 00000000 00:00 0          {}\n",
                m.start, m.end, r, w, x, p, path
            ));
        }
    } else {
        result.push_str("000000400000-000000401000 r-xp 00000000 00:00 0          [text]\n");
        result.push_str("000000600000-000000601000 rw-p 00000000 00:00 0          [data]\n");
        result.push_str("7ffffffde000-7ffffffff000 rw-p 00000000 00:00 0          [stack]\n");
        result.push_str("ffffffffff600000-ffffffffff601000 r-xp 00000000 00:00 0  [vdso]\n");
    }
    result
}

pub fn generate_self_stat(tid: TaskId) -> String {
    let pid = tid.0;
    format!(
        "{} (aethercore) R {} {} 0 0 -1 4194304 0 0 0 0 0 0 0 0 20 0 1 0 0 4096000 200 18446744073709551615 4194304 4239000 140736200000000 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0\n",
        pid,
        pid.max(1) - 1,
        pid,
    )
}

pub fn generate_cmdline() -> String {
    String::from("aethercore\0")
}
