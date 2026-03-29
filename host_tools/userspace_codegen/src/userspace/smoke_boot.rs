use super::smoke_detect::{detect_llvm_readobj, detect_qemu_binary};
use super::smoke_proof::{lossy_bytes, write_proof_files};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn qemu_probe_iso_args(boot_image_dir: &Path) -> Vec<String> {
    vec![
        "-nographic".to_string(),
        "-no-reboot".to_string(),
        "-monitor".to_string(),
        "none".to_string(),
        "-m".to_string(),
        "512".to_string(),
        "-smp".to_string(),
        "2".to_string(),
        "-cdrom".to_string(),
        boot_image_dir
            .join("hypercore-probe.iso")
            .display()
            .to_string(),
        "-boot".to_string(),
        "d".to_string(),
    ]
}

fn qemu_iso_args(boot_image_dir: &Path) -> Vec<String> {
    vec![
        "-nographic".to_string(),
        "-no-reboot".to_string(),
        "-monitor".to_string(),
        "none".to_string(),
        "-m".to_string(),
        "512".to_string(),
        "-smp".to_string(),
        "2".to_string(),
        "-cdrom".to_string(),
        boot_image_dir.join("hypercore.iso").display().to_string(),
        "-boot".to_string(),
        "d".to_string(),
    ]
}

fn qemu_direct_args(stage_boot_dir: &Path) -> Vec<String> {
    vec![
        "-nographic".to_string(),
        "-no-reboot".to_string(),
        "-monitor".to_string(),
        "none".to_string(),
        "-m".to_string(),
        "512".to_string(),
        "-smp".to_string(),
        "2".to_string(),
        "-kernel".to_string(),
        stage_boot_dir.join("hypercore.elf").display().to_string(),
        "-initrd".to_string(),
        stage_boot_dir.join("initramfs.cpio.gz").display().to_string(),
        "-append".to_string(),
        "console=ttyS0 loglevel=7 HYPERCORE_RUN_LINKED_PROBE=1".to_string(),
    ]
}

fn boot_attempt_log_prefix(mode: &str, timed_out: bool, exit_code: Option<i32>) -> String {
    format!(
        "[hypercore-linked-probe-attempt]\nmode={mode}\ntimed_out={}\nexit_code={}\n\n",
        if timed_out { "yes" } else { "no" },
        exit_code
            .map(|code| code.to_string())
            .unwrap_or_else(|| "none".to_string())
    )
}

fn qemu_probe_ps1_command(preferred_boot_mode: &str) -> String {
    match preferred_boot_mode {
        "probe_iso" => "& $qemu -nographic -no-reboot -monitor none -m 512 -smp 2 -cdrom $probeIso -boot d *>&1 | Tee-Object -FilePath $liveLog | Tee-Object -FilePath $artifactLog".to_string(),
        "iso" => "& $qemu -nographic -no-reboot -monitor none -m 512 -smp 2 -cdrom $iso -boot d *>&1 | Tee-Object -FilePath $liveLog | Tee-Object -FilePath $artifactLog".to_string(),
        _ => "& $qemu -nographic -no-reboot -monitor none -m 512 -smp 2 -kernel $kernel -initrd $initramfs -append \"console=ttyS0 loglevel=7 HYPERCORE_RUN_LINKED_PROBE=1\" *>&1 | Tee-Object -FilePath $liveLog | Tee-Object -FilePath $artifactLog".to_string(),
    }
}

fn qemu_probe_sh_command(preferred_boot_mode: &str) -> String {
    match preferred_boot_mode {
        "probe_iso" => "\"${QEMU_BIN}\" -nographic -no-reboot -monitor none -m 512 -smp 2 -cdrom \"${PROBE_ISO}\" -boot d 2>&1 | tee \"${LIVE_LOG}\" | tee \"${ARTIFACT_LOG}\"".to_string(),
        "iso" => "\"${QEMU_BIN}\" -nographic -no-reboot -monitor none -m 512 -smp 2 -cdrom \"${ISO}\" -boot d 2>&1 | tee \"${LIVE_LOG}\" | tee \"${ARTIFACT_LOG}\"".to_string(),
        _ => "\"${QEMU_BIN}\" -nographic -no-reboot -monitor none -m 512 -smp 2 -kernel \"${KERNEL}\" -initrd \"${INITRAMFS}\" -append \"console=ttyS0 loglevel=7 HYPERCORE_RUN_LINKED_PROBE=1\" 2>&1 | tee \"${LIVE_LOG}\" | tee \"${ARTIFACT_LOG}\"".to_string(),
    }
}

