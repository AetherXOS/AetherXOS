use super::super::*;
use crate::modules::linux_compat::linux::open_flags;

// Linux UAPI: _LINUX_CAPABILITY_VERSION_3 from linux/capability.h.
pub(super) const LINUX_CAP_VERSION_3: u32 = 0x2008_0522;

// Linux UAPI: MEMBARRIER_CMD_PRIVATE_EXPEDITED bit position.
pub(super) const MEMBARRIER_PRIVATE_EXPEDITED_BIT: usize = 3;

pub(super) const MLOCK_ONFAULT_FLAG: usize = 0x01;
pub(super) const SYSFS_OPTION_2_FILESYSTEM_TYPE_NAME: usize = 2;
pub(super) const SYSFS_OPTION_3_FILESYSTEM_INDEX_BY_NAME: usize = 3;
pub(super) const SYSFS_NODEV_FS_NAME: &str = "nodev";

// Linux reboot(2) ABI guards from linux/reboot.h.
pub(super) const REBOOT_MAGIC1: usize = 0xFEE1_DEAD;
pub(super) const REBOOT_MAGIC2_A: usize = 672_274_793;
pub(super) const REBOOT_MAGIC2_B: usize = 850_722_78;
pub(super) const REBOOT_MAGIC2_C: usize = 369_367_448;

// Linux reboot(2) command values from linux/reboot.h.
pub(super) const REBOOT_CMD_RESTART: usize = 0x0123_4567;
pub(super) const REBOOT_CMD_HALT: usize = 0xCDEF_0123;
pub(super) const REBOOT_CMD_POWER_OFF: usize = 0x4321_FEDC;
pub(super) const REBOOT_CMD_RESTART2: usize = 0xA1B2_C3D4;

pub(super) const IOPL_MAX_LEVEL: usize = 3;
pub(super) const BPF_CMD_MAP_CREATE: usize = 0;
pub(super) const BPF_MAP_FD_BASE: usize = 500_000;
pub(super) const OPEN_BY_HANDLE_ALLOWED_FLAGS: usize = open_flags::O_CLOEXEC;

pub(super) const HC_SYSCTL_FLAG_READ: usize = 1 << 0;
pub(super) const HC_SYSCTL_FLAG_WRITE: usize = 1 << 1;
pub(super) const HC_SYSCTL_FLAG_PATH: usize = 1 << 2;
