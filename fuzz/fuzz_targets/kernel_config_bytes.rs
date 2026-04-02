#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = aethercore::config::KernelConfig::time_slice();
    let _ = aethercore::config::KernelConfig::stack_size();
    let _ = aethercore::config::KernelConfig::is_telemetry_enabled();
    let _ = data.len();
});
