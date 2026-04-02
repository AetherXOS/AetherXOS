use super::super::PosixErrno;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    static ref ENV_TABLE: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());
    static ref HOSTNAME: Mutex<String> = Mutex::new(String::from("aethercore"));
    static ref DOMAINNAME: Mutex<String> = Mutex::new(String::from("localdomain"));
    static ref GROUP_MEMBERSHIP: Mutex<Vec<u32>> = Mutex::new(Vec::new());
}

static REAL_UID: AtomicU32 = AtomicU32::new(0);
static EFFECTIVE_UID: AtomicU32 = AtomicU32::new(0);
static SAVED_UID: AtomicU32 = AtomicU32::new(0);
static REAL_GID: AtomicU32 = AtomicU32::new(0);
static EFFECTIVE_GID: AtomicU32 = AtomicU32::new(0);
static SAVED_GID: AtomicU32 = AtomicU32::new(0);
static UMASK_BITS: AtomicU32 = AtomicU32::new(0o022);
static PERSONALITY: AtomicU32 = AtomicU32::new(0);

#[inline(always)]
fn credentials_enforced() -> bool {
    crate::config::KernelConfig::credential_enforcement_enabled()
}

#[inline(always)]
fn multi_user_enabled() -> bool {
    crate::config::KernelConfig::multi_user_enabled()
}

#[inline(always)]
fn is_effective_root() -> bool {
    geteuid() == 0
}

#[inline(always)]
fn may_assume_uid(uid: u32) -> bool {
    if !credentials_enforced() {
        return true;
    }
    if is_effective_root() {
        return true;
    }
    uid == REAL_UID.load(Ordering::Relaxed)
        || uid == EFFECTIVE_UID.load(Ordering::Relaxed)
        || uid == SAVED_UID.load(Ordering::Relaxed)
}

#[inline(always)]
fn may_assume_gid(gid: u32) -> bool {
    if !credentials_enforced() {
        return true;
    }
    if is_effective_root() {
        return true;
    }
    gid == REAL_GID.load(Ordering::Relaxed)
        || gid == EFFECTIVE_GID.load(Ordering::Relaxed)
        || gid == SAVED_GID.load(Ordering::Relaxed)
}

pub fn get_personality() -> u32 {
    PERSONALITY.load(Ordering::Relaxed)
}

pub fn set_personality(persona: u32) -> u32 {
    PERSONALITY.swap(persona, Ordering::Relaxed)
}

