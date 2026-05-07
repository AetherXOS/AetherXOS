use super::super::*;

pub fn generate_version() -> String {
    format!(
        "Linux version {} ({}) (rustc) #1 SMP\n",
        crate::config::KernelConfig::linux_release(),
        crate::config::KernelConfig::linux_version(),
    )
}

pub fn generate_uptime() -> String {
    let ticks = crate::hal::cpu::rdtsc();
    let seconds = ticks / 2_400_000_000;
    let idle_seconds = seconds / 2;
    format!("{}.00 {}.00\n", seconds, idle_seconds)
}

pub fn generate_loadavg() -> String {
    format!(
        "0.00 0.00 0.00 1/{} 1\n",
        crate::kernel::process_registry::process_count()
    )
}

pub fn generate_stat() -> String {
    let cpu_count = crate::hal::smp::cpu_count().max(1);

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
