use super::*;
use crate::modules::posix::{mman, fs};

#[cfg(feature = "vfs")]
#[test_case]
fn posix_mman_file_mapping_management_works() {
    let fs_id = fs::mount_ramfs("/posix_mman").expect("mount");
    let fd = fs::open(fs_id, "/posix_mman/a.bin", true).expect("open create");
    fs::write(fd, b"abcdef").expect("write");
    fs::close(fd).expect("close");

    let map_id = mman::mmap_file(
        fs_id,
        "/posix_mman/a.bin",
        0,
        6,
        crate::modules::posix_consts::mman::PROT_READ | crate::modules::posix_consts::mman::PROT_WRITE,
        crate::modules::posix_consts::mman::MAP_SHARED,
    )
    .expect("mmap_file");

    assert_eq!(mman::get_flags(map_id).expect("flags"), crate::modules::posix_consts::mman::MAP_SHARED);
    assert!(mman::mincore(map_id).expect("mincore"));

    let mut buf = [0u8; 8];
    let rd = mman::mmap_read(map_id, &mut buf, 0).expect("mmap_read");
    assert_eq!(rd, 6);
    assert_eq!(&buf[..rd], b"abcdef");
    let wr = mman::mmap_write(map_id, b"XYZ", 0).expect("mmap_write");
    assert_eq!(wr, 3);

    mman::mprotect(map_id, crate::modules::posix_consts::mman::PROT_READ).expect("mprotect");
    assert_eq!(mman::get_prot(map_id).expect("prot"), crate::modules::posix_consts::mman::PROT_READ);
    assert!(mman::can_read(map_id).expect("can read"));
    assert!(!mman::can_write(map_id).expect("can write"));
    assert!(!mman::can_exec(map_id).expect("can exec"));
    assert_eq!(mman::mmap_write(map_id, b"Q", 0), Err(super::PosixErrno::PermissionDenied));

    mman::mlock(map_id).expect("mlock");
    assert!(mman::is_locked(map_id).expect("is_locked"));
    mman::munlock(map_id).expect("munlock");
    assert!(!mman::is_locked(map_id).expect("is_locked after"));

    mman::madvise(map_id, crate::modules::posix_consts::mman::MADV_SEQUENTIAL).expect("madvise");
    mman::msync_flags(map_id, crate::modules::posix_consts::mman::MS_SYNC).expect("msync flags");
    assert_eq!(mman::mapped_len(map_id).expect("mapped len"), 6);
    mman::mremap(map_id, 4).expect("mremap");
    assert_eq!(mman::mapped_len(map_id).expect("mapped len after"), 4);
    mman::msync_range(map_id, 0, 4).expect("msync_range");
    mman::msync(map_id).expect("msync");

    let anon_id = mman::mmap_anonymous(
        8,
        crate::modules::posix_consts::mman::PROT_READ | crate::modules::posix_consts::mman::PROT_WRITE,
        crate::modules::posix_consts::mman::MAP_PRIVATE,
    )
    .expect("mmap anonymous");
    assert_eq!(
        mman::get_flags(anon_id).expect("anon flags") & crate::modules::posix_consts::mman::MAP_ANONYMOUS,
        crate::modules::posix_consts::mman::MAP_ANONYMOUS
    );
    let wrote_anon = mman::mmap_write(anon_id, b"anon", 0).expect("anon write");
    assert_eq!(wrote_anon, 4);
    let mut anon_buf = [0u8; 8];
    let read_anon = mman::mmap_read(anon_id, &mut anon_buf, 0).expect("anon read");
    assert_eq!(read_anon, 8);
    assert_eq!(&anon_buf[..4], b"anon");

    mman::mlockall(
        crate::modules::posix_consts::mman::MCL_CURRENT
            | crate::modules::posix_consts::mman::MCL_FUTURE,
    )
    .expect("mlockall");
    assert_ne!(mman::mlockall_mode(), 0);
    assert!(mman::is_locked(anon_id).expect("anon locked"));
    mman::munlockall();
    assert_eq!(mman::mlockall_mode(), 0);
    assert!(!mman::is_locked(anon_id).expect("anon unlocked"));

    mman::munmap(anon_id).expect("munmap anon");
    mman::munmap(map_id).expect("munmap");

    fs::unlink(fs_id, "/posix_mman/a.bin").expect("unlink");
    fs::unmount(fs_id).expect("unmount");
}

#[test_case]
#[cfg(feature = "posix_fs")]
fn posix_fs_append_flag_forces_writes_to_end() {
    let fs_id = fs::mount_ramfs("/posix_append").expect("mount");
    let fd = fs::open(fs_id, "/posix_append/log.txt", true).expect("open create");
    fs::write(fd, b"abc").expect("write initial");
    fs::lseek(fd, 0, fs::SeekWhence::Set).expect("rewind");
    fs::fcntl_set_status_flags(fd, crate::modules::posix_consts::fs::O_APPEND as u32)
        .expect("set append");
    fs::write(fd, b"Z").expect("append write");
    fs::lseek(fd, 0, fs::SeekWhence::Set).expect("rewind read");
    let mut out = [0u8; 8];
    let n = fs::read(fd, &mut out).expect("read back");
    assert_eq!(&out[..n], b"abcZ");
    fs::fcntl_set_status_flags(
        fd,
        (crate::modules::posix_consts::fs::O_APPEND as u32) | 0xFFFF_0000,
    )
    .expect("set masked flags");
    assert_eq!(
        fs::fcntl_get_status_flags(fd).expect("get masked flags"),
        crate::modules::posix_consts::fs::O_APPEND as u32
    );
    fs::close(fd).expect("close");
    fs::unlink(fs_id, "/posix_append/log.txt").expect("unlink");
    fs::unmount(fs_id).expect("unmount");
}

#[test_case]
#[cfg(all(feature = "posix_fs", feature = "posix_pipe"))]
fn posix_fcntl_nonblock_updates_pipe_runtime_behavior() {
    use crate::modules::posix::pipe;
    let (rfd, wfd) = pipe::pipe2(false).expect("pipe2");
    assert_eq!(fs::fcntl_get_status_flags(rfd).expect("initial flags"), 0x2);

    fs::fcntl_set_status_flags(
        rfd,
        0x2 | crate::modules::posix_consts::net::O_NONBLOCK as u32,
    )
    .expect("set nonblock");
    assert_eq!(
        fs::fcntl_get_status_flags(rfd).expect("flags after nonblock"),
        0x2 | crate::modules::posix_consts::net::O_NONBLOCK as u32
    );

    let mut out = [0u8; 4];
    assert_eq!(pipe::read(rfd, &mut out), Err(super::PosixErrno::Again));

    fs::fcntl_set_status_flags(rfd, 0x2).expect("clear nonblock");
    assert_eq!(fs::fcntl_get_status_flags(rfd).expect("flags after clear"), 0x2);

    pipe::close(wfd).expect("close writer");
    let eof = pipe::read(rfd, &mut out).expect("read eof after clear");
    assert_eq!(eof, 0);
    pipe::close(rfd).expect("close reader");
}
