use super::*;

#[test_case]
fn errno_numeric_codes_are_stable() {
    assert_eq!(PosixErrno::Again.code(), crate::modules::posix_consts::errno::EAGAIN);
    assert_eq!(PosixErrno::BadFileDescriptor.code(), crate::modules::posix_consts::errno::EBADF);
    assert_eq!(PosixErrno::Invalid.code(), crate::modules::posix_consts::errno::EINVAL);
    assert_eq!(PosixErrno::NoSys.code(), crate::modules::posix_consts::errno::ENOSYS);
    assert_eq!(PosixErrno::from_code(crate::modules::posix_consts::errno::EEXIST), PosixErrno::AlreadyExists);
}

#[cfg(feature = "posix_fs")]
#[test_case]
fn seek_whence_numeric_values_are_posix_like() {
    assert_eq!(SeekWhence::Set.as_raw(), crate::modules::posix_consts::fs::SEEK_SET);
    assert_eq!(SeekWhence::Cur.as_raw(), crate::modules::posix_consts::fs::SEEK_CUR);
    assert_eq!(SeekWhence::End.as_raw(), crate::modules::posix_consts::fs::SEEK_END);
    assert_eq!(SeekWhence::from_raw(crate::modules::posix_consts::fs::SEEK_SET), Some(SeekWhence::Set));
    assert_eq!(SeekWhence::from_raw(crate::modules::posix_consts::fs::SEEK_CUR), Some(SeekWhence::Cur));
    assert_eq!(SeekWhence::from_raw(crate::modules::posix_consts::fs::SEEK_END), Some(SeekWhence::End));
    assert_eq!(SeekWhence::from_raw(77), None);
}
