use super::*;

fn cpu_count() -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        crate::hal::x86_64::smp::CPUS.lock().len().max(1)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        1
    }
}

pub(super) fn generate_version() -> String {
    format!(
        "Linux version {} ({}) (rustc) #1 SMP\n",
        crate::config::KernelConfig::linux_release(),
        crate::config::KernelConfig::linux_version(),
    )
}

pub(super) fn generate_meminfo() -> String {
    // Default to 256MB; will be refined when memory manager exposes live stats.
    let total_kb: u64 = 256 * 1024;
    let free_kb = total_kb / 2;
    let available_kb = free_kb;
    let buffers_kb = total_kb / 16;
    let cached_kb = total_kb / 8;

    format!(
        "MemTotal:       {} kB\n\
         MemFree:        {} kB\n\
         MemAvailable:   {} kB\n\
         Buffers:        {} kB\n\
         Cached:         {} kB\n\
         SwapCached:            0 kB\n\
         Active:         {} kB\n\
         Inactive:       {} kB\n\
         SwapTotal:             0 kB\n\
         SwapFree:              0 kB\n\
         Dirty:                 0 kB\n\
         Writeback:             0 kB\n\
         AnonPages:      {} kB\n\
         Mapped:         {} kB\n\
         Shmem:                 0 kB\n\
         Slab:           {} kB\n\
         SReclaimable:   {} kB\n\
         SUnreclaim:     {} kB\n\
         KernelStack:         256 kB\n\
         PageTables:          128 kB\n\
         CommitLimit:    {} kB\n\
         Committed_AS:   {} kB\n\
         VmallocTotal:   34359738367 kB\n\
         VmallocUsed:         1024 kB\n\
         VmallocChunk:   34359737344 kB\n\
         HugePages_Total:       0\n\
         HugePages_Free:        0\n\
         HugePages_Rsvd:        0\n\
         HugePages_Surp:        0\n\
         Hugepagesize:       2048 kB\n",
        total_kb,
        free_kb,
        available_kb,
        buffers_kb,
        cached_kb,
        total_kb / 4,
        total_kb / 4,
        total_kb / 8,
        total_kb / 16,
        total_kb / 32,
        total_kb / 64,
        total_kb / 64,
        total_kb,
        total_kb / 4,
    )
}

pub(super) fn generate_cpuinfo() -> String {
    let cpu_count = cpu_count();

    let mut result = String::new();
    for i in 0..cpu_count {
        result.push_str(&format!(
            "processor\t: {}\n\
             vendor_id\t: GenuineIntel\n\
             cpu family\t: 6\n\
             model\t\t: 158\n\
             model name\t: HyperCore Virtual CPU\n\
             stepping\t: 10\n\
             cpu MHz\t\t: 2400.000\n\
             cache size\t: 8192 KB\n\
             physical id\t: 0\n\
             siblings\t: {}\n\
             core id\t\t: {}\n\
             cpu cores\t: {}\n\
             apicid\t\t: {}\n\
             fpu\t\t: yes\n\
             fpu_exception\t: yes\n\
             cpuid level\t: 22\n\
             wp\t\t: yes\n\
             flags\t\t: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ss syscall nx pdpe1gb rdtscp lm constant_tsc rep_good nopl xtopology cpuid pni pclmulqdq ssse3 fma cx16 sse4_1 sse4_2 x2apic movbe popcnt aes xsave avx f16c rdrand hypervisor lahf_lm abm cpuid_fault pti fsgsbase bmi1 avx2 bmi2 erms rdseed adx clflushopt\n\
             bogomips\t: 4800.00\n\
             clflush size\t: 64\n\
             cache_alignment\t: 64\n\
             address sizes\t: 48 bits physical, 48 bits virtual\n\
             power management:\n\n",
            i,
            cpu_count,
            i,
            cpu_count,
            i,
        ));
    }
    result
}

pub(super) fn generate_uptime() -> String {
    let ticks = crate::hal::cpu::rdtsc();
    let seconds = ticks / 2_400_000_000;
    let idle_seconds = seconds / 2;
    format!("{}.00 {}.00\n", seconds, idle_seconds)
}

