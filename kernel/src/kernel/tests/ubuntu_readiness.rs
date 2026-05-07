#[cfg(test)]
mod tests {
    use crate::interfaces::TaskId;
    use crate::modules::vfs::{FileSystem, types::SeekFrom};
    use crate::modules::vfs::procfs::ProcFs;
    use alloc::string::String;
    use alloc::vec::Vec;

    #[test_case]
    fn test_procfs_self_exe_symlink() {
        let fs = ProcFs::new();
        let tid = TaskId(1);
        
        // 1. Check readdir includes 'exe'
        let entries = fs.readdir("/self", tid).expect("readdir /self failed");
        assert!(entries.iter().any(|e| e.name == "exe"), "exe entry missing in /self");

        // 2. Check stat reports symlink mode
        let stat = fs.stat("/self/exe", tid).expect("stat /self/exe failed");
        assert_eq!(stat.mode & 0o170000, 0o120000, "expected symlink mode for /self/exe");

        // 3. Check readlink returns something (even if empty/fallback)
        let link = fs.readlink("/self/exe", tid).expect("readlink /self/exe failed");
        assert!(!link.is_empty(), "readlink returned empty string");
    }

    #[test_case]
    fn test_procfs_pid_exe_symlink() {
        let fs = ProcFs::new();
        let tid = TaskId(1);
        
        // Use PID 1 (should exist as it's the init process usually)
        let entries = fs.readdir("/1", tid).expect("readdir /1 failed");
        assert!(entries.iter().any(|e| e.name == "exe"), "exe entry missing in /1");

        let stat = fs.stat("/1/exe", tid).expect("stat /1/exe failed");
        assert_eq!(stat.mode & 0o170000, 0o120000, "expected symlink mode for /1/exe");

        let link = fs.readlink("/1/exe", tid).expect("readlink /1/exe failed");
        assert!(!link.is_empty(), "readlink /1/exe returned empty string");
    }
}
