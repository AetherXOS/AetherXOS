fn main() {
    println!("arch={}", hypercore::config::KernelConfig::arch());
    println!(
        "time_slice={}",
        hypercore::config::KernelConfig::time_slice()
    );
    println!(
        "telemetry_enabled={}",
        hypercore::config::KernelConfig::is_telemetry_enabled()
    );
}
