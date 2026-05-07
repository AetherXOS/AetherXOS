use super::*;

#[test_case]
fn posix_fs_extended_apis_work() {
    let fs_id = fs::mount_ramfs("/posix_ext").expect("mount");

    let fd = fs::creat(fs_id, "/ext_file.txt", 0o644).expect("creat");
    fs::write(fd, b"abcdef").expect("write");

    fs::ftruncate(fd, 3).expect("ftruncate");
    let _ = fs::lseek(fd, 0, fs::SeekWhence::Set).expect("seek");
    let mut buf = [0u8; 8];
    let read = fs::read(fd, &mut buf).expect("read");
    assert_eq!(&buf[..read], b"abc");

    let md = fs::stat(fs_id, "/ext_file.txt").expect("stat");
    assert_eq!(md.mode, 0o644);
    assert_eq!(md.uid, 0);
    assert_eq!(md.gid, 0);

    fs::chmod(fs_id, "/ext_file.txt", 0o600).expect("chmod");
    fs::chown(fs_id, "/ext_file.txt", 1000, 1000).expect("chown");
    let md_after = fs::stat(fs_id, "/ext_file.txt").expect("stat after chmod/chown");
    assert_eq!(md_after.mode, 0o600);
    assert_eq!(md_after.uid, 1000);
    assert_eq!(md_after.gid, 1000);
    fs::link(fs_id, "/ext_file.txt", "/hard.txt").expect("link");
    let hard_target = fs::stat(fs_id, "/hard.txt").expect("stat hard");
    assert_eq!(hard_target.size, md_after.size);
    fs::symlink(fs_id, "/ext_file.txt", "/sym.txt").expect("symlink");
    let target = fs::readlink(fs_id, "/sym.txt").expect("readlink");
    assert_eq!(target, "/ext_file.txt");
    let sym_lstat = fs::lstat(fs_id, "/sym.txt").expect("lstat symlink");
    assert!(sym_lstat.is_symlink);
    assert_eq!(sym_lstat.size, "/ext_file.txt".len() as u64);
    fs::utimensat(fs_id, "/ext_file.txt").expect("utimensat");

    let map_id = fs::mmap(fs_id, "/ext_file.txt", 0, 3, true).expect("mmap");
    let mut mapped = [0u8; 3];
    let map_read = fs::mmap_read(map_id, &mut mapped, 0).expect("mmap_read");
    assert_eq!(map_read, 3);
    assert_eq!(&mapped, b"abc");

    let map_write = fs::mmap_write(map_id, b"XYZ", 0).expect("mmap_write");
    assert_eq!(map_write, 3);
    fs::fdatasync(fd).expect("fdatasync");
    fs::msync(map_id).expect("msync");
    fs::munmap(map_id).expect("munmap");

    let _ = fs::lseek(fd, 0, fs::SeekWhence::Set).expect("seek after msync");
    let mut verify = [0u8; 4];
    let verify_read = fs::read(fd, &mut verify).expect("read verify");
    assert_eq!(verify_read, 3);
    assert_eq!(&verify[..3], b"XYZ");

    fs::close(fd).expect("close");
    fs::unlink(fs_id, "/hard.txt").expect("unlink hard");
    fs::unlink(fs_id, "/sym.txt").expect("unlink symlink");
    fs::unlink(fs_id, "/ext_file.txt").expect("unlink file");
    fs::unmount(fs_id).expect("unmount");
}
