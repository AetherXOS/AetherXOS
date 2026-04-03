use crate::utils::process;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HostPlatform {
    Windows,
    Macos,
    LinuxApt,
    LinuxPacman,
    LinuxUnknown,
    Other,
}

pub(crate) struct ProvisionPlan {
    pub(crate) tool: &'static str,
    pub(crate) windows: Option<&'static [&'static str]>,
    pub(crate) macos: Option<&'static [&'static str]>,
    pub(crate) linux_apt: Option<&'static [&'static str]>,
    pub(crate) linux_pacman: Option<&'static [&'static str]>,
}

pub(crate) fn detect_platform() -> HostPlatform {
    if cfg!(windows) {
        return HostPlatform::Windows;
    }
    if cfg!(target_os = "macos") {
        return HostPlatform::Macos;
    }
    if cfg!(target_os = "linux") {
        if process::which("apt-get") {
            return HostPlatform::LinuxApt;
        }
        if process::which("pacman") {
            return HostPlatform::LinuxPacman;
        }
        return HostPlatform::LinuxUnknown;
    }
    HostPlatform::Other
}

pub(crate) fn run_provision_plan(plan: &ProvisionPlan, platform: HostPlatform) {
    match platform {
        HostPlatform::Windows => {
            if let Some(args) = plan.windows {
                let _ = process::run_best_effort(args[0], &args[1..]);
            }
        }
        HostPlatform::Macos => {
            if let Some(args) = plan.macos {
                let _ = process::run_best_effort(args[0], &args[1..]);
            }
        }
        HostPlatform::LinuxApt => {
            if let Some(args) = plan.linux_apt {
                let _ = process::run_best_effort("sudo", args);
            }
        }
        HostPlatform::LinuxPacman => {
            if let Some(args) = plan.linux_pacman {
                let _ = process::run_best_effort("sudo", args);
            }
        }
        HostPlatform::LinuxUnknown | HostPlatform::Other => {}
    }
}

pub(crate) fn ensure_tool_with_plan(
    binaries: &[&str],
    missing_message: &str,
    plan: &ProvisionPlan,
    platform: HostPlatform,
    windows_preflight: Option<(&str, &str)>,
) {
    if process::which_any(binaries) {
        return;
    }

    println!("{}", missing_message);
    println!(
        "[setup::provision] Applying provisioning plan for {}.",
        plan.tool
    );

    if platform == HostPlatform::Windows {
        if let Some((binary, warning_message)) = windows_preflight {
            if !process::which(binary) {
                println!("{}", warning_message);
                return;
            }
        }
    }

    run_provision_plan(plan, platform);
}
