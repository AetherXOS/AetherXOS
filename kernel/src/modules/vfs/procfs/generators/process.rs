use super::super::*;

const PROC_SELF_STATUS_FDSIZE: usize = 256;
const PROC_SELF_STATUS_VMLIB_KB: usize = 1024;

pub fn generate_self_status(tid: TaskId) -> String {
    let pid = tid.0;

    let seccomp_mode = crate::kernel::syscalls::linux_seccomp_mode_for_tid(pid as usize);
    let no_new_privs = u8::from(crate::kernel::syscalls::linux_no_new_privs_for_tid(pid as usize));

    // Fetch real process name from task registry
    let name = crate::kernel::task::get_task(tid)
        .map(|t| t.lock().name.clone())
        .unwrap_or_else(|| alloc::string::String::from("aethercore"));

    // Fetch real PPID/PGID/SID
    let (ppid, pgid, sid) =
        if let Some(proc) = crate::kernel::process_registry::get_process(crate::interfaces::task::ProcessId(pid as usize)) {
            let ppid = proc.parent_id.load(core::sync::atomic::Ordering::Relaxed);
            let pgid = proc.pgid.load(core::sync::atomic::Ordering::Relaxed) as usize;
            let sid  = proc.sid.load(core::sync::atomic::Ordering::Relaxed) as usize;
            (ppid, pgid, sid)
        } else {
            (0, pid, pid)
        };

    // Fetch signal masks from the task
    let (sig_pnd, sig_blk, sig_ign, sig_cgt) =
        if let Some(t) = crate::kernel::task::get_task(tid) {
            let task = t.lock();
            let mask  = task.signal_mask;
            let queue_bits: u64 = {
                let q = task.signal_queue.lock();
                q.iter().fold(0u64, |acc, si| {
                    if si.nr > 0 && si.nr <= 64 { acc | (1u64 << (si.nr - 1)) } else { acc }
                })
            };
            (queue_bits, mask, 0u64, 0u64) // ign/cgt would need SIGNAL_ACTIONS scan
        } else {
            (0u64, 0u64, 0u64, 0u64)
        };

    let mut result = alloc::string::String::new();
    use alloc::format;
    result.push_str(&format!("Name:\t{}\n", &name[..name.len().min(15)]));
    result.push_str("Umask:\t0022\n");
    result.push_str("State:\tR (running)\n");
    result.push_str(&format!("Tgid:\t{}\n", pid));
    result.push_str("Ngid:\t0\n");
    result.push_str(&format!("Pid:\t{}\n", pid));
    result.push_str(&format!("PPid:\t{}\n", ppid));
    result.push_str("TracerPid:\t0\n");
    result.push_str("Uid:\t0\t0\t0\t0\n");
    result.push_str("Gid:\t0\t0\t0\t0\n");
    result.push_str(&format!("FDSize:\t{}\n", PROC_SELF_STATUS_FDSIZE));
    result.push_str("Groups:\t\n");
    result.push_str(&format!("NStgid:\t{}\n", pid));
    result.push_str(&format!("NSpid:\t{}\n", pid));
    result.push_str(&format!("NSpgid:\t{}\n", pgid));
    result.push_str("NSsid:\t1\n");

    if let Some(process) = crate::kernel::process_registry::get_process(crate::interfaces::task::ProcessId(pid as usize)) {
        let mappings = process.mappings.lock();
        let mut total_pages = 0usize;
        let mut data_pages  = 0usize;
        let mut stack_pages = 0usize;
        let mut exe_pages   = 0usize;

        for m in mappings.iter() {
            let pages = ((m.end - m.start) / 4096) as usize;
            total_pages += pages;
            if m.prot & 4 != 0 { exe_pages   += pages; }
            if m.prot & 2 != 0 { data_pages  += pages; }
            if m.start >= 0x7000_0000_0000 { stack_pages += pages; }
        }
        let rss_kb   = total_pages * 4;
        let vsize_kb = total_pages * 4;
        let threads  = process.threads.lock().len();

        result.push_str(&format!("VmPeak:\t{:>8} kB\n", vsize_kb));
        result.push_str(&format!("VmSize:\t{:>8} kB\n", vsize_kb));
        result.push_str(&format!("VmLck:\t       0 kB\n"));
        result.push_str(&format!("VmPin:\t       0 kB\n"));
        result.push_str(&format!("VmHWM:\t{:>8} kB\n", rss_kb));
        result.push_str(&format!("VmRSS:\t{:>8} kB\n", rss_kb));
        result.push_str(&format!("RssAnon:\t{:>8} kB\n", rss_kb.saturating_sub(exe_pages * 4)));
        result.push_str(&format!("RssFile:\t{:>8} kB\n", exe_pages * 4));
        result.push_str(&format!("RssShmem:\t       0 kB\n"));
        result.push_str(&format!("VmData:\t{:>8} kB\n", data_pages * 4));
        result.push_str(&format!("VmStk:\t{:>8} kB\n", stack_pages * 4));
        result.push_str(&format!("VmExe:\t{:>8} kB\n", exe_pages * 4));
        result.push_str(&format!("VmLib:\t{:>8} kB\n", PROC_SELF_STATUS_VMLIB_KB));
        result.push_str(&format!("VmPTE:\t       4 kB\n"));
        result.push_str(&format!("VmSwap:\t       0 kB\n"));
        result.push_str(&format!("Threads:\t{}\n", threads));
    } else {
        result.push_str("VmPeak:\t    4096 kB\n");
        result.push_str("VmSize:\t    4096 kB\n");
        result.push_str("VmRSS:\t    2048 kB\n");
        result.push_str("VmData:\t    1024 kB\n");
        result.push_str("VmStk:\t     256 kB\n");
        result.push_str("VmExe:\t     512 kB\n");
        result.push_str(&format!("VmLib:\t    {} kB\n", PROC_SELF_STATUS_VMLIB_KB));
        result.push_str("Threads:\t1\n");
    }

    result.push_str("SigQ:\t0/31439\n");
    result.push_str(&format!("SigPnd:\t{:016x}\n", sig_pnd));
    result.push_str("ShdPnd:\t0000000000000000\n");
    result.push_str(&format!("SigBlk:\t{:016x}\n", sig_blk));
    result.push_str(&format!("SigIgn:\t{:016x}\n", sig_ign));
    result.push_str(&format!("SigCgt:\t{:016x}\n", sig_cgt));
    result.push_str("CapInh:\t0000000000000000\n");
    result.push_str("CapPrm:\t000001ffffffffff\n");
    result.push_str("CapEff:\t000001ffffffffff\n");
    result.push_str("CapBnd:\t000001ffffffffff\n");
    result.push_str("CapAmb:\t0000000000000000\n");
    result.push_str(&format!("NoNewPrivs:\t{}\n", no_new_privs));
    result.push_str(&format!("Seccomp:\t{}\n", seccomp_mode));
    result.push_str("Seccomp_filters:\t0\n");
    result.push_str("Speculation_Store_Bypass:\tvulnerable\n");
    result.push_str("SpeculationIndirectBranch:\talways enabled\n");
    result.push_str("voluntary_ctxt_switches:\t1\n");
    result.push_str("nonvoluntary_ctxt_switches:\t1\n");
    result
}

