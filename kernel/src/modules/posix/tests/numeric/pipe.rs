use super::*;
use crate::modules::posix::{pipe, fs, io, signal};

#[test_case]
#[cfg(feature = "posix_pipe")]
fn posix_pipe_roundtrip_and_nonblock_flow() {
    let (rfd, wfd) = pipe::pipe2(false).expect("pipe2");
    let wfd2 = pipe::dup(wfd).expect("dup writer");
    let wfd3 = pipe::dup2(wfd2, 60001).expect("dup2 writer");
    assert_eq!(wfd3, 60001);
    assert_eq!(pipe::pending_readable(rfd).expect("pending empty"), 0);
    assert_eq!(pipe::poll(rfd, crate::modules::posix_consts::net::POLLIN).expect("poll in empty"), 0);

    let wrote = pipe::write(wfd, b"pipe-data").expect("pipe write");
    assert_eq!(wrote, 9);
    assert_eq!(pipe::poll(rfd, crate::modules::posix_consts::net::POLLIN).expect("poll in ready"), crate::modules::posix_consts::net::POLLIN);
    assert_eq!(pipe::pending_readable(rfd).expect("pending non-empty"), 1);

    let mut out = [0u8; 16];
    let got = pipe::read(rfd, &mut out).expect("pipe read");
    assert_eq!(got, 9);
    assert_eq!(&out[..got], b"pipe-data");

    pipe::set_nonblock(rfd, true).expect("set nonblock");
    assert_eq!(pipe::read(rfd, &mut out), Err(super::PosixErrno::Again));

    pipe::close(wfd).expect("close writer");
    pipe::close(wfd2).expect("close dup writer");
    pipe::close(wfd3).expect("close dup2 writer");
    let eof = pipe::read(rfd, &mut out).expect("read eof");
    assert_eq!(eof, 0);
    pipe::close(rfd).expect("close reader");
}

#[test_case]
#[cfg(all(feature = "posix_pipe", feature = "posix_fs"))]
fn posix_pipe2_nonblock_is_visible_via_fcntl_flags() {
    let (rfd, wfd) = pipe::pipe2(true).expect("pipe2 nonblock");
    let expected = 0x2 | crate::modules::posix_consts::net::O_NONBLOCK as u32;
    assert_eq!(fs::fcntl_get_status_flags(rfd).expect("rfd flags"), expected);
    assert_eq!(fs::fcntl_get_status_flags(wfd).expect("wfd flags"), expected);

    let mut out = [0u8; 4];
    assert_eq!(pipe::read(rfd, &mut out), Err(super::PosixErrno::Again));

    pipe::close(wfd).expect("close writer");
    pipe::close(rfd).expect("close reader");
}

#[test_case]
#[cfg(all(feature = "posix_io", feature = "posix_fs"))]
fn posix_eventfd_nonblock_is_visible_via_fcntl_and_returns_again() {
    let fd = io::eventfd_create_errno(0, crate::modules::posix_consts::net::O_NONBLOCK)
        .expect("eventfd nonblock");
    let expected = 0x2 | crate::modules::posix_consts::net::O_NONBLOCK as u32;
    assert_eq!(fs::fcntl_get_status_flags(fd).expect("eventfd flags"), expected);

    let mut out = [0u8; 8];
    assert_eq!(fs::read(fd, &mut out), Err(super::PosixErrno::Again));
    fs::close(fd).expect("close eventfd");
}

#[test_case]
#[cfg(all(feature = "posix_signal", feature = "posix_fs", feature = "vfs"))]
fn posix_signalfd_nonblock_is_visible_via_fcntl_and_returns_again() {
    let fd = signal::signalfd_create_errno(0, crate::modules::posix_consts::net::O_NONBLOCK)
        .expect("signalfd nonblock");
    let expected = 0x2 | crate::modules::posix_consts::net::O_NONBLOCK as u32;
    assert_eq!(fs::fcntl_get_status_flags(fd).expect("signalfd flags"), expected);

    let mut out = [0u8; 128];
    assert_eq!(fs::read(fd, &mut out), Err(super::PosixErrno::Again));
    fs::close(fd).expect("close signalfd");
}

#[test_case]
#[cfg(all(feature = "posix_io", feature = "posix_pipe", feature = "posix_time"))]
fn posix_io_mixed_poll_and_select_work() {
    let (rfd, wfd) = pipe::pipe().expect("pipe");
    let wrote = pipe::write(wfd, b"x").expect("write one");
    assert_eq!(wrote, 1);

    let mut pfds = [io::PosixPollFd::new(rfd, crate::modules::posix_consts::net::POLLIN)];
    let ready = io::poll_mixed(&mut pfds, 0).expect("poll mixed");
    assert_eq!(ready, 1);
    assert_ne!(pfds[0].revents & crate::modules::posix_consts::net::POLLIN, 0);

    let mut pfds_ts = [io::PosixPollFd::new(rfd, crate::modules::posix_consts::net::POLLIN)];
    let ready_ts = io::poll_mixed_timespec(
        &mut pfds_ts,
        PosixTimespec { sec: 0, nsec: 1_000_000 },
    )
    .expect("poll mixed timespec");
    assert_eq!(ready_ts, 1);

    let sel = io::select_mixed(&[rfd], &[], &[], 0).expect("select mixed");
    assert_eq!(sel.readable.len(), 1);
    assert_eq!(sel.readable[0], rfd);

    let sel_ts = io::select_mixed_timespec(
        &[rfd],
        &[],
        &[],
        PosixTimespec { sec: 0, nsec: 1_000_000 },
    )
    .expect("select mixed timespec");
    assert_eq!(sel_ts.readable.len(), 1);
    assert_eq!(sel_ts.readable[0], rfd);

    let mut out = [0u8; 4];
    let _ = pipe::read(rfd, &mut out).expect("drain");
    pipe::close(wfd).expect("close wfd");
    pipe::close(rfd).expect("close rfd");
}