pub fn publish_linked_probe(
    repo_root: &Path,
    userspace_dir: &Path,
    smoke_dir: &Path,
    link_target: &Path,
    target_mode: &str,
    smoke_lines: &mut Vec<String>,
) -> Result<(), String> {
    let published_probe = userspace_dir.join("probe-linked.elf");
    fs::copy(link_target, &published_probe).map_err(|err| {
        format!(
            "failed to publish {} -> {}: {err}",
            link_target.display(),
            published_probe.display()
        )
    })?;

    let mut header_one_line = "unavailable".to_string();
    if let Some(readobj) = detect_llvm_readobj() {
        let output = Command::new(&readobj)
            .arg("--file-header")
            .arg(link_target)
            .current_dir(userspace_dir)
            .output()
            .map_err(|err| format!("failed to run {}: {err}", readobj.display()))?;
        let header_dump = lossy_bytes(&output.stdout, &output.stderr);
        fs::write(smoke_dir.join("probe-readobj.txt"), &header_dump)
            .map_err(|err| format!("failed to write probe-readobj.txt: {err}"))?;
        let selected = header_dump
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with("Format:")
                    || trimmed.starts_with("Arch:")
                    || trimmed.starts_with("Entry:")
                {
                    Some(trimmed.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if !selected.is_empty() {
            header_one_line = selected.join(" | ");
        }
    }

    fs::write(
        userspace_dir.join("probe-linked.txt"),
        [
            "[hypercore-userspace-linked-probe]".to_string(),
            "output=probe-linked.elf".to_string(),
            format!(
                "source=smoke-objects/{}",
                link_target
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("probe-smoke.elf")
            ),
            format!("header={header_one_line}"),
            String::new(),
        ]
        .join("\n"),
    )
    .map_err(|err| format!("failed to write probe-linked.txt: {err}"))?;

    let boot_image_dir = repo_root.join("artifacts").join("boot_image");
    let probe_iso_artifact = boot_image_dir.join("hypercore-probe.iso");
    let iso_artifact = boot_image_dir.join("hypercore.iso");
    let prefer_probe_iso_boot = probe_iso_artifact.exists() || boot_image_dir.exists();
    let prefer_iso_boot = prefer_probe_iso_boot || iso_artifact.exists();
    let host_root_rel = "..\\..\\..\\..\\..";
    let host_root_rel_sh = "../../../../../";
    let preferred_boot_mode = if prefer_probe_iso_boot {
        "probe_iso"
    } else if prefer_iso_boot {
        "iso"
    } else {
        "direct_kernel"
    };
    let qemu_binary = detect_qemu_binary().unwrap_or_else(|| PathBuf::from("qemu-system-x86_64"));

    let (qemu_command, probe_ps1_command, probe_sh_command) = if prefer_probe_iso_boot {
        (
            format!(
                "\"{}\" -nographic -no-reboot -monitor none -m 512 -smp 2 -cdrom \"{}artifacts/boot_image/hypercore-probe.iso\" -boot d",
                qemu_binary.display(),
                host_root_rel_sh
            ),
            qemu_probe_ps1_command("probe_iso"),
            qemu_probe_sh_command("probe_iso"),
        )
    } else if prefer_iso_boot {
        (
            format!(
                "\"{}\" -nographic -no-reboot -monitor none -m 512 -smp 2 -cdrom \"{}artifacts/boot_image/hypercore.iso\" -boot d",
                qemu_binary.display(),
                host_root_rel_sh
            ),
            qemu_probe_ps1_command("iso"),
            qemu_probe_sh_command("iso"),
        )
    } else {
        (
            format!(
                "\"{}\" -nographic -no-reboot -monitor none -m 512 -smp 2 -kernel \"{}artifacts/boot_image/stage/boot/hypercore.elf\" -initrd \"{}artifacts/boot_image/stage/boot/initramfs.cpio.gz\" -append \"console=ttyS0 loglevel=7 HYPERCORE_RUN_LINKED_PROBE=1\"",
                qemu_binary.display(),
                host_root_rel_sh,
                host_root_rel_sh
            ),
            qemu_probe_ps1_command("direct_kernel"),
            qemu_probe_sh_command("direct_kernel"),
        )
    };

    fs::write(
        userspace_dir.join("probe-boot-harness.txt"),
        [
            "[hypercore-userspace-boot-harness]".to_string(),
            "artifact=probe-linked.elf".to_string(),
            format!("preferred_boot_mode={preferred_boot_mode}"),
            "kernel_append=console=ttyS0 loglevel=7 HYPERCORE_RUN_LINKED_PROBE=1".to_string(),
            "qemu_mode=-nographic".to_string(),
            "expected_log=[hyper_init] linked probe exit status: 0".to_string(),
            "fallback=continue_boot_to_init_elf".to_string(),
            String::new(),
        ]
        .join("\n"),
    )
    .map_err(|err| format!("failed to write probe-boot-harness.txt: {err}"))?;
    fs::write(userspace_dir.join("probe-qemu-command.txt"), format!("{qemu_command}\n"))
        .map_err(|err| format!("failed to write probe-qemu-command.txt: {err}"))?;
    fs::write(
        userspace_dir.join("run-linked-probe.ps1"),
        [
            "$ErrorActionPreference = 'Stop'".to_string(),
            format!("$qemu = '{}'", qemu_binary.display()),
            format!("$workspaceRoot = Join-Path $PSScriptRoot '{host_root_rel}'"),
            "$kernel = Join-Path $workspaceRoot 'artifacts\\boot_image\\stage\\boot\\hypercore.elf'".to_string(),
            "$initramfs = Join-Path $workspaceRoot 'artifacts\\boot_image\\stage\\boot\\initramfs.cpio.gz'".to_string(),
            "$iso = Join-Path $workspaceRoot 'artifacts\\boot_image\\hypercore.iso'".to_string(),
            "$probeIso = Join-Path $workspaceRoot 'artifacts\\boot_image\\hypercore-probe.iso'".to_string(),
            "$liveLog = Join-Path $workspaceRoot 'artifacts\\boot_image\\qemu_linked_probe_live.log'".to_string(),
            "$artifactLog = Join-Path $workspaceRoot 'artifacts\\boot_image\\qemu_linked_probe.log'".to_string(),
            "$buildScript = Join-Path $workspaceRoot 'scripts\\hypercore.ps1'".to_string(),
            "& powershell -ExecutionPolicy Bypass -File $buildScript -Command build-iso".to_string(),
            probe_ps1_command,
            String::new(),
        ]
        .join("\n"),
    )
    .map_err(|err| format!("failed to write run-linked-probe.ps1: {err}"))?;
    fs::write(
        userspace_dir.join("run-linked-probe.sh"),
        [
            "#!/bin/sh".to_string(),
            "set -eu".to_string(),
            format!("QEMU_BIN=\"{}\"", qemu_binary.display()),
            format!("WORKSPACE_ROOT=\"{host_root_rel_sh}\""),
            "KERNEL=\"${WORKSPACE_ROOT}artifacts/boot_image/stage/boot/hypercore.elf\"".to_string(),
            "INITRAMFS=\"${WORKSPACE_ROOT}artifacts/boot_image/stage/boot/initramfs.cpio.gz\"".to_string(),
            "ISO=\"${WORKSPACE_ROOT}artifacts/boot_image/hypercore.iso\"".to_string(),
            "PROBE_ISO=\"${WORKSPACE_ROOT}artifacts/boot_image/hypercore-probe.iso\"".to_string(),
            "LIVE_LOG=\"${WORKSPACE_ROOT}artifacts/boot_image/qemu_linked_probe_live.log\"".to_string(),
            "ARTIFACT_LOG=\"${WORKSPACE_ROOT}artifacts/boot_image/qemu_linked_probe.log\"".to_string(),
            probe_sh_command,
            String::new(),
        ]
        .join("\n"),
    )
    .map_err(|err| format!("failed to write run-linked-probe.sh: {err}"))?;

    smoke_lines.extend([
        "link_probe_published=probe-linked.elf".to_string(),
        format!("link_probe_header={header_one_line}"),
        format!("link_probe_boot_mode={preferred_boot_mode}"),
        format!("link_probe_mode={target_mode}"),
        String::new(),
    ]);

    attempt_linked_probe_boot(repo_root, preferred_boot_mode)?;
    write_proof_files(repo_root, userspace_dir, Some(&published_probe))?;
    Ok(())
}

fn attempt_linked_probe_boot(repo_root: &Path, preferred_boot_mode: &str) -> Result<(), String> {
    let Some(qemu) = detect_qemu_binary() else {
        return Ok(());
    };
    let boot_image_dir = repo_root.join("artifacts").join("boot_image");
    let stage_boot_dir = boot_image_dir.join("stage").join("boot");
    let mut attempts = Vec::new();
    if preferred_boot_mode == "probe_iso" && boot_image_dir.join("hypercore-probe.iso").exists() {
        attempts.push((boot_image_dir.join("qemu_linked_probe.log"), qemu_probe_iso_args(&boot_image_dir)));
    } else if preferred_boot_mode == "iso" && boot_image_dir.join("hypercore.iso").exists() {
        attempts.push((boot_image_dir.join("qemu_linked_probe.log"), qemu_iso_args(&boot_image_dir)));
    }
    if stage_boot_dir.join("hypercore.elf").exists() && stage_boot_dir.join("initramfs.cpio.gz").exists() {
        attempts.push((boot_image_dir.join("qemu_linked_probe_direct.log"), qemu_direct_args(&stage_boot_dir)));
    }
    for (log_path, args) in attempts {
        let mode = log_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("linked_probe");
        let mut command = Command::new(&qemu);
        command.args(&args).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(_) => continue,
        };
        let deadline = Instant::now() + Duration::from_secs(120);
        let mut timed_out = false;
        loop {
            if let Ok(Some(_)) = child.try_wait() {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                timed_out = true;
                break;
            }
            thread::sleep(Duration::from_millis(250));
        }
        let output = child
            .wait_with_output()
            .map_err(|err| format!("failed to collect linked probe boot output: {err}"))?;
        let body = lossy_bytes(&output.stdout, &output.stderr);
        let decorated = format!(
            "{}{}",
            boot_attempt_log_prefix(mode, timed_out, output.status.code()),
            body
        );
        fs::write(&log_path, decorated)
            .map_err(|err| format!("failed to write {}: {err}", log_path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_qemu_arg_helpers_include_no_reboot_and_monitor_none() {
        let boot = Path::new("C:/boot");
        let stage = Path::new("C:/stage");

        for args in [qemu_probe_iso_args(boot), qemu_iso_args(boot), qemu_direct_args(stage)] {
            assert!(args.windows(2).any(|w| w == ["-no-reboot", "-monitor"]));
            assert!(args.windows(2).any(|w| w == ["-monitor", "none"]));
            assert!(args.contains(&"-nographic".to_string()));
        }
    }

    #[test]
    fn probe_script_commands_match_hardened_qemu_flags() {
        for mode in ["probe_iso", "iso", "direct_kernel"] {
            let ps1 = qemu_probe_ps1_command(mode);
            let sh = qemu_probe_sh_command(mode);
            assert!(ps1.contains("-no-reboot -monitor none"));
            assert!(sh.contains("-no-reboot -monitor none"));
            assert!(ps1.contains("Tee-Object -FilePath $liveLog"));
            assert!(sh.contains("tee \"${LIVE_LOG}\""));
        }
    }

    #[test]
    fn boot_attempt_log_prefix_captures_mode_timeout_and_exit() {
        let prefix = boot_attempt_log_prefix("qemu_linked_probe", true, Some(124));
        assert!(prefix.contains("mode=qemu_linked_probe"));
        assert!(prefix.contains("timed_out=yes"));
        assert!(prefix.contains("exit_code=124"));

        let prefix = boot_attempt_log_prefix("qemu_linked_probe_direct", false, None);
        assert!(prefix.contains("mode=qemu_linked_probe_direct"));
        assert!(prefix.contains("timed_out=no"));
        assert!(prefix.contains("exit_code=none"));
    }
}
