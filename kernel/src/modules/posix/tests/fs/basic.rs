use super::*;

#[test_case]
fn posix_fs_basic_file_flow() {
    let fs_id = fs::mount_ramfs("/posix").expect("mount");

    let fd = fs::open(fs_id, "/posix/demo.txt", true).expect("open create");
    let wrote = fs::write(fd, b"hello-posix").expect("write");
    assert_eq!(wrote, b"hello-posix".len());

    let _ = fs::lseek(fd, 0, fs::SeekWhence::Set).expect("seek start");
    let mut buf = [0u8; 16];
    let read = fs::read(fd, &mut buf).expect("read");
    assert_eq!(&buf[..read], b"hello-posix");

    let md = fs::stat(fs_id, "/posix/demo.txt").expect("stat");
    assert_eq!(md.size, b"hello-posix".len() as u64);

    fs::close(fd).expect("close");
    fs::unlink(fs_id, "/posix/demo.txt").expect("unlink");
    fs::unmount(fs_id).expect("unmount");
}

#[test_case]
fn posix_fs_directory_ops_work() {
    let fs_id = fs::mount_ramfs("/posix_dir_ops").expect("mount");
    fs::mkdir(fs_id, "/dir", 0o755).expect("mkdir");

    let fd = fs::open(fs_id, "/dir/a", true).expect("create");
    fs::write(fd, b"x").expect("write");
    fs::close(fd).expect("close");

    fs::rename(fs_id, "/dir/a", "/dir/b").expect("rename");
    assert!(fs::access(fs_id, "/dir/b").expect("access"));
    fs::unlink(fs_id, "/dir/b").expect("unlink");
    fs::rmdir(fs_id, "/dir").expect("rmdir");
    fs::unmount(fs_id).expect("unmount");
}
