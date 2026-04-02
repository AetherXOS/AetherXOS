use std::fs;
use std::path::Path;

fn decode_utf16le_lossy(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 2 || bytes.len() % 2 != 0 {
        return None;
    }
    let words: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    Some(String::from_utf16_lossy(&words))
}

fn read_text_lossy(path: &Path) -> String {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return String::new(),
    };

    if let Ok(text) = String::from_utf8(bytes.clone()) {
        return text;
    }

    if let Some(text) = decode_utf16le_lossy(&bytes) {
        return text;
    }

    String::from_utf8_lossy(&bytes).into_owned()
}

pub fn write_proof_files(
    repo_root: &Path,
    userspace_dir: &Path,
    published_probe: Option<&Path>,
) -> Result<(), String> {
    let boot_logs = [
        repo_root.join("artifacts/boot_image/qemu_probe_iso.log"),
        repo_root.join("artifacts/boot_image/qemu_iso_probe.log"),
        repo_root.join("artifacts/boot_image/qemu_smoke.log"),
        repo_root.join("artifacts/boot_image/qemu_probe_iso_manual.log"),
        repo_root.join("artifacts/boot_image/qemu_linked_probe.log"),
        repo_root.join("artifacts/boot_image/qemu_linked_probe_direct.log"),
    ];
    let expected_probe = "[aether_init] linked probe exit status: 0";
    let verified_probe = "[aether_init] linked probe execution verified";
    let mut scanned_logs = Vec::new();
    let mut probe_verified_in = None::<String>;
    let mut fini_observed_in = None::<String>;
    let mut kernel_stage = "no_boot_log";
    for log in boot_logs {
        if !log.exists() {
            continue;
        }
        let content = read_text_lossy(&log);
        scanned_logs.push(log.display().to_string());
        if kernel_stage == "no_boot_log" && content.contains("limine: Loading executable") {
            kernel_stage = "kernel_loaded";
        }
        if content.contains("smp: Successfully brought up AP") {
            kernel_stage = "smp_online";
        }
        let seabios_count = content.matches("SeaBIOS (version").count();
        if seabios_count > 1 && kernel_stage == "smp_online" {
            kernel_stage = "reboot_after_smp";
        }
        if content.contains("triple fault") || content.contains("Triple fault") {
            kernel_stage = "triple_fault_after_smp";
        }
        if content.contains("[EARLY SERIAL] runtime boot start") {
            kernel_stage = "runtime_boot_started";
        }
        if content.contains("[EARLY SERIAL] runtime boot mark_stage begin") {
            kernel_stage = "runtime_boot_mark_stage_begin";
        }
        if content.contains("[EARLY SERIAL] runtime boot mark_stage returned") {
            kernel_stage = "runtime_boot_mark_stage_returned";
        }
        if content.contains("[EARLY SERIAL] runtime boot prelude call begin") {
            kernel_stage = "runtime_boot_prelude_call_begin";
        }
        if content.contains("[EARLY SERIAL] runtime boot prelude returned") {
            kernel_stage = "runtime_boot_prelude_returned";
        }
        if content.contains("[EARLY SERIAL] prelude init begin")
            || content.contains("[EARLY SERIAL] prelude boot_info init returned")
            || content.contains("[EARLY SERIAL] prelude boot_info get returned")
            || content.contains("[EARLY SERIAL] prelude cmdline parsed")
            || content.contains("[EARLY SERIAL] kernel_runtime entered")
        {
            kernel_stage = "prelude_progress";
        }
        if content.contains("[EARLY SERIAL] runtime boot context ready") {
            kernel_stage = "runtime_boot_context_ready";
        }
        if content.contains("[EARLY SERIAL] heap init call begin") {
            kernel_stage = "heap_call_started";
        }
        if content.contains("[EARLY SERIAL] heap init entry") {
            kernel_stage = "heap_entry";
        }
        if content.contains("[EARLY SERIAL] slab init begin") {
            kernel_stage = "slab_init_begin";
        }
        if content.contains("[EARLY SERIAL] linked list heap init begin") {
            kernel_stage = "linked_list_heap_init_begin";
        }
        if content.contains("[EARLY SERIAL] linked list heap init returned") {
            kernel_stage = "linked_list_heap_init_returned";
        }
        if content.contains("[EARLY SERIAL] slab init returned") {
            kernel_stage = "slab_init_returned";
        }
        if content.contains("[EARLY SERIAL] heap allocator init complete") {
            kernel_stage = "heap_allocator_init_complete";
        }
        if content.contains("[EARLY SERIAL] after_heap_init hook returned") {
            kernel_stage = "after_heap_init_hook_returned";
        }
        if content.contains("[EARLY SERIAL] hal early init call begin") {
            kernel_stage = "hal_early_init_call_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 serial initialized") {
            kernel_stage = "hal_serial_initialized";
        }
        if content.contains("[EARLY SERIAL] x86_64 bootstrap gdt request begin") {
            kernel_stage = "hal_bootstrap_gdt_request_begin";
        }
        if content.contains("[EARLY SERIAL] bootstrap gdt init begin") {
            kernel_stage = "bootstrap_gdt_init_begin";
        }
        if content.contains("[EARLY SERIAL] gdt new begin") {
            kernel_stage = "gdt_new_begin";
        }
        if content.contains("[EARLY SERIAL] gdt tss created") {
            kernel_stage = "gdt_tss_created";
        }
        if content.contains("[EARLY SERIAL] gdt double fault ist ready") {
            kernel_stage = "gdt_double_fault_ist_ready";
        }
        if content.contains("[EARLY SERIAL] gdt page fault ist ready") {
            kernel_stage = "gdt_page_fault_ist_ready";
        }
        if content.contains("[EARLY SERIAL] gdt table created") {
            kernel_stage = "gdt_table_created";
        }
        if content.contains("[EARLY SERIAL] gdt selectors added") {
            kernel_stage = "gdt_selectors_added";
        }
        if content.contains("[EARLY SERIAL] gdt new returning") {
            kernel_stage = "gdt_new_returning";
        }
        if content.contains("[EARLY SERIAL] bootstrap gdt init returned") {
            kernel_stage = "bootstrap_gdt_init_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 bootstrap gdt request returned") {
            kernel_stage = "hal_bootstrap_gdt_request_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 gdt load call begin") {
            kernel_stage = "hal_gdt_load_call_begin";
        }
        if content.contains("[EARLY SERIAL] gdt load begin") {
            kernel_stage = "gdt_load_begin";
        }
        if content.contains("[EARLY SERIAL] gdt tss descriptor added") {
            kernel_stage = "gdt_tss_descriptor_added";
        }
        if content.contains("[EARLY SERIAL] gdt table loaded") {
            kernel_stage = "gdt_table_loaded";
        }
        if content.contains("[EARLY SERIAL] gdt cs loaded") {
            kernel_stage = "gdt_cs_loaded";
        }
        if content.contains("[EARLY SERIAL] gdt tss loaded") {
            kernel_stage = "gdt_tss_loaded";
        }
        if content.contains("[EARLY SERIAL] gdt data segments loaded") {
            kernel_stage = "gdt_data_segments_loaded";
        }
        if content.contains("[EARLY SERIAL] x86_64 gdt loaded") {
            kernel_stage = "hal_gdt_loaded";
        }
        if content.contains("[EARLY SERIAL] x86_64 idt initialized") {
            kernel_stage = "hal_idt_initialized";
        }
        if content.contains("[EARLY SERIAL] x86_64 local apic initialized") {
            kernel_stage = "hal_local_apic_initialized";
        }
        if content.contains("[EARLY SERIAL] x86_64 syscalls initialized") {
            kernel_stage = "hal_syscalls_initialized";
        }
        if content.contains("[EARLY SERIAL] x86_64 post-syscall checkpoint returned") {
            kernel_stage = "hal_post_syscall_checkpoint_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 bsp cpu local request begin") {
            kernel_stage = "hal_bsp_cpu_local_request_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 bsp cpu local bootstrap begin") {
            kernel_stage = "hal_bsp_cpu_local_bootstrap_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 bsp scheduler create begin") {
            kernel_stage = "hal_bsp_scheduler_create_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 early call checkpoint entered") {
            kernel_stage = "hal_early_call_checkpoint_entered";
        }
        if content.contains("[EARLY SERIAL] x86_64 early call checkpoint returned") {
            kernel_stage = "hal_early_call_checkpoint_returned";
        }
        if content.contains("[EARLY SERIAL] bootstrap active scheduler wrapper begin") {
            kernel_stage = "hal_bootstrap_active_scheduler_wrapper_begin";
        }
        if content.contains("[EARLY SERIAL] bootstrap active scheduler wrapper returned") {
            kernel_stage = "hal_bootstrap_active_scheduler_wrapper_returned";
        }
        if content.contains("[EARLY SERIAL] cfs new begin") {
            kernel_stage = "hal_cfs_new_begin";
        }
        if content.contains("[EARLY SERIAL] cfs timeline map begin") {
            kernel_stage = "hal_cfs_timeline_map_begin";
        }
        if content.contains("[EARLY SERIAL] cfs timeline map returned") {
            kernel_stage = "hal_cfs_timeline_map_returned";
        }
        if content.contains("[EARLY SERIAL] cfs task metadata map begin") {
            kernel_stage = "hal_cfs_task_metadata_map_begin";
        }
        if content.contains("[EARLY SERIAL] cfs task metadata map returned") {
            kernel_stage = "hal_cfs_task_metadata_map_returned";
        }
        if content.contains("[EARLY SERIAL] cfs tasks map begin") {
            kernel_stage = "hal_cfs_tasks_map_begin";
        }
        if content.contains("[EARLY SERIAL] cfs tasks map returned") {
            kernel_stage = "hal_cfs_tasks_map_returned";
        }
        if content.contains("[EARLY SERIAL] cfs groups map begin") {
            kernel_stage = "hal_cfs_groups_map_begin";
        }
        if content.contains("[EARLY SERIAL] cfs groups map returned") {
            kernel_stage = "hal_cfs_groups_map_returned";
        }
        if content.contains("[EARLY SERIAL] cfs group timeline map begin") {
            kernel_stage = "hal_cfs_group_timeline_map_begin";
        }
        if content.contains("[EARLY SERIAL] cfs group timeline map returned") {
            kernel_stage = "hal_cfs_group_timeline_map_returned";
        }
        if content.contains("[EARLY SERIAL] cfs schedstats map begin") {
            kernel_stage = "hal_cfs_schedstats_map_begin";
        }
        if content.contains("[EARLY SERIAL] cfs schedstats map returned") {
            kernel_stage = "hal_cfs_schedstats_map_returned";
        }
        if content.contains("[EARLY SERIAL] cfs autogroups map begin") {
            kernel_stage = "hal_cfs_autogroups_map_begin";
        }
        if content.contains("[EARLY SERIAL] cfs autogroups map returned") {
            kernel_stage = "hal_cfs_autogroups_map_returned";
        }
        if content.contains("[EARLY SERIAL] cfs new returned") {
            kernel_stage = "hal_cfs_new_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 bsp scheduler create returned") {
            kernel_stage = "hal_bsp_scheduler_create_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 bsp scheduler mutex begin") {
            kernel_stage = "hal_bsp_scheduler_mutex_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 bsp scheduler mutex returned") {
            kernel_stage = "hal_bsp_scheduler_mutex_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 bsp cpu local write begin") {
            kernel_stage = "hal_bsp_cpu_local_write_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 bsp cpu local write returned") {
            kernel_stage = "hal_bsp_cpu_local_write_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 bsp cpu local bootstrap returned") {
            kernel_stage = "hal_bsp_cpu_local_bootstrap_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 bsp cpu local request returned") {
            kernel_stage = "hal_bsp_cpu_local_request_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 cpu local init begin") {
            kernel_stage = "hal_cpu_local_init_begin";
        }
        if content.contains("[EARLY SERIAL] cpu local gsbase write begin") {
            kernel_stage = "hal_cpu_local_gsbase_write_begin";
        }
        if content.contains("[EARLY SERIAL] cpu local gsbase write returned") {
            kernel_stage = "hal_cpu_local_gsbase_write_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 cpu local initialized") {
            kernel_stage = "hal_cpu_local_initialized";
        }
        if content.contains("[EARLY SERIAL] x86_64 bsp register begin") {
            kernel_stage = "hal_bsp_register_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 register_cpu push begin") {
            kernel_stage = "hal_register_cpu_push_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 register_cpu push returned") {
            kernel_stage = "hal_register_cpu_push_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 bsp registered") {
            kernel_stage = "hal_bsp_registered";
        }
        if content.contains("[EARLY SERIAL] hal early init returned") {
            kernel_stage = "hal_early_init_returned";
        }
        if content.contains("[EARLY SERIAL] after_hal_early_init hook returned") {
            kernel_stage = "after_hal_early_init_hook_returned";
        }
        if content.contains("[EARLY SERIAL] platform services call begin") {
            kernel_stage = "platform_services_call_begin";
        }
        if content.contains("[EARLY SERIAL] after_platform_services hook returned") {
            kernel_stage = "after_platform_services_hook_returned";
        }
        if content.contains("[EARLY SERIAL] prelude finalize deferred") {
            kernel_stage = "prelude_finalize_deferred";
        }
        if content.contains("[EARLY SERIAL] prelude finalize skipped in boot path") {
            kernel_stage = "prelude_finalize_skipped";
        }
        if content.contains("[EARLY SERIAL] runtime boot entering main loop")
            || content.contains("[EARLY SERIAL] main loop entered")
        {
            kernel_stage = "main_loop_entered";
        }
        if content.contains("[EARLY SERIAL] platform services returned") {
            kernel_stage = "platform_services_returned";
        }
        if content.contains("[EARLY SERIAL] runtime activation returned") {
            kernel_stage = "runtime_activation_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap entry begin") {
            kernel_stage = "ap_entry_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap cpu id ready") {
            kernel_stage = "ap_cpu_id_ready";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap gdt bundle begin") {
            kernel_stage = "ap_gdt_bundle_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap gdt heap alloc begin") {
            kernel_stage = "ap_gdt_heap_alloc_begin";
        }
        if content.contains("[EARLY SERIAL] ap gdt slot write begin") {
            kernel_stage = "ap_gdt_slot_write_begin";
        }
        if content.contains("[EARLY SERIAL] ap gdt slot write returned") {
            kernel_stage = "ap_gdt_slot_write_returned";
        }
        if content.contains("[EARLY SERIAL] ap gdt ready mask set begin") {
            kernel_stage = "ap_gdt_ready_mask_set_begin";
        }
        if content.contains("[EARLY SERIAL] ap gdt ready mask set returned") {
            kernel_stage = "ap_gdt_ready_mask_set_returned";
        }
        if content.contains("[EARLY SERIAL] ap gdt slot return begin") {
            kernel_stage = "ap_gdt_slot_return_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap gdt heap alloc returned") {
            kernel_stage = "ap_gdt_heap_alloc_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap gdt bundle returned") {
            kernel_stage = "ap_gdt_bundle_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap gdt load begin") {
            kernel_stage = "ap_gdt_load_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap gdt load returned") {
            kernel_stage = "ap_gdt_load_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap local apic begin") {
            kernel_stage = "ap_local_apic_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap local apic returned") {
            kernel_stage = "ap_local_apic_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap cpu local alloc begin") {
            kernel_stage = "ap_cpu_local_alloc_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap scheduler create begin") {
            kernel_stage = "ap_scheduler_create_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap scheduler create returned") {
            kernel_stage = "ap_scheduler_create_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap scheduler mutex begin") {
            kernel_stage = "ap_scheduler_mutex_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap scheduler mutex returned") {
            kernel_stage = "ap_scheduler_mutex_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap cpu local heap alloc begin") {
            kernel_stage = "ap_cpu_local_heap_alloc_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap cpu local heap alloc returned") {
            kernel_stage = "ap_cpu_local_heap_alloc_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap cpu local alloc returned") {
            kernel_stage = "ap_cpu_local_alloc_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap cpu local init begin") {
            kernel_stage = "ap_cpu_local_init_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap cpu local init returned") {
            kernel_stage = "ap_cpu_local_init_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap syscalls init begin") {
            kernel_stage = "ap_syscalls_init_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap syscalls init returned") {
            kernel_stage = "ap_syscalls_init_returned";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap register cpu begin") {
            kernel_stage = "ap_register_cpu_begin";
        }
        if content.contains("[EARLY SERIAL] x86_64 ap register cpu returned") {
            kernel_stage = "ap_register_cpu_returned";
        }
        if content.contains("[EARLY SERIAL] interrupts enabled") {
            kernel_stage = "interrupts_enabled";
        }
        if content.contains("[EARLY SERIAL] linked probe linux compat ready")
            || content.contains("[EARLY SERIAL] linked probe main loop armed")
        {
            kernel_stage = "linked_probe_boot_kernel_ready";
        }
        if content.contains("[LINKED PROBE] probe boot requested")
            || content.contains("[LINKED PROBE] main loop armed for linked probe boot")
            || content.contains("[LINKED PROBE] linux-compat ready; awaiting aether_init probe execution")
        {
            kernel_stage = "linked_probe_boot_kernel_ready";
        }
        if probe_verified_in.is_none()
            && (content.contains(expected_probe) || content.contains(verified_probe))
        {
            probe_verified_in = Some(log.display().to_string());
        }
        if fini_observed_in.is_none()
            && (content.contains("runtime_fini_entry=")
                || content.contains("[aethercore-libc] runtime fini trampoline completed")
                || content.contains("[aethercore-libc] runtime fini counts:"))
        {
            fini_observed_in = Some(log.display().to_string());
        }
    }

    let probe_status = if probe_verified_in.is_some() {
        "verified"
    } else if published_probe.is_some() {
        "linked_not_boot_verified"
    } else {
        "not_linked"
    };
    fs::write(
        userspace_dir.join("probe-execution-proof.txt"),
        [
            "[aethercore-userspace-probe-proof]".to_string(),
            format!("status={probe_status}"),
            format!("artifact_present={}", yes_no(published_probe.is_some())),
            format!("expected_log={expected_probe}"),
            format!(
                "verified_in={}",
                probe_verified_in.unwrap_or_else(|| "none".to_string())
            ),
            format!(
                "scanned_logs={}",
                if scanned_logs.is_empty() {
                    "none".to_string()
                } else {
                    scanned_logs.join(",")
                }
            ),
            format!("kernel_stage={kernel_stage}"),
            String::new(),
        ]
        .join("\n"),
    )
    .map_err(|err| format!("failed to write probe-execution-proof.txt: {err}"))?;

    let telemetry_text = fs::read_to_string(userspace_dir.join("runtime-fini-telemetry.txt"))
        .unwrap_or_else(|_| String::new());
    let fini_status = if fini_observed_in.is_some() {
        "kernel_log_observed"
    } else if telemetry_text.contains("aethercore_runtime_fini_attempt_count") {
        "telemetry_ready"
    } else {
        "missing"
    };
    fs::write(
        userspace_dir.join("runtime-fini-proof.txt"),
        [
            "[aethercore-runtime-fini-proof]".to_string(),
            format!("status={fini_status}"),
            format!(
                "observed_in={}",
                fini_observed_in.unwrap_or_else(|| "none".to_string())
            ),
            "expected_kernel_marker=runtime_fini_entry=".to_string(),
            format!(
                "telemetry_symbols_present={}",
                yes_no(telemetry_text.contains("aethercore_runtime_fini_attempt_count"))
            ),
            String::new(),
        ]
        .join("\n"),
    )
    .map_err(|err| format!("failed to write runtime-fini-proof.txt: {err}"))?;
    Ok(())
}

pub fn lossy_bytes(stdout: &[u8], stderr: &[u8]) -> String {
    let mut out = String::from_utf8_lossy(stdout).into_owned();
    out.push_str(&String::from_utf8_lossy(stderr));
    out
}

pub fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
