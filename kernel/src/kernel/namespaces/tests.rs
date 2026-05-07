use super::*;

#[test_case]
fn test_unshare_zero_flags_is_noop() {
    ensure_namespace_set(0);
    let id = unshare_process_namespaces(0, 0).expect("zero flags must succeed");
    assert_eq!(id, 0, "zero flags must return the same namespace id");
}

#[test_case]
fn test_unshare_pid_allocates_unique_ids() {
    ensure_namespace_set(0);
    let id1 = unshare_process_namespaces(0, CloneFlags::CLONE_NEWPID.bits())
        .expect("CLONE_NEWPID unshare must succeed");
    let id2 = unshare_process_namespaces(0, CloneFlags::CLONE_NEWPID.bits())
        .expect("second CLONE_NEWPID unshare must succeed");
    assert_ne!(id1, id2, "each unshare must produce a distinct id");
    assert_ne!(id1, 0, "new id must differ from root");
}

#[test_case]
fn test_unshare_unknown_flag_returns_error() {
    let unknown = 0x0000_0001u32; // not a valid CLONE_NEW* bit
    let result = unshare_process_namespaces(0, unknown);
    assert!(
        result.is_err(),
        "unknown flags must be rejected with EINVAL"
    );
}

#[test_case]
fn test_nsfd_setns_roundtrip() {
    ensure_namespace_set(0);
    let ns_id = unshare_process_namespaces(0, CloneFlags::CLONE_NEWNET.bits())
        .expect("CLONE_NEWNET unshare must succeed");
    let fd = nsfd_open(ns_id);
    let result = setns_process_namespaces(0, fd, 0);
    assert!(result.is_ok(), "setns via valid nsfd must succeed");
    nsfd_close(fd);
    // After close, the same fd must fail
    let result2 = setns_process_namespaces(0, fd, 0);
    assert!(result2.is_err(), "setns via closed fd must fail with EBADF");
}

#[test_case]
fn test_setns_invalid_fd_returns_ebadf() {
    let result = setns_process_namespaces(0, -1, 0);
    assert!(result.is_err(), "invalid fd -1 must return EBADF");
}

#[test_case]
fn test_setns_invalid_nstype_returns_einval() {
    ensure_namespace_set(0);
    let ns_id = unshare_process_namespaces(0, CloneFlags::CLONE_NEWUTS.bits())
        .expect("CLONE_NEWUTS unshare must succeed");
    let fd = nsfd_open(ns_id);
    let result = setns_process_namespaces(0, fd, 0x1);
    assert!(result.is_err(), "unknown nstype must return EINVAL");
    nsfd_close(fd);
}
