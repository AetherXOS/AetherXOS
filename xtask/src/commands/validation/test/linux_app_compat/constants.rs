pub const SYSTEM_PKG_MANAGERS: &[&str] = &["apt-get", "dnf", "pacman", "apk", "zypper"];

pub const LANGUAGE_PKG_MANAGERS: &[&str] = &["pip", "pip3", "npm", "cargo"];

pub const DESKTOP_SESSION_BINARIES: &[&str] = &["xfce4-session", "gnome-shell"];

pub const WAYLAND_CLIENT_REQUIRED_PREFIXES: &[&str] = &[
    "validate_surface_commit_prefix",
    "validate_registry_bind_prefix",
    "validate_registry_advertisement_path",
];

pub const X11_CLIENT_REQUIRED_PREFIXES: &[&str] = &[
    "validate_client_request_prefix",
    "x11_reply_event_semantics_supported",
    "validate_core_opcode_dispatch_prefix",
];

pub const X11_PROTO_REQUIRED_PREFIXES: &[&str] = &[
    "parse_request_prefix",
    "parse_reply_prefix",
    "has_complete_server_packet",
];

pub const DISKFS_BOOTSTRAP_TELEMETRY_EVENTS: &[&str] = &[
    "event=tmpfs_fallback",
    "event=diskfs_mounted",
    "event=diskfs_mode_set",
];

pub const PIVOT_ROOT_SETUP_VARS: &[&str] = &[
    "pivot-root",
    "AETHERCORE_ENABLE_PIVOT_ROOT",
    "switch_root",
    "chroot",
    "pivot-root.status",
];

pub const LINUX_MOUNT_TYPES: &[&str] = &[
    "FsType", "Ext4", "Fat32", "Overlay", "Tmpfs", "Procfs", "Sysfs",
];

pub const SYSCALL_SEMANTIC_PARITY_TESTS: &[&str] = &[
    "signal_frame_parity",
    "af_unix_parity",
    "fs_backend_parity",
    "memory_mapping_parity",
    "ptrace_debugging_parity",
    "proc_sysctl_consistency_parity",
    "pid_uts_namespace_parity",
    "socket_options_parity",
];

pub const GPU_IOCTL_COVERAGE_REQS: &[&str] = &[
    "DRM_IOCTL_VERSION",
    "DRM_IOCTL_MODE_GETRESOURCES",
    "VIRTGPU",
];

pub const SCORE_WEIGHT_HOST: f64 = 0.25;
pub const SCORE_WEIGHT_INTEGRATION: f64 = 0.20;
pub const SCORE_WEIGHT_RUNTIME_PROBE: f64 = 0.10;
pub const SCORE_WEIGHT_KERNEL_GATE: f64 = 0.25;
pub const SCORE_WEIGHT_QEMU_GATE: f64 = 0.20;