pub fn generate_self_maps(tid: TaskId) -> String {
    use alloc::format;
    let pid = crate::interfaces::task::ProcessId(tid.0);
    let mut result = alloc::string::String::new();

    if let Some(process) = crate::kernel::process_registry::get_process(pid) {
        let mappings = process.mappings.lock();
        let exec_path = process.exec_path_snapshot();

        for m in mappings.iter() {
            let r = if m.prot & 1 != 0 { 'r' } else { '-' };
            let w = if m.prot & 2 != 0 { 'w' } else { '-' };
            let x = if m.prot & 4 != 0 { 'x' } else { '-' };
            let p = if m.flags & 2 != 0 { 'p' } else { 's' };

            let path: &str = if m.prot & 4 != 0 && !exec_path.is_empty() {
                &exec_path
            } else if m.start >= 0x7fff_0000_0000 {
                "[stack]"
            } else if m.start == 0xffff_ffff_ff60_0000 {
                "[vsyscall]"
            } else {
                ""
            };

            let offset = 0u64; // Simplified; full impl would track file offset
            result.push_str(&format!(
                "{:016x}-{:016x} {}{}{}{} {:08x} 00:00 {:<10}  {}\n",
                m.start, m.end, r, w, x, p,
                offset,
                if path.is_empty() { 0 } else { m.start >> 12 }, // fake inode
                path
            ));
        }
    } else {
        // Fallback: plausible layout for a static binary
        result.push_str("0000000000400000-0000000000401000 r-xp 00000000 00:00 1234       /aethercore\n");
        result.push_str("0000000000600000-0000000000601000 rw-p 00001000 00:00 1234       /aethercore\n");
        result.push_str("00007fff00000000-00007fff00020000 rw-p 00000000 00:00 0          [stack]\n");
    }
    result
}

