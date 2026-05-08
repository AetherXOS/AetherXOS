use crate::utils::ui::logging;
use anyhow::{Context, Result, anyhow, bail};
use std::fs;
use std::path::Path;

pub fn validate_elf(path: &Path) -> Result<()> {
    use xmas_elf::ElfFile;
    use xmas_elf::header::{Class, Data, Machine, Type};
    use xmas_elf::program::Type as PhType;

    let data =
        fs::read(path).with_context(|| format!("Failed to read ELF binary: {}", path.display()))?;
    let elf = ElfFile::new(&data).map_err(|e| anyhow!("ELF parse error: {}", e))?;

    let hdr = elf.header;

    // ── 1. Core Header Checks ───────────────────────────────────────────────
    if hdr.pt1.class() != Class::SixtyFour {
        bail!("Validation Fault: Only 64-bit ELF binaries are supported in AetherXOS.");
    }
    if hdr.pt1.data() != Data::LittleEndian {
        bail!("Validation Fault: Byte order mismatch. Expected Little Endian (x86_64).");
    }

    let machine = hdr.pt2.machine().as_machine();
    if machine != Machine::X86_64 && machine != Machine::AArch64 {
        bail!(
            "Validation Fault: Unsupported machine architecture: {:?}",
            machine
        );
    }

    let elf_type = hdr.pt2.type_().as_type();
    match elf_type {
        Type::Executable => {} // Ideal: ET_EXEC = static, no PIE
        Type::SharedObject => {
            // ET_DYN = PIE. Warn — suggests relocation-model=static is not applied.
            logging::warn(
                "elf",
                "Binary is ET_DYN (PIE) — expected ET_EXEC for a static bare-metal kernel",
                &[(
                    "hint",
                    "add '-C relocation-model=static' to kernel/.cargo/config.toml",
                )],
            );
        }
        other => bail!(
            "Validation Fault: Unexpected ELF type {:?}. Expected ET_EXEC.",
            other
        ),
    }

    // ── 2. Program Header Analysis ──────────────────────────────────────────
    let mut has_load = false;
    let mut has_interp = false;
    let mut has_dynamic = false;
    let mut nx_stack = false;

    for ph in elf.program_iter() {
        match ph.get_type() {
            Ok(PhType::Load) => {
                has_load = true;
                if !cfg!(debug_assertions) {
                    let align = ph.align();
                    if align > 0 && ph.virtual_addr() % align != 0 {
                        logging::warn(
                            "elf",
                            "PRODUCTION: improperly aligned segment",
                            &[
                                ("vaddr", &format!("{:#x}", ph.virtual_addr())),
                                ("align", &format!("{:#x}", align)),
                            ],
                        );
                    }
                }
            }
            Ok(PhType::Interp) => {
                has_interp = true;
            }
            Ok(PhType::Dynamic) => {
                has_dynamic = true;
            }
            _ => {
                // PT_GNU_STACK (0x6474e551) is not a named variant in xmas_elf 0.9.
                // Approximate: unknown type with no PF_X flag → NX stack marker.
                if ph.get_type().is_err() && (ph.flags().0 & 0x1 == 0) {
                    nx_stack = true;
                }
            }
        }
    }

    // ── 3. Security Gate ────────────────────────────────────────────────────
    if has_interp {
        bail!(
            "Security Fault: PT_INTERP (dynamic interpreter) found.\n  \
             A bare-metal kernel MUST NOT depend on a runtime linker.\n  \
             Fix: ensure linker.ld /DISCARD/ contains *(.interp)"
        );
    }

    if has_dynamic {
        // PT_DYNAMIC can appear in PIE binaries even with no real dependencies.
        // Distinguish:  empty/benign .dynamic  vs  actual DT_NEEDED entries.
        let has_real_deps = check_dynamic_has_deps(&data);
        if has_real_deps {
            bail!(
                "Security Fault: .dynamic section contains DT_NEEDED entries.\n  \
                 The kernel has actual runtime dependencies — this is INVALID for bare-metal.\n  \
                 Fix: add '-C relocation-model=static' to kernel/.cargo/config.toml\n  \
                 and ensure linker.ld discards *(.dynamic)"
            );
        } else {
            // Benign: PIE overhead with no real deps. Warn but don't fail.
            // This disappears once relocation-model=static is properly picked up.
            logging::warn(
                "elf",
                "PT_DYNAMIC present but empty (no DT_NEEDED) — benign in PIE, harmless in ET_EXEC",
                &[(
                    "hint",
                    "verify '-C relocation-model=static' is active; run with --release to confirm",
                )],
            );
        }
    }

    if !has_load {
        bail!("Structural Fault: No PT_LOAD segments found. Binary is not loadable.");
    }

    if !nx_stack {
        logging::warn(
            "elf",
            "stack not explicitly marked NX (PT_GNU_STACK missing)",
            &[(
                "hint",
                "linker.ld PHDRS block should include: stack PT_GNU_STACK FLAGS(6);",
            )],
        );
    }

    // ── 4. Entry Point ──────────────────────────────────────────────────────
    let entry = hdr.pt2.entry_point();
    if entry == 0 {
        bail!("Structural Fault: NULL entry point. Binary is non-bootable.");
    }

    // ── 5. Success Report ───────────────────────────────────────────────────
    let machine_str = match machine {
        Machine::X86_64 => "x86_64",
        Machine::AArch64 => "aarch64",
        _ => "unknown",
    };

    let elf_type_str = match elf_type {
        Type::Executable => "ET_EXEC (static) ✓",
        Type::SharedObject => "ET_DYN (PIE) ⚠",
        _ => "unknown",
    };

    let entry_str = format!("{:#x}", entry);
    let path_str = path.to_string_lossy();
    logging::info(
        "elf",
        "Binary integrity verified",
        &[
            ("path", path_str.as_ref()),
            ("machine", machine_str),
            ("type", elf_type_str),
            ("entry", entry_str.as_str()),
            ("nx_stack", if nx_stack { "YES ✓" } else { "NO ⚠" }),
            ("static", if !has_dynamic { "YES ✓" } else { "WARN" }),
        ],
    );

    Ok(())
}

/// Scan the raw `.dynamic` section for DT_NEEDED (tag=1) entries.
/// Returns true only when actual shared-library dependencies are declared.
/// An empty or metadata-only .dynamic section returns false (benign).
fn check_dynamic_has_deps(elf_data: &[u8]) -> bool {
    use xmas_elf::ElfFile;
    use xmas_elf::sections::SectionData;

    let elf = match ElfFile::new(elf_data) {
        Ok(e) => e,
        Err(_) => return false,
    };

    for section in elf.section_iter() {
        if section.get_name(&elf).unwrap_or("") != ".dynamic" {
            continue;
        }
        if let Ok(SectionData::Dynamic64(entries)) = section.get_data(&elf) {
            for entry in entries {
                // DT_NEEDED = 1 → actual runtime dependency
                if entry.get_tag() == Ok(xmas_elf::dynamic::Tag::Needed) {
                    return true;
                }
            }
        }
    }
    false
}
