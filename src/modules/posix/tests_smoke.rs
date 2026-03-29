use super::PosixErrno;

#[test_case]
fn posix_errno_codes_are_stable() {
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
}

#[test_case]
fn posix_errno_roundtrip_is_defined() {
    assert_eq!(
        PosixErrno::from_code(crate::modules::posix_consts::errno::EEXIST),
        PosixErrno::AlreadyExists
    );
    assert_eq!(
        PosixErrno::from_code(crate::modules::posix_consts::errno::EACCES),
        PosixErrno::PermissionDenied
    );
}
