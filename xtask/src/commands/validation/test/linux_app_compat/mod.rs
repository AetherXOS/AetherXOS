use anyhow::{Result, bail};
use std::process::Command;

use crate::utils::paths;

mod constants;
mod helpers;
mod models;
mod probe_output;
mod profile;
mod probes;
mod reporting;
mod scoring;
use helpers::{run_case, run_optional, skip_case};
use models::{Layer, Totals};
use profile::NormalizedOptions;

#[derive(Debug, Clone, Copy)]
pub struct LinuxAppCompatOptions {
    pub desktop_smoke: bool,
    pub quick: bool,
    pub qemu: bool,
    pub strict: bool,
    pub ci: bool,
    pub require_busybox: bool,
    pub require_glibc: bool,
    pub require_wayland: bool,
    pub require_x11: bool,
    pub require_fs_stack: bool,
    pub require_package_stack: bool,
    pub require_desktop_app_stack: bool,
}

pub fn run(opts: LinuxAppCompatOptions) -> Result<()> {
    println!("[test::linux-app-compat] Native layered validator");

    let normalized = NormalizedOptions::from_raw(opts);

    if normalized.desktop_smoke {
        println!(
            "[test::linux-app-compat] Desktop smoke profile enabled (Wayland/X11 probes required)"
        );
    }

    let mut totals = Totals::default();
    let mut host = Layer::default();
    let mut integration = Layer::default();
    let mut compat = Layer::default();
    let mut kernel = Layer::default();
    let mut qemu_layer = Layer::default();

    println!("\nPhase 1: Host Shell and Linux Primitive Smoke");
    if cfg!(windows) {
        let _ = run_case(&mut host, &mut totals, "Process creation", "ver >nul");
        let _ = run_case(
            &mut host,
            &mut totals,
            "File read/write",
            "echo test>%TEMP%\\hc_test.txt && findstr test %TEMP%\\hc_test.txt >nul",
        );
        let _ = run_case(
            &mut host,
            &mut totals,
            "Pipe chaining",
            "echo hello | findstr hello >nul",
        );
        skip_case(&mut host, &mut totals, "procfs available");
        if !normalized.quick {
            skip_case(&mut host, &mut totals, "Loop execution");
        }
    } else {
        let _ = run_case(&mut host, &mut totals, "Process creation", "exit 0");
        let _ = run_case(
            &mut host,
            &mut totals,
            "File read/write",
            "echo test >/tmp/hc_test.txt; cat /tmp/hc_test.txt | grep test",
        );
        let _ = run_case(
            &mut host,
            &mut totals,
            "Pipe chaining",
            "echo hello | cat | grep hello",
        );
        let _ = run_case(
            &mut host,
            &mut totals,
            "procfs available",
            "ls /proc >/dev/null",
        );
        if !normalized.quick {
            let _ = run_case(
                &mut host,
                &mut totals,
                "Loop execution",
                "for i in 1 2 3; do echo $i; done | wc -l | grep '^3$'",
            );
        }
    }

    println!("\nPhase 1b: App Integration");
    if cfg!(windows) {
        skip_case(&mut integration, &mut totals, "awk aggregation");
        skip_case(&mut integration, &mut totals, "sed transform");
        skip_case(&mut integration, &mut totals, "tar round-trip");
    } else {
        let _ = run_case(
            &mut integration,
            &mut totals,
            "awk aggregation",
            "printf 'a 1\na 2\n' | awk '$1==\"a\" {s+=$2} END{print s}' | grep '^3$'",
        );
        let _ = run_case(
            &mut integration,
            &mut totals,
            "sed transform",
            "printf 'linux-compat\n' | sed 's/linux/hyper/' | grep '^hyper-compat$'",
        );
        let _ = run_case(
            &mut integration,
            &mut totals,
            "tar round-trip",
            "mkdir -p /tmp/hc_tar/src; echo payload >/tmp/hc_tar/src/a.txt; tar -cf /tmp/hc_tar/a.tar -C /tmp/hc_tar/src .; mkdir -p /tmp/hc_tar/out; tar -xf /tmp/hc_tar/a.tar -C /tmp/hc_tar/out; cat /tmp/hc_tar/out/a.txt | grep payload",
        );
    }

    println!("\nPhase 1c: Runtime Probe");
    run_optional(
        &mut compat,
        &mut totals,
        "busybox availability",
        "command -v busybox >/dev/null 2>&1",
        "busybox --help >/dev/null",
        normalized.require_busybox,
    );
    run_optional(
        &mut compat,
        &mut totals,
        "busybox applet smoke",
        "command -v busybox >/dev/null 2>&1",
        "busybox ls / >/dev/null",
        normalized.require_busybox,
    );
    run_optional(
        &mut compat,
        &mut totals,
        "glibc detection",
        "getconf GNU_LIBC_VERSION >/dev/null 2>&1",
        "getconf GNU_LIBC_VERSION | grep -i glibc",
        normalized.require_glibc,
    );

    let desktop_probes = probes::run_runtime_probes(&mut compat, &mut totals, &opts);

    println!("\nPhase 2: Kernel Gates");
    print!("[GATE] cargo check --lib --features linux_compat");
    let host_target = crate::utils::cargo::detect_host_triple().ok();
    let mut cargo_args = vec!["check", "--lib", "--features", "linux_compat"];
    if let Some(target) = host_target.as_deref() {
        cargo_args.push("--target");
        cargo_args.push(target);
    }
    let build_ok = Command::new("cargo")
        .args(&cargo_args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if build_ok {
        println!(" OK");
        kernel.total += 1;
        kernel.passed += 1;
        totals.passed += 1;
    } else {
        println!(" FAIL");
        kernel.total += 1;
        kernel.failed += 1;
        totals.failed += 1;
    }

    print!("[GATE] syscall coverage summary");
    let _ = crate::commands::validation::syscall_coverage::execute(
        true,
        "md",
        &Some("reports/linux_app_compat_syscall_coverage.md".to_string()),
    );
    let cov_ok = paths::resolve("reports/syscall_coverage_summary.json").exists();
    if cov_ok {
        println!(" OK");
        kernel.total += 1;
        kernel.passed += 1;
        totals.passed += 1;
    } else {
        println!(" FAIL");
        kernel.total += 1;
        kernel.failed += 1;
        totals.failed += 1;
    }

    if normalized.needs_qemu_gate() {
        println!("\nPhase 3: QEMU Gate");
        print!("[GATE] qemu smoke");
        if crate::commands::ops::qemu::smoke_test().is_ok() {
            println!(" OK");
            qemu_layer.total += 1;
            qemu_layer.passed += 1;
            totals.passed += 1;
        } else {
            println!(" FAIL");
            qemu_layer.total += 1;
            qemu_layer.failed += 1;
            totals.failed += 1;
        }
    }

    let score = scoring::build_score_bundle(
        normalized.score_profile(),
        normalized.ci,
        &totals,
        &host,
        &integration,
        &compat,
        &kernel,
        &qemu_layer,
        normalized.needs_qemu_gate(),
    );
    reporting::write_reports(normalized, &compat, desktop_probes, &score.scorecard)?;

    let total = totals.passed + totals.failed + totals.skipped;

    println!("\nPassed: {}/{}", totals.passed, total);
    println!("Failed: {}/{}", totals.failed, total);
    println!("Skipped: {}/{}", totals.skipped, total);
    println!("Pass Rate: {}%", score.pass_rate);

    if normalized.ci && !score.ci_ok {
        bail!("ci policy failed")
    }
    if totals.failed > 0 {
        bail!("linux app compatibility validation failed")
    }
    Ok(())
}