#[inline(always)]
pub fn getuid() -> u32 {
    REAL_UID.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn geteuid() -> u32 {
    EFFECTIVE_UID.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn getgid() -> u32 {
    REAL_GID.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn getegid() -> u32 {
    EFFECTIVE_GID.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn getresuid() -> (u32, u32, u32) {
    let ruid = getuid();
    let euid = geteuid();
    let suid = SAVED_UID.load(Ordering::Relaxed);
    (ruid, euid, suid)
}

#[inline(always)]
pub fn getresgid() -> (u32, u32, u32) {
    let rgid = getgid();
    let egid = getegid();
    let sgid = SAVED_GID.load(Ordering::Relaxed);
    (rgid, egid, sgid)
}

pub fn setuid(uid: u32) -> Result<(), PosixErrno> {
    if !multi_user_enabled() && uid != 0 {
        return Err(PosixErrno::PermissionDenied);
    }
    if !may_assume_uid(uid) {
        return Err(PosixErrno::PermissionDenied);
    }
    if is_effective_root() || !credentials_enforced() {
        REAL_UID.store(uid, Ordering::Relaxed);
        EFFECTIVE_UID.store(uid, Ordering::Relaxed);
        SAVED_UID.store(uid, Ordering::Relaxed);
    } else {
        EFFECTIVE_UID.store(uid, Ordering::Relaxed);
    }
    Ok(())
}

pub fn seteuid(uid: u32) -> Result<(), PosixErrno> {
    if !multi_user_enabled() && uid != 0 {
        return Err(PosixErrno::PermissionDenied);
    }
    if !may_assume_uid(uid) {
        return Err(PosixErrno::PermissionDenied);
    }
    EFFECTIVE_UID.store(uid, Ordering::Relaxed);
    Ok(())
}

pub fn setgid(gid: u32) -> Result<(), PosixErrno> {
    if !multi_user_enabled() && gid != 0 {
        return Err(PosixErrno::PermissionDenied);
    }
    if !may_assume_gid(gid) {
        return Err(PosixErrno::PermissionDenied);
    }
    if is_effective_root() || !credentials_enforced() {
        REAL_GID.store(gid, Ordering::Relaxed);
        EFFECTIVE_GID.store(gid, Ordering::Relaxed);
        SAVED_GID.store(gid, Ordering::Relaxed);
    } else {
        EFFECTIVE_GID.store(gid, Ordering::Relaxed);
    }
    Ok(())
}

pub fn setegid(gid: u32) -> Result<(), PosixErrno> {
    if !multi_user_enabled() && gid != 0 {
        return Err(PosixErrno::PermissionDenied);
    }
    if !may_assume_gid(gid) {
        return Err(PosixErrno::PermissionDenied);
    }
    EFFECTIVE_GID.store(gid, Ordering::Relaxed);
    Ok(())
}

pub fn setresuid(ruid: u32, euid: u32, suid: u32) -> Result<(), PosixErrno> {
    if !multi_user_enabled() && (ruid != 0 || euid != 0 || suid != 0) {
        return Err(PosixErrno::PermissionDenied);
    }
    if credentials_enforced()
        && !is_effective_root()
        && !(may_assume_uid(ruid) && may_assume_uid(euid) && may_assume_uid(suid))
    {
        return Err(PosixErrno::PermissionDenied);
    }
    REAL_UID.store(ruid, Ordering::Relaxed);
    EFFECTIVE_UID.store(euid, Ordering::Relaxed);
    SAVED_UID.store(suid, Ordering::Relaxed);
    Ok(())
}

pub fn setresgid(rgid: u32, egid: u32, sgid: u32) -> Result<(), PosixErrno> {
    if !multi_user_enabled() && (rgid != 0 || egid != 0 || sgid != 0) {
        return Err(PosixErrno::PermissionDenied);
    }
    if credentials_enforced()
        && !is_effective_root()
        && !(may_assume_gid(rgid) && may_assume_gid(egid) && may_assume_gid(sgid))
    {
        return Err(PosixErrno::PermissionDenied);
    }
    REAL_GID.store(rgid, Ordering::Relaxed);
    EFFECTIVE_GID.store(egid, Ordering::Relaxed);
    SAVED_GID.store(sgid, Ordering::Relaxed);
    Ok(())
}

pub fn umask(new_mask: u32) -> u32 {
    let masked = new_mask & 0o777;
    UMASK_BITS.swap(masked, Ordering::Relaxed)
}

#[inline(always)]
pub fn current_umask() -> u32 {
    UMASK_BITS.load(Ordering::Relaxed)
}

pub fn getgroups(out: &mut [u32]) -> Result<usize, PosixErrno> {
    let groups = GROUP_MEMBERSHIP.lock();
    if out.len() < groups.len() {
        return Err(PosixErrno::Invalid);
    }
    for (index, gid) in groups.iter().enumerate() {
        out[index] = *gid;
    }
    Ok(groups.len())
}

pub fn setgroups(groups: &[u32]) -> Result<(), PosixErrno> {
    if credentials_enforced() && !is_effective_root() {
        return Err(PosixErrno::PermissionDenied);
    }
    let mut table = GROUP_MEMBERSHIP.lock();
    table.clear();
    table.extend_from_slice(groups);
    Ok(())
}

pub fn initgroups(primary_gid: u32) -> Result<(), PosixErrno> {
    setgroups(&[primary_gid])
}

pub fn get_groups_len() -> usize {
    GROUP_MEMBERSHIP.lock().len()
}

pub fn get_groups_snapshot() -> Vec<u32> {
    GROUP_MEMBERSHIP.lock().clone()
}

pub fn setenv(key: &str, value: &str, overwrite: bool) -> Result<(), PosixErrno> {
    if key.is_empty() || key.contains('=') {
        return Err(PosixErrno::Invalid);
    }

    let mut env = ENV_TABLE.lock();
    if env.contains_key(key) && !overwrite {
        return Ok(());
    }
    env.insert(String::from(key), String::from(value));
    Ok(())
}

pub fn getenv(key: &str) -> Option<String> {
    ENV_TABLE.lock().get(key).cloned()
}

pub fn unsetenv(key: &str) -> Result<(), PosixErrno> {
    if key.is_empty() || key.contains('=') {
        return Err(PosixErrno::Invalid);
    }
    ENV_TABLE.lock().remove(key);
    Ok(())
}

pub fn clearenv() {
    ENV_TABLE.lock().clear();
}

pub fn environ_snapshot() -> Vec<(String, String)> {
    ENV_TABLE
        .lock()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

pub fn sethostname(name: &str) -> Result<(), PosixErrno> {
    if credentials_enforced() && !is_effective_root() {
        return Err(PosixErrno::PermissionDenied);
    }
    if name.is_empty() {
        return Err(PosixErrno::Invalid);
    }
    *HOSTNAME.lock() = String::from(name);
    Ok(())
}

pub fn gethostname(out: &mut [u8]) -> Result<usize, PosixErrno> {
    let name = HOSTNAME.lock();
    let bytes = name.as_bytes();
    if out.len() < bytes.len() {
        return Err(PosixErrno::Invalid);
    }
    out[..bytes.len()].copy_from_slice(bytes);
    Ok(bytes.len())
}

pub fn setdomainname(name: &str) -> Result<(), PosixErrno> {
    if credentials_enforced() && !is_effective_root() {
        return Err(PosixErrno::PermissionDenied);
    }
    if name.is_empty() {
        return Err(PosixErrno::Invalid);
    }
    *DOMAINNAME.lock() = String::from(name);
    Ok(())
}

pub fn getdomainname(out: &mut [u8]) -> Result<usize, PosixErrno> {
    let name = DOMAINNAME.lock();
    let bytes = name.as_bytes();
    if out.len() < bytes.len() {
        return Err(PosixErrno::Invalid);
    }
    out[..bytes.len()].copy_from_slice(bytes);
    Ok(bytes.len())
}

pub fn get_hostname() -> String {
    HOSTNAME.lock().clone()
}

pub fn get_domainname() -> String {
    DOMAINNAME.lock().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn umask_masks_high_bits_and_returns_previous_value() {
        let original = current_umask();
        let previous = umask(0o1777);
        assert_eq!(previous, original);
        assert_eq!(current_umask(), 0o777);
        let _ = umask(original);
    }

    #[test_case]
    fn setenv_rejects_empty_and_equals_keys() {
        assert_eq!(setenv("", "x", true), Err(PosixErrno::Invalid));
        assert_eq!(setenv("A=B", "x", true), Err(PosixErrno::Invalid));
    }

    #[test_case]
    fn getgroups_rejects_short_output_buffer() {
        setgroups(&[1, 2, 3]).expect("setgroups");
        let mut short = [0u32; 2];
        assert_eq!(getgroups(&mut short), Err(PosixErrno::Invalid));
    }

    #[test_case]
    fn hostname_and_domainname_reject_empty_and_short_buffers() {
        assert_eq!(sethostname(""), Err(PosixErrno::Invalid));
        assert_eq!(setdomainname(""), Err(PosixErrno::Invalid));

        sethostname("aethercore-test").expect("sethostname");
        setdomainname("kernel.test").expect("setdomainname");

        let mut short = [0u8; 4];
        assert_eq!(gethostname(&mut short), Err(PosixErrno::Invalid));
        assert_eq!(getdomainname(&mut short), Err(PosixErrno::Invalid));
    }

    #[test_case]
    fn setresuid_and_setresgid_reject_nonzero_ids() {
        assert_eq!(setresuid(1, 0, 0), Err(PosixErrno::PermissionDenied));
        assert_eq!(setresgid(0, 1, 0), Err(PosixErrno::PermissionDenied));
    }

    #[test_case]
    fn multi_user_toggle_controls_non_root_identity_assignment() {
        crate::config::KernelConfig::reset_runtime_overrides();
        assert_eq!(setuid(42), Err(PosixErrno::PermissionDenied));

        crate::config::KernelConfig::set_multi_user_enabled(Some(true));
        crate::config::KernelConfig::set_credential_enforcement_enabled(Some(false));
        assert_eq!(setuid(42), Ok(()));
        assert_eq!(getuid(), 42);
        assert_eq!(geteuid(), 42);
        assert_eq!(getresuid(), (42, 42, 42));

        crate::config::KernelConfig::reset_runtime_overrides();
        let _ = setuid(0);
    }
}
