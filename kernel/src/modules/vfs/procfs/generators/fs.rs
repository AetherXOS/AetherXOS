use super::super::*;

pub fn generate_mounts() -> String {
    let mut result = String::new();
    result.push_str("devfs /dev devfs rw 0 0\n");
    result.push_str("proc /proc proc rw,nosuid,nodev,noexec 0 0\n");
    result.push_str("sysfs /sys sysfs rw,nosuid,nodev,noexec 0 0\n");
    result.push_str("tmpfs /tmp tmpfs rw 0 0\n");
    result.push_str("ramfs / ramfs rw 0 0\n");
    result
}

pub fn generate_filesystems() -> String {
    let mut result = String::new();
    result.push_str("nodev\tramfs\n");
    result.push_str("nodev\tdevfs\n");
    result.push_str("nodev\tprocfs\n");
    result.push_str("nodev\tsysfs\n");
    result.push_str("nodev\ttmpfs\n");
    #[cfg(feature = "vfs_ext4")]
    result.push_str("\text4\n");
    #[cfg(feature = "vfs_fatfs")]
    result.push_str("\tvfat\n");
    result
}
