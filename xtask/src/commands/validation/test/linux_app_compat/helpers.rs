use std::fs;
use std::path::Path;
use std::process::Command;

use super::{Layer, Totals};

pub(super) fn shell_cmd(command: &str) -> std::io::Result<std::process::Output> {
    #[cfg(windows)]
    {
        Command::new("cmd").args(["/C", command]).output()
    }
    #[cfg(not(windows))]
    {
        Command::new("sh").args(["-c", command]).output()
    }
}

pub(super) fn command_exists(cmd: &str) -> bool {
    #[cfg(windows)]
    {
        Command::new("where")
            .arg(cmd)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        shell_cmd(&format!("command -v {} >/dev/null 2>&1", cmd))
            .map(|out| out.status.success())
            .unwrap_or(false)
    }
}

pub(super) fn file_contains(path: &Path, needle: &str) -> bool {
    if !path.exists() {
        return false;
    }
    fs::read_to_string(path)
        .map(|text| text.contains(needle))
        .unwrap_or(false)
}

pub(super) fn file_contains_all(path: &Path, needles: &[&str]) -> bool {
    if !path.exists() {
        return false;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    needles.iter().all(|needle| text.contains(needle))
}

pub(super) fn count_occurrences(path: &Path, needle: &str) -> usize {
    if !path.exists() {
        return 0;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return 0;
    };
    text.matches(needle).count()
}

pub(super) fn run_case(layer: &mut Layer, totals: &mut Totals, name: &str, cmd: &str) -> bool {
    print!("[TEST] {}", name);
    match shell_cmd(cmd) {
        Ok(out) if out.status.success() => {
            println!(" OK");
            layer.total += 1;
            layer.passed += 1;
            totals.passed += 1;
            true
        }
        Ok(_) | Err(_) => {
            println!(" FAIL");
            layer.total += 1;
            layer.failed += 1;
            totals.failed += 1;
            false
        }
    }
}

pub(super) fn run_optional(
    layer: &mut Layer,
    totals: &mut Totals,
    name: &str,
    check_cmd: &str,
    cmd: &str,
    required: bool,
) {
    let present = shell_cmd(check_cmd)
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !present {
        print!("[TEST] {}", name);
        if required {
            println!(" FAIL");
            layer.total += 1;
            layer.failed += 1;
            totals.failed += 1;
        } else {
            println!(" SKIP");
            layer.total += 1;
            layer.skipped += 1;
            totals.skipped += 1;
        }
        return;
    }
    let _ = run_case(layer, totals, name, cmd);
}

pub(super) fn run_source_probe(
    layer: &mut Layer,
    totals: &mut Totals,
    name: &str,
    ok: bool,
    required: bool,
) {
    print!("[TEST] {}", name);
    if ok {
        println!(" OK");
        layer.total += 1;
        layer.passed += 1;
        totals.passed += 1;
        return;
    }

    if required {
        println!(" FAIL");
        layer.total += 1;
        layer.failed += 1;
        totals.failed += 1;
    } else {
        println!(" SKIP");
        layer.total += 1;
        layer.skipped += 1;
        totals.skipped += 1;
    }
}

pub(super) fn skip_case(layer: &mut Layer, totals: &mut Totals, name: &str) {
    print!("[TEST] {}", name);
    println!(" SKIP");
    layer.total += 1;
    layer.skipped += 1;
    totals.skipped += 1;
}

pub(super) fn rate(layer: &Layer) -> f64 {
    let executed = layer.total.saturating_sub(layer.skipped);
    if executed == 0 {
        100.0
    } else {
        ((layer.passed as f64 / executed as f64) * 1000.0).round() / 10.0
    }
}