pub fn generate_self_stat(tid: TaskId) -> String {
    use alloc::format;
    let pid = tid.0;

    let (ppid, pgid, sid, utime, stime, vsize, rss) =
        if let Some(proc) = crate::kernel::process_registry::get_process(crate::interfaces::task::ProcessId(pid)) {
            let ppid  = proc.parent_id.load(core::sync::atomic::Ordering::Relaxed);
            let pgid  = proc.pgid.load(core::sync::atomic::Ordering::Relaxed) as usize;
            let sid   = proc.sid.load(core::sync::atomic::Ordering::Relaxed) as usize;
            let maps  = proc.mappings.lock();
            let pages: usize = maps.iter().map(|m| ((m.end - m.start) / 4096) as usize).sum();
            (ppid, pgid, sid, 0u64, 0u64, pages * 4096, pages as i64)
        } else {
            (0, pid, pid, 0u64, 0u64, 4096 * 1024, 512i64)
        };

    let name = crate::kernel::task::get_task(tid)
        .map(|t| t.lock().name.clone())
        .unwrap_or_else(|| alloc::string::String::from("aethercore"));
    let short_name = &name[..name.len().min(15)];

    // Format: pid (comm) state ppid pgrp session tty_nr tpgid flags
    //         minflt cminflt majflt cmajflt utime stime cutime cstime
    //         priority nice num_threads itrealvalue starttime vsize rss ...
    format!(
        "{pid} ({name}) R {ppid} {pgid} {sid} 0 -1 4194304 \
         0 0 0 0 {utime} {stime} 0 0 \
         20 0 1 0 0 {vsize} {rss} \
         18446744073709551615 4194304 4239000 \
         140736200000000 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0\n",
        pid  = pid,
        name = short_name,
        ppid = ppid,
        pgid = pgid,
        sid  = sid,
        utime = utime,
        stime = stime,
        vsize = vsize,
        rss   = rss,
    )
}

/// `/proc/[pid]/stat` - shorter alias
pub fn generate_stat(tid: TaskId) -> alloc::string::String {
    generate_self_stat(tid)
}

/// `/proc/[pid]/cmdline` - null-separated arguments
pub fn generate_cmdline(tid: TaskId) -> alloc::string::String {
    let name = crate::kernel::task::get_task(tid)
        .map(|t| t.lock().name.clone())
        .unwrap_or_else(|| alloc::string::String::from("aethercore"));
    let mut s = name;
    s.push('\0');
    s
}

/// `/proc/[pid]/exe` symlink target
pub fn generate_exe_path(tid: TaskId) -> alloc::string::String {
    if let Some(proc) = crate::kernel::process_registry::get_process(crate::interfaces::task::ProcessId(tid.0)) {
        let path = proc.exec_path_snapshot();
        if !path.is_empty() {
            return path;
        }
    }
    alloc::string::String::from("/proc/self/exe")
}

/// `/proc/[pid]/fd/` - open file descriptor listing
pub fn generate_fd_list(tid: TaskId) -> alloc::vec::Vec<(u32, alloc::string::String)> {
    #[cfg(feature = "posix_fs")]
    {
        let table = crate::modules::posix::fs::FILE_TABLE.lock();
        table.iter()
            .map(|(fd, desc)| {
                let target = if desc.path.is_empty() {
                    alloc::format!("socket:[{}]", fd)
                } else {
                    desc.path.clone()
                };
                (*fd, target)
            })
            .collect()
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = tid;
        alloc::vec![
            (0u32, alloc::string::String::from("/dev/stdin")),
            (1u32, alloc::string::String::from("/dev/stdout")),
            (2u32, alloc::string::String::from("/dev/stderr")),
        ]
    }
}

/// `/proc/[pid]/wchan` - kernel symbol where process is sleeping
pub fn generate_wchan(_tid: TaskId) -> alloc::string::String {
    alloc::string::String::from("0\n")
}

/// `/proc/[pid]/io` - I/O accounting
pub fn generate_io(_tid: TaskId) -> alloc::string::String {
    alloc::string::String::from(
        "rchar: 0\nwchar: 0\nsyscr: 0\nsyscw: 0\n\
         read_bytes: 0\nwrite_bytes: 0\ncancelled_write_bytes: 0\n"
    )
}
