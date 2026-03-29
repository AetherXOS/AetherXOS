use super::super::*;
mod process_runtime;
mod trace_seccomp;
#[path = "handlers/vectored_io.rs"]
mod vectored_io;
#[path = "handlers/privileged_admin.rs"]
mod privileged_admin;
#[path = "handlers/fd_async_sandbox.rs"]
mod fd_async_sandbox;
#[path = "handlers/misc_kernel_apis.rs"]
mod misc_kernel_apis;
#[path = "handlers/capability_and_sysctl.rs"]
mod capability_and_sysctl;
#[path = "handlers/linux_abi.rs"]
mod linux_abi;
#[path = "handlers/runtime_state.rs"]
mod runtime_state;
use linux_abi::*;
use runtime_state::*;
pub use process_runtime::{sys_linux_execveat, sys_linux_rseq};
pub use trace_seccomp::{sys_linux_ptrace, sys_linux_seccomp};

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct LinuxCapUserHeader {
    version: u32,
    pid: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct LinuxCapUserData {
    effective: u32,
    permitted: u32,
    inheritable: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct LinuxCloneArgs {
    flags: u64,
    pidfd: u64,
    child_tid: u64,
    parent_tid: u64,
    exit_signal: u64,
    stack: u64,
    stack_size: u64,
    tls: u64,
    set_tid: u64,
    set_tid_size: u64,
    cgroup: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct HyperCompatSysctlArgs {
    key_ptr: u64,
    key_len: u64,
    value_ptr: u64,
    value_len: u64,
    out_ptr: u64,
    out_len: u64,
    flags: u64,
    reserved: u64,
}

#[inline(always)]
fn require_control_plane_access(resource: u64) -> Result<(), usize> {
    if crate::modules::security::check_control_plane_access(resource) {
        Ok(())
    } else {
        Err(linux_eperm())
    }
}

pub(crate) use capability_and_sysctl::{sys_linux_capget, sys_linux_capset};
pub use capability_and_sysctl::{
    sys_linux_rt_sigqueueinfo, sys_linux_sysctl, sys_linux_sysfs,
};
pub use fd_async_sandbox::{
    sys_linux_cachestat, sys_linux_io_uring_enter, sys_linux_io_uring_register,
    sys_linux_io_uring_setup, sys_linux_landlock_add_rule, sys_linux_landlock_create_ruleset,
    sys_linux_landlock_restrict_self, sys_linux_memfd_secret, sys_linux_membarrier,
    sys_linux_pidfd_getfd, sys_linux_pidfd_open, sys_linux_pidfd_send_signal,
    sys_linux_process_madvise, sys_linux_process_mrelease, sys_linux_quotactl_fd,
    sys_linux_userfaultfd,
};
pub use misc_kernel_apis::{
    sys_linux_bpf, sys_linux_io_pgetevents, sys_linux_kexec_file_load, sys_linux_memfd_create,
    sys_linux_open_by_handle_at, sys_linux_pkey_alloc, sys_linux_pkey_free,
    sys_linux_pkey_mprotect, sys_linux_timer_create, sys_linux_timer_delete,
};
pub(crate) use misc_kernel_apis::sys_linux_clone3;
pub use privileged_admin::{
    sys_linux_acct, sys_linux_create_module, sys_linux_delete_module, sys_linux_init_module,
    sys_linux_ioperm, sys_linux_iopl, sys_linux_reboot, sys_linux_security, sys_linux_vhangup,
};
pub use vectored_io::{
    sys_linux_copy_file_range, sys_linux_preadv, sys_linux_preadv2, sys_linux_pwritev,
    sys_linux_pwritev2,
};

pub fn sys_linux_mlock2(addr: UserPtr<u8>, len: usize, flags: usize) -> usize {
    if (flags & !MLOCK_ONFAULT_FLAG) != 0 {
        return linux_inval();
    }
    sys_linux_mlock(addr, len)
}

pub fn sys_linux_fsconfig(
    fd: Fd,
    cmd: usize,
    key: UserPtr<u8>,
    value: UserPtr<u8>,
    aux: usize,
) -> usize {
    crate::modules::linux_compat::fs::mount::sys_linux_fsconfig_apply(fd, cmd, key, value, aux)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn reboot_validates_magic_values() {
        assert_eq!(sys_linux_reboot(0, 0, REBOOT_CMD_RESTART, 0), linux_inval());
        assert_eq!(
            sys_linux_reboot(REBOOT_MAGIC1, REBOOT_MAGIC2_A, REBOOT_CMD_RESTART, 0),
            0
        );
    }

    #[test_case]
    fn iopl_rejects_out_of_range_level() {
        assert_eq!(sys_linux_iopl(IOPL_MAX_LEVEL + 1), linux_inval());
        assert_eq!(sys_linux_iopl(IOPL_MAX_LEVEL), 0);
    }

    #[test_case]
    fn ioperm_validates_arguments() {
        assert_eq!(sys_linux_ioperm(0, 0, 1), linux_inval());
        assert_eq!(sys_linux_ioperm(0, 4, 2), linux_inval());
        assert_eq!(sys_linux_ioperm(0x10, 8, 1), 0);
        assert_eq!(sys_linux_ioperm(0x10, 8, 0), 0);
    }

    #[test_case]
    fn security_and_vhangup_have_defined_behavior() {
        assert_eq!(sys_linux_security(0, 0, 0, 0), 0);
        assert_eq!(sys_linux_security(1, 0, 0, 0), linux_inval());
        assert_eq!(sys_linux_vhangup(), 0);
    }

    #[test_case]
    fn create_module_and_bpf_validate_inputs() {
        assert_eq!(sys_linux_create_module(UserPtr::new(0), 1), linux_inval());
        assert_eq!(sys_linux_create_module(UserPtr::new(1), 0), linux_inval());
        assert_eq!(
            sys_linux_bpf(BPF_CMD_MAP_CREATE, UserPtr::new(0), 16),
            linux_inval()
        );
        assert_eq!(
            sys_linux_bpf(BPF_CMD_MAP_CREATE, UserPtr::new(1), 0),
            linux_inval()
        );
    }

    #[test_case]
    fn kexec_and_open_by_handle_validate_inputs() {
        assert_eq!(
            sys_linux_kexec_file_load(Fd(1), Fd(2), 0, UserPtr::new(0), 1),
            linux_inval()
        );
        assert_eq!(
            sys_linux_kexec_file_load(Fd(1), Fd(2), 8, UserPtr::new(0), 0),
            linux_fault()
        );
        assert_eq!(
            sys_linux_open_by_handle_at(Fd(1), UserPtr::new(0), 0),
            linux_fault()
        );
    }

    #[test_case]
    fn memfd_create_sets_linux_cloexec_flag() {
        use crate::kernel::syscalls::syscalls_consts::linux::memfd_flags::MFD_CLOEXEC;

        let name = b"memfd-cloexec\0";
        let fd = sys_linux_memfd_create(UserPtr::new(name.as_ptr() as usize), MFD_CLOEXEC) as u32;
        assert_eq!(
            crate::modules::linux_compat::fs::io::linux_fd_get_descriptor_flags(fd)
                & crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
            crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC
        );
    }

    #[test_case]
    fn sysfs_validates_option_and_pointers() {
        assert_eq!(sys_linux_sysfs(0, 0, 0), linux_inval());
        assert_eq!(sys_linux_sysfs(1, 1, 0), linux_inval());
        assert_eq!(
            sys_linux_sysfs(SYSFS_OPTION_2_FILESYSTEM_TYPE_NAME, 0, 0),
            linux_fault()
        );
    }

    #[test_case]
    fn privileged_paths_respect_security_denials() {
        use crate::interfaces::security::SecurityLevel;
        use crate::modules::security::{MacLabel, RESOURCE_MODULE_LOAD};

        crate::modules::security::mac::set_resource_security_level(
            RESOURCE_MODULE_LOAD,
            SecurityLevel::TopSecret,
        );
        crate::modules::security::set_mac_subject_clearance(MacLabel::Confidential);

        assert_eq!(
            sys_linux_bpf(BPF_CMD_MAP_CREATE, UserPtr::new(1), 16),
            linux_eperm()
        );

        crate::modules::security::set_mac_subject_clearance(MacLabel::TopSecret);
        crate::modules::security::mac::set_resource_security_level(
            RESOURCE_MODULE_LOAD,
            SecurityLevel::Unclassified,
        );
    }

    #[test_case]
    fn sysctl_rejects_null_argument_pointer() {
        assert_eq!(sys_linux_sysctl(UserPtr::new(0)), linux_fault());
    }

    #[test_case]
    fn sysctl_respects_surface_policy_gate() {
        crate::config::KernelConfig::set_sysctl_api_exposed(Some(false));

        let res = sys_linux_sysctl(UserPtr::new(1));
        assert_eq!(res, linux_eperm());

        crate::config::KernelConfig::set_sysctl_api_exposed(None);
    }

    #[test_case]
    fn sysctl_validates_flag_mask_and_rejects_zero_flags() {
        crate::config::KernelConfig::set_sysctl_api_exposed(Some(true));

        let zero_flags = HyperCompatSysctlArgs {
            key_ptr: 0,
            key_len: 0,
            value_ptr: 0,
            value_len: 0,
            out_ptr: 0,
            out_len: 0,
            flags: 0,
            reserved: 0,
        };
        assert_eq!(
            sys_linux_sysctl(UserPtr::new((&zero_flags as *const HyperCompatSysctlArgs) as usize)),
            linux_inval()
        );

        let unknown_flag = HyperCompatSysctlArgs {
            flags: (HC_SYSCTL_FLAG_READ | (1 << 9)) as u64,
            ..zero_flags
        };
        assert_eq!(
            sys_linux_sysctl(UserPtr::new((&unknown_flag as *const HyperCompatSysctlArgs) as usize)),
            linux_inval()
        );

        crate::config::KernelConfig::set_sysctl_api_exposed(None);
    }

    #[test_case]
    fn sysctl_read_requires_output_pointer_and_length() {
        crate::config::KernelConfig::set_sysctl_api_exposed(Some(true));

        let key = b"sysctl_api_exposed\0";
        let args = HyperCompatSysctlArgs {
            key_ptr: key.as_ptr() as u64,
            key_len: key.len() as u64,
            value_ptr: 0,
            value_len: 0,
            out_ptr: 0,
            out_len: 16,
            flags: HC_SYSCTL_FLAG_READ as u64,
            reserved: 0,
        };
        assert_eq!(
            sys_linux_sysctl(UserPtr::new((&args as *const HyperCompatSysctlArgs) as usize)),
            linux_fault()
        );

        crate::config::KernelConfig::set_sysctl_api_exposed(None);
    }

    #[test_case]
    fn sysctl_read_returns_rendered_config_value() {
        crate::config::KernelConfig::set_sysctl_api_exposed(Some(true));

        let key = b"sysctl_api_exposed\0";
        let mut out = [0u8; 32];
        let args = HyperCompatSysctlArgs {
            key_ptr: key.as_ptr() as u64,
            key_len: key.len() as u64,
            value_ptr: 0,
            value_len: 0,
            out_ptr: out.as_mut_ptr() as u64,
            out_len: out.len() as u64,
            flags: HC_SYSCTL_FLAG_READ as u64,
            reserved: 0,
        };

        let copied = sys_linux_sysctl(UserPtr::new((&args as *const HyperCompatSysctlArgs) as usize));
        assert!(copied > 0 && copied <= out.len(), "sysctl read should copy a non-empty payload");

        let rendered = core::str::from_utf8(&out[..copied]).expect("sysctl payload should be utf8");
        assert!(
            rendered == "true\n" || rendered == "false\n",
            "sysctl key should render canonical bool line"
        );

        crate::config::KernelConfig::set_sysctl_api_exposed(None);
    }
}
