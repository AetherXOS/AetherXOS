use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::utils::{paths, report};

#[derive(Debug, Serialize)]
struct DesktopPlanReport {
    schema_version: u32,
    wayland_score: f64,
    x11_score: f64,
    graphics_readiness: f64,
    phases: Vec<PlanPhase>,
    prerequisites: Vec<PlanItem>,
    immediate_backlog: Vec<PlanItem>,
    notes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PlanPhase {
    id: String,
    name: String,
    duration_weeks: String,
    goal: String,
    success_criteria: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PlanItem {
    area: String,
    item: String,
    status: String,
    owner_hint: String,
}

pub fn run() -> Result<()> {
    println!("[linux-abi::desktop-plan] Building desktop integration roadmap");

    let root = paths::repo_root();
    let out_dir = paths::resolve("reports/linux_desktop_plan");
    paths::ensure_dir(&out_dir)?;

    let wayland_score = score_stack(&root, "wayland");
    let x11_score = score_stack(&root, "x11");
    let graphics_readiness = ((wayland_score + x11_score) / 2.0 * 10.0).round() / 10.0;

    let report_data = DesktopPlanReport {
        schema_version: 1,
        wayland_score,
        x11_score,
        graphics_readiness,
        phases: vec![
            PlanPhase {
                id: "P1".to_string(),
                name: "Userspace ABI Stabilization".to_string(),
                duration_weeks: "4-8".to_string(),
                goal: "Close remaining userspace ABI gaps before desktop shell integration".to_string(),
                success_criteria: vec![
                    "glibc closure gate passes in strict mode".to_string(),
                    "linux app compat strict profile stable in CI".to_string(),
                    "writable rootfs + session process tree reliability validated".to_string(),
                ],
            },
            PlanPhase {
                id: "P2".to_string(),
                name: "Display Server Foundation".to_string(),
                duration_weeks: "8-14".to_string(),
                goal: "Move from protocol parsing to real display server/compositor behavior".to_string(),
                success_criteria: vec![
                    "Wayland: wl_display + wl_registry + wl_compositor request handling".to_string(),
                    "X11: setup/auth handshake + core request dispatch + event/reply path".to_string(),
                    "Frame buffer handoff path validated with test client".to_string(),
                ],
            },
            PlanPhase {
                id: "P3".to_string(),
                name: "Desktop Environment Bring-up".to_string(),
                duration_weeks: "8-16".to_string(),
                goal: "Boot a usable shell session (XFCE first, GNOME later)".to_string(),
                success_criteria: vec![
                    "Display manager/session bootstrap script runs end-to-end".to_string(),
                    "XFCE session starts with terminal + panel + basic input".to_string(),
                    "GNOME deferred until D-Bus, logind, and rendering stack maturity".to_string(),
                ],
            },
        ],
        prerequisites: vec![
            PlanItem {
                area: "Kernel/userspace ABI".to_string(),
                item: "Maintain linux app strict gate green (syscall + signal + futex behavior)".to_string(),
                status: "in_progress".to_string(),
                owner_hint: "linux_compat + posix maintainers".to_string(),
            },
            PlanItem {
                area: "Graphics IPC".to_string(),
                item: "Implement real server-side object lifecycle for Wayland/X11 messages".to_string(),
                status: "todo".to_string(),
                owner_hint: "userspace_graphics".to_string(),
            },
            PlanItem {
                area: "Rendering".to_string(),
                item: "Define initial rendering backend (software framebuffer first, GPU later)".to_string(),
                status: "todo".to_string(),
                owner_hint: "drivers + userspace_graphics".to_string(),
            },
            PlanItem {
                area: "Session stack".to_string(),
                item: "Provide init + service supervision for desktop session dependencies".to_string(),
                status: "todo".to_string(),
                owner_hint: "runtime + packaging".to_string(),
            },
        ],
        immediate_backlog: vec![
            PlanItem {
                area: "Wayland".to_string(),
                item: "Add wl_registry advertisement + bind path smoke tests".to_string(),
                status: "todo".to_string(),
                owner_hint: "src/modules/userspace_graphics/wayland".to_string(),
            },
            PlanItem {
                area: "X11".to_string(),
                item: "Add minimal opcode dispatch table for core requests (CreateWindow/MapWindow)".to_string(),
                status: "todo".to_string(),
                owner_hint: "src/modules/userspace_graphics/x11".to_string(),
            },
            PlanItem {
                area: "XTASK".to_string(),
                item: "Add desktop smoke profile to linux-app-compat (x11/wayland probes)".to_string(),
                status: "todo".to_string(),
                owner_hint: "xtask validation".to_string(),
            },
            PlanItem {
                area: "Desktop strategy".to_string(),
                item: "Prioritize XFCE bring-up before GNOME to minimize systemd/logind hard dependency".to_string(),
                status: "planned".to_string(),
                owner_hint: "architecture".to_string(),
            },
        ],
        notes: vec![
            "Current codebase shows protocol parsing readiness, not full compositor/X server semantics".to_string(),
            "GNOME integration is heavier and typically needs mature D-Bus/session stack; XFCE is better first target".to_string(),
        ],
    };

    report::write_json_report(&out_dir.join("summary.json"), &report_data)?;
    fs::write(out_dir.join("summary.md"), render_markdown(&report_data))?;

    println!(
        "[linux-abi::desktop-plan] readiness={:.1} (wayland={:.1}, x11={:.1})",
        graphics_readiness, wayland_score, x11_score
    );
    println!(
        "[linux-abi::desktop-plan] wrote {}",
        out_dir.join("summary.json").display()
    );

    Ok(())
}

fn score_stack(root: &Path, stack_type: &str) -> f64 {
    let mut score: f64 = 0.0;
    let base = root.join("src/modules/userspace_graphics").join(stack_type);
    let mod_rs = base.join("mod.rs");
    let protocol_rs = base.join("protocol.rs");

    if mod_rs.exists() {
        score += 20.0;
    }

    let checks = if stack_type == "wayland" {
        vec![
            (&mod_rs, "protocol_socket_supported", 8.0),
            (&mod_rs, "shm_path_supported", 6.0),
            (&protocol_rs, "parse_wire_header", 8.0),
            (&mod_rs, "validate_client_handshake_prefix", 6.0),
            (&protocol_rs, "is_complete_frame", 6.0),
            (&mod_rs, "connect_sockaddr_precheck", 5.0),
        ]
    } else {
        vec![
            (&mod_rs, "unix_display_socket_supported", 8.0),
            (&protocol_rs, "parse_setup_prefix", 8.0),
            (&protocol_rs, "parse_request_prefix", 8.0),
            (&protocol_rs, "parse_server_packet_prefix", 8.0),
            (&mod_rs, "validate_client_setup_request", 6.0),
            (&mod_rs, "validate_server_reply_prefix", 6.0),
        ]
    };

    for (path, needle, points) in checks {
        if path.exists() {
            if let Ok(text) = fs::read_to_string(path) {
                if text.contains(needle) {
                    score += points;
                }
            }
        }
    }

    score.min(100.0)
}

fn render_markdown(report: &DesktopPlanReport) -> String {
    let mut md = String::new();
    md.push_str("# Linux Desktop Integration Plan\n\n");
    md.push_str(&format!(
        "- graphics_readiness: `{:.1}`\n- wayland_score: `{:.1}`\n- x11_score: `{:.1}`\n\n",
        report.graphics_readiness, report.wayland_score, report.x11_score
    ));

    md.push_str("## Phases\n\n");
    for phase in &report.phases {
        md.push_str(&format!(
            "### {} - {}\n- duration_weeks: `{}`\n- goal: {}\n",
            phase.id, phase.name, phase.duration_weeks, phase.goal
        ));
        md.push_str("- success_criteria:\n");
        for c in &phase.success_criteria {
            md.push_str(&format!("  - {}\n", c));
        }
        md.push('\n');
    }

    md.push_str("## Immediate Backlog\n\n");
    for item in &report.immediate_backlog {
        md.push_str(&format!(
            "- [{}] {} :: {} (owner: {})\n",
            item.status, item.area, item.item, item.owner_hint
        ));
    }

    md
}
