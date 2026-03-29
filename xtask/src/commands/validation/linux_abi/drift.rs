use anyhow::{Result, bail};
use std::collections::HashMap;
use std::fs;

use crate::utils::paths;
use crate::config;

pub fn run() -> Result<()> {
    println!("[linux-abi::policy-drift] Checking policy drift ABI constants");

    let root = paths::repo_root();
    let syscalls_path = root.join(config::SYSCALL_CONSTS_PATH);
    let generated_path = root.join(config::GENERATED_CONSTS_PATH);

    if !syscalls_path.exists() || !generated_path.exists() {
        bail!("Missing syscalls_consts.rs or generated_consts.rs at standard paths.");
    }

    let nr_re = regex::Regex::new(r"pub const ([A-Z0-9_]+): usize = (\d+);")?;
    let u64_re = regex::Regex::new(r"pub const ([A-Z0-9_]+): u64 = (\d+);")?;

    let mut nr_map = HashMap::new();
    let text_nr = fs::read_to_string(&syscalls_path)?;
    for cap in nr_re.captures_iter(&text_nr) {
        if let Ok(v) = cap[2].parse::<usize>() {
            nr_map.insert(cap[1].to_string(), v);
        }
    }

    let mut gen_map = HashMap::new();
    let text_gen = fs::read_to_string(&generated_path)?;
    for cap in u64_re.captures_iter(&text_gen) {
        if let Ok(v) = cap[2].parse::<u64>() {
            gen_map.insert(cap[1].to_string(), v);
        }
    }

    let expected_nr = vec![
        ("SET_POLICY_DRIFT_CONTROL", 58),
        ("GET_POLICY_DRIFT_CONTROL", 59),
        ("GET_POLICY_DRIFT_REASON_TEXT", 60),
    ];

    let required_consts = vec![
        "GOVERNOR_RUNTIME_POLICY_DRIFT_SAMPLE_INTERVAL_TICKS",
        "GOVERNOR_RUNTIME_POLICY_DRIFT_REAPPLY_COOLDOWN_TICKS",
    ];

    let mut failures = Vec::new();

    for &(name, expected) in &expected_nr {
        match nr_map.get(name) {
            Some(&got) if got == expected => {}
            Some(&got) => failures.push(format!("{}: expected {}, got {}", name, expected, got)),
            None => failures.push(format!("{}: missing", name)),
        }
    }

    for &name in &required_consts {
        match gen_map.get(name) {
            Some(&v) if v > 0 => {}
            Some(&v) => failures.push(format!("{}: invalid ({})", name, v)),
            None => failures.push(format!("{}: missing", name)),
        }
    }

    if failures.is_empty() {
        println!("[linux-abi::policy-drift] PASS");
        for (k, _) in &expected_nr {
            if let Some(v) = nr_map.get(*k) { println!("  {}={}", k, v); }
        }
        for k in &required_consts {
            if let Some(v) = gen_map.get(*k) { println!("  {}={}", k, v); }
        }
        Ok(())
    } else {
        println!("[linux-abi::policy-drift] FAIL");
        for f in &failures {
            println!("  - {}", f);
        }
        bail!("Policy drift ABI smoke failed.");
    }
}
