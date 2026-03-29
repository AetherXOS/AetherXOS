use crate::models::UserspaceSnapshot;
use std::fs;
use std::path::Path;

use super::program_binaries;

pub fn emit_userspace_dir(snapshot: &UserspaceSnapshot, dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|err| format!("failed to create {}: {err}", dir.display()))?;

    write_files(dir, &snapshot.artifact_files)?;
    write_files(dir, &snapshot.build_files)?;
    write_files(dir, &snapshot.runtime.artifact_files)?;
    write_files(dir, &snapshot.elf.artifact_files)?;
    write_files(dir, &snapshot.libc.artifact_files)?;

    for (name, content) in &snapshot.runtime.source_blobs {
        fs::write(dir.join(name), content)
            .map_err(|err| format!("failed to write {}: {err}", dir.join(name).display()))?;
    }
    for (name, content) in &snapshot.libc.source_blobs {
        fs::write(dir.join(name), content)
            .map_err(|err| format!("failed to write {}: {err}", dir.join(name).display()))?;
    }

    let binaries = program_binaries::binaries();
    let mut program_names = Vec::new();
    for program in &snapshot.programs {
        program_names.push(program.output_name.to_string());
        fs::write(
            dir.join(format!("{}.program.txt", program.output_name.trim_end_matches(".elf"))),
            [
                format!(
                    "[hypercore-userspace-program:{}]",
                    program.output_name.trim_end_matches(".elf")
                ),
                format!("role={}", program.role),
                format!("output={}", program.output_name),
                format!("probe_features={}", program.probe_features.join(",")),
                format!("source_units={}", program.source_units.join(",")),
                format!("candidates={}", program.candidates.join(",")),
                String::new(),
            ]
            .join("\n"),
        )
        .map_err(|err| format!("failed to write program manifest {}: {err}", program.output_name))?;
        for (name, content) in &program.source_blobs {
            fs::write(dir.join(name), content)
                .map_err(|err| format!("failed to write {}: {err}", dir.join(name).display()))?;
        }
        if let Some(binary) = binaries.get(program.output_name) {
            fs::write(dir.join(program.output_name), binary)
                .map_err(|err| format!("failed to write {}: {err}", dir.join(program.output_name).display()))?;
        }
    }
    fs::write(dir.join("userspace-programs.txt"), format!("{}\n", program_names.join("\n")))
        .map_err(|err| format!("failed to write userspace-programs.txt: {err}"))?;
    Ok(())
}

fn write_files(dir: &Path, files: &std::collections::BTreeMap<String, String>) -> Result<(), String> {
    for (name, content) in files {
        fs::write(dir.join(name), content)
            .map_err(|err| format!("failed to write {}: {err}", dir.join(name).display()))?;
    }
    Ok(())
}
