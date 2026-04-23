use super::LinuxAppCompatOptions;

#[derive(Debug, Clone, Copy)]
pub(super) struct NormalizedOptions {
    pub(super) desktop_smoke: bool,
    pub(super) quick: bool,
    pub(super) qemu: bool,
    pub(super) strict: bool,
    pub(super) ci: bool,
    pub(super) require_busybox: bool,
    pub(super) require_glibc: bool,
    pub(super) require_wayland: bool,
    pub(super) require_x11: bool,
    pub(super) require_fs_stack: bool,
    pub(super) require_package_stack: bool,
    pub(super) require_desktop_app_stack: bool,
}

impl NormalizedOptions {
    pub(super) fn from_raw(raw: LinuxAppCompatOptions) -> Self {
        let desktop_smoke = raw.desktop_smoke;
        Self {
            desktop_smoke,
            quick: raw.quick,
            qemu: raw.qemu,
            strict: raw.strict,
            ci: raw.ci,
            require_busybox: raw.require_busybox,
            require_glibc: raw.require_glibc,
            require_wayland: raw.require_wayland || desktop_smoke,
            require_x11: raw.require_x11 || desktop_smoke,
            require_fs_stack: raw.require_fs_stack,
            require_package_stack: raw.require_package_stack,
            require_desktop_app_stack: raw.require_desktop_app_stack || desktop_smoke,
        }
    }

    pub(super) fn score_profile(self) -> &'static str {
        if self.strict { "strict" } else { "standard" }
    }

    pub(super) fn needs_qemu_gate(self) -> bool {
        self.qemu || self.strict
    }
}
