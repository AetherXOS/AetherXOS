//! Tests for tmpfs filesystem.

#[cfg(test)]
mod tests {
    use super::super::filesystem::TmpFs;
    use crate::modules::vfs::types::FileSystem;
    use crate::interfaces::task::TaskId;

    #[test_case]
    fn open_follows_symlink_to_file() {
        let fs = TmpFs::new();
        let tid = TaskId(1);

        let mut file = fs.create("/target", tid).expect("create target");
        file.write(b"ubuntu").expect("write target");
        fs.symlink("/target", "/link", tid).expect("symlink");

        let mut handle = fs.open("/link", tid).expect("open link");
        let mut buf = [0u8; 6];
        assert_eq!(handle.read(&mut buf).expect("read link"), 6);
        assert_eq!(&buf, b"ubuntu");
    }

    #[test_case]
    fn open_rejects_symlink_loops() {
        let fs = TmpFs::new();
        let tid = TaskId(2);

        fs.symlink("/b", "/a", tid).expect("symlink a");
        fs.symlink("/a", "/b", tid).expect("symlink b");

        assert!(matches!(fs.open("/a", tid), Err("ELOOP")));
    }

    #[test_case]
    fn open_resolves_relative_symlink_targets() {
        let fs = TmpFs::new();
        let tid = TaskId(3);

        fs.mkdir("/dir", tid).expect("mkdir dir");
        let mut file = fs.create("/dir/target", tid).expect("create target");
        file.write(b"relpath").expect("write target");
        fs.symlink("target", "/dir/link", tid).expect("symlink");

        let mut handle = fs.open("/dir/link", tid).expect("open relative symlink");
        let mut buf = [0u8; 7];
        assert_eq!(handle.read(&mut buf).expect("read link"), 7);
        assert_eq!(&buf, b"relpath");
    }
}
