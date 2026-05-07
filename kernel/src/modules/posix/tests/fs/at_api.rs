use super::*;

#[test_case]
fn posix_fs_at_and_time_apis_work() {
    let fs_id = fs::mount_ramfs("/posix_at").expect("mount");
    let fd = fs::openat(fs_id, "/", "at_f.txt", true).expect("openat create");
    fs::write(fd, b"hello").expect("write");

    assert!(fs::faccessat(fs_id, "/", "at_f.txt").expect("faccessat"));
    let st = fs::fstatat(fs_id, "/", "at_f.txt", true).expect("fstatat");
    assert_eq!(st.size, 5);

    fs::symlinkat(fs_id, "/at_f.txt", "/", "ln.txt").expect("symlinkat");
    let link_target = fs::readlinkat(fs_id, "/", "ln.txt").expect("readlinkat");
    assert_eq!(link_target, "/at_f.txt");
    fs::linkat(fs_id, "/", "at_f.txt", "/", "hard.txt").expect("linkat");

    let atime = PosixTimespec { sec: 123, nsec: 0 };
    let mtime = PosixTimespec { sec: 456, nsec: 0 };
    fs::utimes(fs_id, "/at_f.txt", atime, mtime).expect("utimes");
    fs::futimes(fd, atime, mtime).expect("futimes");
    fs::futimens(fd, atime, mtime).expect("futimens");
    fs::mkdirat(fs_id, "/", "sub", 0o755).expect("mkdirat");
    fs::rmdir(fs_id, "/sub").expect("rmdir sub");

    fs::close(fd).expect("close");
    fs::unlink(fs_id, "/hard.txt").expect("unlink hard");
    fs::unlink(fs_id, "/ln.txt").expect("unlink ln");
    fs::unlink(fs_id, "/at_f.txt").expect("unlink f");
    fs::unmount(fs_id).expect("unmount");
}

#[test_case]
fn posix_dup2_same_fd_still_validates_oldfd() {
    assert_eq!(fs::dup2(424242, 424242), Err(PosixErrno::BadFileDescriptor));
}
