#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = hypercore::config::KernelConfig::time_slice();
    let _ = hypercore::config::KernelConfig::stack_size();
    let _ = hypercore::config::KernelConfig::is_telemetry_enabled();
    let _ = data.len();
});
