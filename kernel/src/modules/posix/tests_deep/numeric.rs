use crate::modules::posix::PosixErrno;

#[test_case]
fn errno_numeric_codes_are_stable_deep() {
    assert_eq!(
        PosixErrno::Again.code(),
        crate::modules::posix_consts::errno::EAGAIN
    );
    assert_eq!(
        PosixErrno::BadFileDescriptor.code(),
        crate::modules::posix_consts::errno::EBADF
    );
    assert_eq!(
        PosixErrno::Invalid.code(),
        crate::modules::posix_consts::errno::EINVAL
    );
    assert_eq!(
        PosixErrno::NoSys.code(),
        crate::modules::posix_consts::errno::ENOSYS
    );
    assert_eq!(
        PosixErrno::from_code(crate::modules::posix_consts::errno::EEXIST),
        PosixErrno::AlreadyExists
    );
}
