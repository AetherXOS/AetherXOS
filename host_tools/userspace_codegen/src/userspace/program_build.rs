use crate::models::ProgramSnapshot;
use std::collections::BTreeMap;

pub fn build_files(programs: &[ProgramSnapshot]) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    let mut manifest_lines = vec!["[hypercore-userspace-build]".to_string()];
    let mut mk_lines = vec![
        "# hypercore generated userspace build skeleton".to_string(),
        "CC ?= cc".to_string(),
        "CFLAGS ?= -ffreestanding -fno-builtin -nostdlib -I.".to_string(),
        String::new(),
    ];
    let mut sh_lines = vec![
        "#!/bin/sh".to_string(),
        "set -eu".to_string(),
        "CC=\"${CC:-cc}\"".to_string(),
        "AR=\"${AR:-ar}\"".to_string(),
        "CFLAGS=\"${CFLAGS:--ffreestanding -fno-builtin -nostdlib -I.}\"".to_string(),
        "LDFLAGS=\"${LDFLAGS:--nostdlib -T hypercore_userspace.ld}\"".to_string(),
        "STARTUP_OBJ=\"${STARTUP_OBJ:-crt0.o}\"".to_string(),
        String::new(),
        "echo \"[hypercore-userspace-build] using CC=${CC}\"".to_string(),
        "echo \"[hypercore-userspace-build] using AR=${AR}\"".to_string(),
        String::new(),
    ];

    for program in programs {
        let stem = program.output_name.trim_end_matches(".elf");
        let upper = stem.to_uppercase();
        let object_list = program
            .source_units
            .iter()
            .map(|unit| unit.trim_end_matches(".c").to_string() + ".o")
            .collect::<Vec<_>>()
            .join(" ");
        let source_list = program.source_units.join(",");
        manifest_lines.extend([
            format!("program={stem}"),
            format!("role={}", program.role),
            format!("sources={source_list}"),
            format!("objects={}", object_list.replace(' ', ",")),
            format!("output={}", program.output_name),
            "linker_script=hypercore_userspace.ld".to_string(),
            "startup_object=crt0.o".to_string(),
        ]);
        mk_lines.extend([
            format!("{upper}_SOURCES := {}", program.source_units.join(" ")),
            format!("{upper}_OBJECTS := {object_list}"),
            format!("{stem}: $({upper}_OBJECTS) $(STARTUP_OBJ)"),
            format!("\t@echo Linking {} from $({upper}_OBJECTS) $(STARTUP_OBJ)", program.output_name),
            format!("\t$(CC) $(LDFLAGS) -o {} $(STARTUP_OBJ) $({upper}_OBJECTS)", program.output_name),
            String::new(),
        ]);
        sh_lines.extend([
            format!("echo \"[hypercore-userspace-build] building {}\"", program.output_name),
            format!("for src in {}; do", source_list.replace(',', " ")),
            "  obj=\"${src%.c}.o\"".to_string(),
            "  \"${CC}\" ${CFLAGS} -c \"${src}\" -o \"${obj}\"".to_string(),
            "done".to_string(),
            format!("\"${{CC}}\" ${{LDFLAGS}} -o \"{}\" \"${{STARTUP_OBJ}}\" {object_list}", program.output_name),
            String::new(),
        ]);
    }

    manifest_lines.push(String::new());
    mk_lines.extend([
        "%.o: %.c".to_string(),
        "\t$(CC) $(CFLAGS) -c $< -o $@".to_string(),
        String::new(),
        format!(
            "all: {}",
            programs
                .iter()
                .map(|p| p.output_name.trim_end_matches(".elf"))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        String::new(),
    ]);
    sh_lines.extend([
        "echo \"[hypercore-userspace-build] done\"".to_string(),
        String::new(),
    ]);

    files.insert("userspace-build.txt".to_string(), manifest_lines.join("\n"));
    files.insert("userspace.mk".to_string(), mk_lines.join("\n"));
    files.insert("build-userspace.sh".to_string(), sh_lines.join("\n"));
    files.insert(
        "hypercore_userspace.ld".to_string(),
        [
            "ENTRY(_start)",
            "PHDRS",
            "{",
            "  text PT_LOAD FLAGS(5);",
            "  rodata PT_LOAD FLAGS(4);",
            "  data PT_LOAD FLAGS(6);",
            "}",
            "SECTIONS",
            "{",
            "  . = 0x400000;",
            "  .text ALIGN(0x1000) : { *(.text*) } :text",
            "  . = ALIGN(0x1000);",
            "  .rodata ALIGN(0x1000) : { *(.rodata*) *(.eh_frame*) *(.gcc_except_table*) } :rodata",
            "  . = ALIGN(0x1000);",
            "  .data ALIGN(0x1000) : { *(.data*) } :data",
            "  .bss ALIGN(0x1000) : { *(.bss*) *(COMMON) } :data",
            "  /DISCARD/ : { *(.note.GNU-stack) }",
            "}",
            "",
        ]
        .join("\n"),
    );
    files.insert(
        "crt0-plan.txt".to_string(),
        [
            "[hypercore-userspace-startup]",
            "startup_object=crt0.o",
            "startup_source=crt0.S",
            "entry_symbol=_start",
            "handoff_symbol=__hypercore_crt0_start",
            "",
        ]
        .join("\n"),
    );
    files.insert(
        "probe-exec-plan.txt".to_string(),
        [
            "[hypercore-userspace-exec-plan]",
            "artifact=probe-linked.elf",
            "mode=optional_boot_probe",
            "gate_env=HYPERCORE_RUN_LINKED_PROBE",
            "expected_exit=0",
            "on_failure=continue_boot",
            "",
        ]
        .join("\n"),
    );
    files.insert(
        "probe-boot-harness.txt".to_string(),
        [
            "[hypercore-userspace-boot-harness]",
            "artifact=probe-linked.elf",
            "preferred_boot_mode=auto",
            "kernel_append=console=ttyS0 loglevel=7 HYPERCORE_RUN_LINKED_PROBE=1",
            "qemu_mode=-nographic",
            "expected_log=[hyper_init] linked probe exit status: 0",
            "fallback=continue_boot_to_init_elf",
            "",
        ]
        .join("\n"),
    );
    files.insert(
        "probe-kernel-append.txt".to_string(),
        "console=ttyS0 loglevel=7 HYPERCORE_RUN_LINKED_PROBE=1\n".to_string(),
    );
    files.insert(
        "probe-iso-plan.txt".to_string(),
        [
            "[hypercore-userspace-probe-iso]",
            "preferred_iso=hypercore-probe.iso",
            "fallback_iso=hypercore.iso",
            "embedded_kernel_append=console=ttyS0 loglevel=7 HYPERCORE_RUN_LINKED_PROBE=1",
            "",
        ]
        .join("\n"),
    );
    files.insert(
        "probe-qemu-command.txt".to_string(),
        "\"${QEMU_BIN:-qemu-system-x86_64}\" -nographic -m 512 -smp 2 <boot-mode-specific-args>\n"
            .to_string(),
    );
    files.insert(
        "run-linked-probe.sh".to_string(),
        [
            "#!/bin/sh",
            "set -eu",
            "QEMU_BIN=\"${QEMU_BIN:-qemu-system-x86_64}\"",
            "KERNEL=\"${KERNEL:-../../../../../artifacts/boot_image/stage/boot/hypercore.elf}\"",
            "INITRAMFS=\"${INITRAMFS:-../../../../../artifacts/boot_image/stage/boot/initramfs.cpio.gz}\"",
            "ISO=\"${ISO:-../../../../../artifacts/boot_image/hypercore.iso}\"",
            "PROBE_ISO=\"${PROBE_ISO:-../../../../../artifacts/boot_image/hypercore-probe.iso}\"",
            "echo \"Override QEMU_BIN/KERNEL/INITRAMFS/ISO/PROBE_ISO as needed, then run ${QEMU_BIN}.\"",
            "",
        ]
        .join("\n"),
    );
    files.insert(
        "run-linked-probe.ps1".to_string(),
        [
            "$ErrorActionPreference = 'Stop'",
            "$qemu = if ($env:QEMU_BIN) { $env:QEMU_BIN } else { 'qemu-system-x86_64' }",
            "$kernel = if ($env:KERNEL) { $env:KERNEL } else { '..\\..\\..\\..\\..\\artifacts\\boot_image\\stage\\boot\\hypercore.elf' }",
            "$initramfs = if ($env:INITRAMFS) { $env:INITRAMFS } else { '..\\..\\..\\..\\..\\artifacts\\boot_image\\stage\\boot\\initramfs.cpio.gz' }",
            "$iso = if ($env:ISO) { $env:ISO } else { '..\\..\\..\\..\\..\\artifacts\\boot_image\\hypercore.iso' }",
            "$probeIso = if ($env:PROBE_ISO) { $env:PROBE_ISO } else { '..\\..\\..\\..\\..\\artifacts\\boot_image\\hypercore-probe.iso' }",
            "Write-Host 'Override QEMU_BIN/KERNEL/INITRAMFS/ISO/PROBE_ISO as needed, then run qemu.'",
            "",
        ]
        .join("\n"),
    );
    files
}
