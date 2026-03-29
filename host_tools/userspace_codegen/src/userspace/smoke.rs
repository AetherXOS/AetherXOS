use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use super::smoke_boot::publish_linked_probe;
use super::smoke_detect::{compile_flags, detect_host_c_compiler};
use super::smoke_proof::{lossy_bytes, write_proof_files};

pub fn run_generated_userspace_smoke(repo_root: &Path, userspace_dir: &Path) -> Result<(), String> {
    let compiler = detect_host_c_compiler();
    let mut smoke_lines = vec!["[hypercore-userspace-smoke]".to_string()];
    if compiler.is_none() {
        smoke_lines.extend([
            "status=compiler_missing".to_string(),
            "compiled_units=".to_string(),
            "failed_units=".to_string(),
            String::new(),
        ]);
        fs::write(userspace_dir.join("userspace-smoke.txt"), smoke_lines.join("\n"))
            .map_err(|err| format!("failed to write userspace-smoke.txt: {err}"))?;
        write_proof_files(repo_root, userspace_dir, None)?;
        return Ok(());
    }

    let compiler = compiler.unwrap();
    let smoke_dir = userspace_dir.join("smoke-objects");
    if smoke_dir.exists() {
        fs::remove_dir_all(&smoke_dir)
            .map_err(|err| format!("failed to clean {}: {err}", smoke_dir.display()))?;
    }
    fs::create_dir_all(&smoke_dir)
        .map_err(|err| format!("failed to create {}: {err}", smoke_dir.display()))?;

    let runtime_source_units = read_list(&userspace_dir.join("runtime-source-units.txt"));
    let libc_source_modules = read_list(&userspace_dir.join("libc-source-modules.txt"));
    let program_source_units = read_program_source_units(userspace_dir)?;
    let source_units = dedup_units(
        runtime_source_units
            .into_iter()
            .chain(libc_source_modules)
            .chain(program_source_units),
    );

    let (target_mode, linker_bin, cflags) = compile_flags(userspace_dir, &compiler);

    let mut compiled_units = Vec::new();
    let mut failed_units = Vec::new();
    for unit in &source_units {
        let src = userspace_dir.join(unit);
        let obj_name = format!("{}.o", file_stem(unit));
        let obj = smoke_dir.join(&obj_name);
        let output = Command::new(&compiler)
            .args(&cflags)
            .arg("-c")
            .arg(&src)
            .arg("-o")
            .arg(&obj)
            .current_dir(userspace_dir)
            .output()
            .map_err(|err| format!("failed to compile {}: {err}", src.display()))?;
        if output.status.success() {
            compiled_units.push(unit.clone());
        } else {
            failed_units.push(format!("{unit}:{}", output.status.code().unwrap_or(-1)));
            fs::write(
                smoke_dir.join(format!("{}.log", file_stem(unit))),
                lossy_bytes(&output.stdout, &output.stderr),
            )
            .map_err(|err| format!("failed to write compile log for {unit}: {err}"))?;
        }
    }

    smoke_lines.extend([
        format!(
            "status={}",
            if failed_units.is_empty() { "pass" } else { "partial" }
        ),
        format!("compiler={}", compiler.display()),
        format!("target_mode={target_mode}"),
        format!("compiled_units={}", compiled_units.join(",")),
        format!("failed_units={}", failed_units.join(",")),
        format!(
            "smoke_object_dir={}",
            smoke_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("smoke-objects")
        ),
        String::new(),
    ]);

    let mut link_status = "skipped".to_string();
    let link_target = smoke_dir.join("probe-smoke.elf");
    if failed_units.is_empty() {
        let runtime_objs = [
            "runtime_state.o",
            "auxv_runtime.o",
            "env_runtime.o",
            "runtime_syscall.o",
            "runtime_entry.o",
            "runtime_probe.o",
            "runtime_smoke.o",
            "libc_state.o",
            "errno_runtime.o",
            "memory_runtime.o",
            "string_runtime.o",
            "startup_runtime.o",
            "libc_syscall.o",
            "probe_main.o",
            "probe_report.o",
        ]
        .into_iter()
        .map(|name| smoke_dir.join(name))
        .collect::<Vec<_>>();
        let startup_obj = smoke_dir.join("crt0.o");
        let link_output = if target_mode == "elf" {
            Command::new(&linker_bin)
                .arg("-m")
                .arg("elf_x86_64")
                .arg("-T")
                .arg(userspace_dir.join("hypercore_userspace.ld"))
                .arg("-o")
                .arg(&link_target)
                .arg(&startup_obj)
                .args(runtime_objs.iter())
                .current_dir(userspace_dir)
                .output()
        } else {
            Command::new(&compiler)
                .arg("-nostdlib")
                .arg("-fuse-ld=lld")
                .arg(format!(
                    "-Wl,-T,{}",
                    userspace_dir.join("hypercore_userspace.ld").display()
                ))
                .arg("-o")
                .arg(&link_target)
                .arg(&startup_obj)
                .args(runtime_objs.iter())
                .current_dir(userspace_dir)
                .output()
        }
        .map_err(|err| format!("failed to link probe smoke ELF: {err}"))?;
        if link_output.status.success() {
            link_status = "pass".to_string();
        } else {
            link_status = format!("fail:{}", link_output.status.code().unwrap_or(-1));
            fs::write(
                smoke_dir.join("probe-link.log"),
                lossy_bytes(&link_output.stdout, &link_output.stderr),
            )
            .map_err(|err| format!("failed to write probe-link.log: {err}"))?;
        }
    }

    smoke_lines.extend([
        format!("link_probe_status={link_status}"),
        format!(
            "link_probe_output={}",
            if link_status == "pass" {
                "probe-smoke.elf"
            } else {
                ""
            }
        ),
        format!("link_probe_mode={target_mode}"),
        String::new(),
    ]);

    if link_status == "pass" {
        publish_linked_probe(
            repo_root,
            userspace_dir,
            &smoke_dir,
            &link_target,
            &target_mode,
            &mut smoke_lines,
        )?;
    } else {
        write_proof_files(repo_root, userspace_dir, None)?;
    }

    fs::write(userspace_dir.join("userspace-smoke.txt"), smoke_lines.join("\n"))
        .map_err(|err| format!("failed to write userspace-smoke.txt: {err}"))?;
    Ok(())
}

fn dedup_units<I>(units: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for unit in units {
        if seen.insert(unit.clone()) {
            out.push(unit);
        }
    }
    out
}

fn read_list(path: &Path) -> Vec<String> {
    match fs::read_to_string(path) {
        Ok(content) => content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn read_program_source_units(userspace_dir: &Path) -> Result<Vec<String>, String> {
    let mut units = Vec::new();
    for program_name in read_list(&userspace_dir.join("userspace-programs.txt")) {
        let manifest = userspace_dir.join(format!(
            "{}.program.txt",
            program_name.trim_end_matches(".elf")
        ));
        if !manifest.exists() {
            continue;
        }
        let content = fs::read_to_string(&manifest)
            .map_err(|err| format!("failed to read {}: {err}", manifest.display()))?;
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("source_units=") {
                units.extend(
                    rest.split(',')
                        .map(str::trim)
                        .filter(|unit| !unit.is_empty())
                        .map(ToOwned::to_owned),
                );
            }
        }
    }
    Ok(units)
}

fn file_stem(unit: &str) -> String {
    Path::new(unit)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(unit)
        .to_string()
}
