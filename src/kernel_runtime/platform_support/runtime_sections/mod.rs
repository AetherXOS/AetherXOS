mod core_runtime;
mod vfs_runtime;

pub(crate) use self::core_runtime::log_runtime_sections;
#[cfg(feature = "vfs")]
pub(crate) use self::vfs_runtime::log_vfs_runtime_sections;
