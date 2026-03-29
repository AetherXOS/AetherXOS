use crate::modules::posix::process;

#[test_case]
fn deep_process_identity_basics() {
    assert_eq!(process::getuid(), 0);
    assert_eq!(process::geteuid(), 0);
    assert_eq!(process::getgid(), 0);
    assert_eq!(process::getegid(), 0);
    assert!(process::getpid() > 0);
    assert!(process::gettid() > 0);
}
