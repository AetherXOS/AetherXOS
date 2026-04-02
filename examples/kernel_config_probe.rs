fn main() {
    println!("arch={}", aethercore::config::KernelConfig::arch());
    println!(
        "time_slice={}",
        aethercore::config::KernelConfig::time_slice()
    );
    println!(
        "telemetry_enabled={}",
        aethercore::config::KernelConfig::is_telemetry_enabled()
    );
}
