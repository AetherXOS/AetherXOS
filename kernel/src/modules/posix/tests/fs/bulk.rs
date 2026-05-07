use super::*;

#[test_case]
fn posix_fs_bulk_new_apis_work() {
    let fs_id = fs::mount_ramfs("/posix_bulk").expect("mount");
    let fd = fs::open(fs_id, "/bulk_a.txt", true).expect("open create");
    let _ = fs::writev(fd, &[b"ab", b"cd"]).expect("writev");

    let mut pbuf = [0u8; 2];
    let pread_n = fs::pread(fd, &mut pbuf, 1).expect("pread");
    assert_eq!(pread_n, 2);
    assert_eq!(&pbuf, b"bc");

    let pwrite_n = fs::pwrite(fd, b"ZZ", 2).expect("pwrite");
    assert_eq!(pwrite_n, 2);

    let pwritev_n = fs::pwritev(fd, &[b"12", b"34"], 0).expect("pwritev");
    assert_eq!(pwritev_n, 4);

    let mut pv1 = [0u8; 2];
    let mut pv2 = [0u8; 2];
    let mut piov = [&mut pv1[..], &mut pv2[..]];
    let preadv_n = fs::preadv(fd, &mut piov, 0).expect("preadv");
    assert_eq!(preadv_n, 4);
    assert_eq!(&pv1, b"12");
    assert_eq!(&pv2, b"34");

    let _ = fs::lseek(fd, 0, fs::SeekWhence::Set).expect("seek start");
    let mut r1 = [0u8; 2];
    let mut r2 = [0u8; 3];
    let mut iov = [&mut r1[..], &mut r2[..]];
    let rv = fs::readv(fd, &mut iov).expect("readv");
    assert_eq!(rv, 4);
    assert_eq!(&r1, b"ab");
    assert_eq!(&r2[..2], b"ZZ");

    let st = fs::fstat(fd).expect("fstat");
    assert_eq!(st.size, 4);
    fs::fdatasync(fd).expect("fdatasync");

    let fd2 = fs::dup(fd).expect("dup");
    let fd3 = fs::dup2(fd, 60000).expect("dup2");
    assert_eq!(fd3, 60000);

    let lst = fs::lstat(fs_id, "/bulk_a.txt").expect("lstat");
    assert_eq!(lst.mode, 0o644);
    assert_eq!(lst.uid, 0);
    assert_eq!(lst.gid, 0);

    fs::fchmod(fd2, 0o640).expect("fchmod");
    fs::fchown(fd2, 2000, 3000).expect("fchown");
    let lst_after = fs::lstat(fs_id, "/bulk_a.txt").expect("lstat after fchmod/fchown");
    assert_eq!(lst_after.mode, 0o640);
    assert_eq!(lst_after.uid, 2000);
    assert_eq!(lst_after.gid, 3000);
    fs::chdir(fs_id, "/").expect("chdir");
    assert_eq!(fs::getcwd(fs_id).expect("getcwd"), "/");
    assert_eq!(fs::umask(0o027), 0o022);

    let copied = fs::copy_file_range(fs_id, "/bulk_a.txt", "/bulk_b.txt").expect("copy_file_range");
    assert_eq!(copied, 4);
    let rp = fs::realpath(fs_id, "/bulk_b.txt").expect("realpath");
    assert_eq!(rp, "/bulk_b.txt");
    fs::posix_fallocate(fd2, 16).expect("posix_fallocate");
    fs::fallocate(fd2, 0, 20, 4).expect("fallocate with offset");
    let grown = fs::fstat(fd2).expect("fstat grown");
    assert!(grown.size >= 24);
    fs::fallocate(
        fd2,
        crate::modules::posix_consts::fs::FALLOC_FL_KEEP_SIZE,
        30,
        8,
    )
    .expect("fallocate keep size");
    let same = fs::fstat(fd2).expect("fstat keep size");
    assert_eq!(same.size, grown.size);
    fs::fallocate(
        fd2,
        crate::modules::posix_consts::fs::FALLOC_FL_KEEP_SIZE
            | crate::modules::posix_consts::fs::FALLOC_FL_PUNCH_HOLE,
        0,
        2,
    )
    .expect("fallocate punch hole");
    fs::lseek(fd2, 0, fs::SeekWhence::Set).expect("lseek start");
    let mut punched = [0xFFu8; 2];
    let punched_n = fs::read(fd2, &mut punched).expect("read punched");
    assert_eq!(punched_n, 2);
    assert_eq!(punched, [0u8; 2]);

    let entries = fs::scandir(fs_id, "/").expect("scandir");
    assert!(!entries.is_empty());

    fs::renameat(fs_id, "/", "bulk_b.txt", "/", "bulk_c.txt").expect("renameat");
    fs::unlinkat(fs_id, "/", "bulk_c.txt").expect("unlinkat");

    fs::close(fd3).expect("close fd3");
    fs::close(fd2).expect("close fd2");
    fs::close(fd).expect("close fd");
    fs::unlink(fs_id, "/bulk_a.txt").expect("unlink a");
    fs::unmount(fs_id).expect("unmount");
}