pub(super) fn generate_stat() -> String {
    let cpu_count = cpu_count();

    let mut result = String::from("cpu  100 0 50 800 0 10 0 0 0 0\n");
    for i in 0..cpu_count {
        result.push_str(&format!(
            "cpu{} {} 0 {} {} 0 {} 0 0 0 0\n",
            i,
            100 / cpu_count,
            50 / cpu_count,
            800 / cpu_count,
            10 / cpu_count,
        ));
    }
    result.push_str("intr 0\n");
    result.push_str("ctxt 0\n");
    result.push_str("btime 0\n");
    result.push_str(&format!(
        "processes {}\n",
        crate::kernel::process_registry::process_count()
    ));
    result.push_str("procs_running 1\n");
    result.push_str("procs_blocked 0\n");
    result.push_str("softirq 0 0 0 0 0 0 0 0 0 0 0\n");
    result
}

pub(super) fn generate_loadavg() -> String {
    format!(
        "0.00 0.00 0.00 1/{} 1\n",
        crate::kernel::process_registry::process_count()
    )
}

pub(super) fn generate_mounts() -> String {
    let mut result = String::new();
    result.push_str("devfs /dev devfs rw 0 0\n");
    result.push_str("proc /proc proc rw,nosuid,nodev,noexec 0 0\n");
    result.push_str("sysfs /sys sysfs rw,nosuid,nodev,noexec 0 0\n");
    result.push_str("tmpfs /tmp tmpfs rw 0 0\n");
    result.push_str("ramfs / ramfs rw 0 0\n");
    result
}

pub(super) fn generate_filesystems() -> String {
    let mut result = String::new();
    result.push_str("nodev\tramfs\n");
    result.push_str("nodev\tdevfs\n");
    result.push_str("nodev\tprocfs\n");
    result.push_str("nodev\tsysfs\n");
    result.push_str("nodev\ttmpfs\n");
    #[cfg(feature = "vfs_ext4")]
    result.push_str("\text4\n");
    #[cfg(feature = "vfs_fatfs")]
    result.push_str("\tvfat\n");
    result
}

pub(super) fn generate_self_status(tid: TaskId) -> String {
    let pid = tid.0;
    let mut result = String::new();
    result.push_str(&format!("Name:\thypercore\n"));
    result.push_str(&format!("Umask:\t0022\n"));
    result.push_str(&format!("State:\tR (running)\n"));
    result.push_str(&format!("Tgid:\t{}\n", pid));
    result.push_str(&format!("Ngid:\t0\n"));
    result.push_str(&format!("Pid:\t{}\n", pid));
    result.push_str(&format!("PPid:\t1\n"));
    result.push_str(&format!("TracerPid:\t0\n"));
    result.push_str(&format!("Uid:\t0\t0\t0\t0\n"));
    result.push_str(&format!("Gid:\t0\t0\t0\t0\n"));
    result.push_str(&format!("FDSize:\t256\n"));
    result.push_str(&format!("Groups:\t0\n"));
    result.push_str(&format!("VmPeak:\t    4096 kB\n"));
    result.push_str(&format!("VmSize:\t    4096 kB\n"));
    result.push_str(&format!("VmRSS:\t    2048 kB\n"));
    result.push_str(&format!("VmData:\t    1024 kB\n"));
    result.push_str(&format!("VmStk:\t     256 kB\n"));
    result.push_str(&format!("VmExe:\t     512 kB\n"));
    result.push_str(&format!("VmLib:\t    1024 kB\n"));
    result.push_str(&format!("Threads:\t1\n"));
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
    result.push_str(&format!("Seccomp:\t0\n"));
    result.push_str(&format!("voluntary_ctxt_switches:\t0\n"));
    result.push_str(&format!("nonvoluntary_ctxt_switches:\t0\n"));
    result
}

pub(super) fn generate_self_maps(_tid: TaskId) -> String {
    let mut result = String::new();
    result.push_str("000000400000-000000401000 r-xp 00000000 00:00 0          [text]\n");
    result.push_str("000000600000-000000601000 rw-p 00000000 00:00 0          [data]\n");
    result.push_str("7ffffffde000-7ffffffff000 rw-p 00000000 00:00 0          [stack]\n");
    result.push_str("ffffffffff600000-ffffffffff601000 r-xp 00000000 00:00 0  [vdso]\n");
    result
}

pub(super) fn generate_self_stat(tid: TaskId) -> String {
    let pid = tid.0;
    format!(
        "{} (hypercore) R {} {} 0 0 -1 4194304 0 0 0 0 0 0 0 0 20 0 1 0 0 4096000 200 18446744073709551615 4194304 4239000 140736200000000 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0\n",
        pid,
        pid.max(1) - 1,
        pid,
    )
}

pub(super) fn generate_cmdline() -> String {
    String::from("hypercore\0")
}
