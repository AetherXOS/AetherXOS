use super::DevFsPolicyProfile;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub(crate) const BOOL_OVERRIDE_DEFAULT: usize = 0;
pub(crate) const BOOL_OVERRIDE_FALSE: usize = 1;
pub(crate) const BOOL_OVERRIDE_TRUE: usize = 2;
pub(crate) const TLS_POLICY_PROFILE_OVERRIDE_DEFAULT: usize = 0;
pub(crate) const TLS_POLICY_PROFILE_OVERRIDE_MINIMAL: usize = 1;
pub(crate) const TLS_POLICY_PROFILE_OVERRIDE_BALANCED: usize = 2;
pub(crate) const TLS_POLICY_PROFILE_OVERRIDE_STRICT: usize = 3;
pub(crate) const DEVFS_POLICY_PROFILE_OVERRIDE_DEFAULT: usize = 0;
pub(crate) const DEVFS_POLICY_PROFILE_OVERRIDE_STRICT: usize = 1;
pub(crate) const DEVFS_POLICY_PROFILE_OVERRIDE_BALANCED: usize = 2;
pub(crate) const DEVFS_POLICY_PROFILE_OVERRIDE_DEV: usize = 3;
pub(crate) const BOUNDARY_MODE_OVERRIDE_DEFAULT: usize = 0;
pub(crate) const BOUNDARY_MODE_OVERRIDE_STRICT: usize = 1;
pub(crate) const BOUNDARY_MODE_OVERRIDE_BALANCED: usize = 2;
pub(crate) const BOUNDARY_MODE_OVERRIDE_COMPAT: usize = 3;

#[inline(always)]
pub(crate) fn decode_bool_override(override_value: usize, default: bool) -> bool {
    match override_value {
        BOOL_OVERRIDE_DEFAULT => default,
        BOOL_OVERRIDE_TRUE => true,
        _ => false,
    }
}

#[inline(always)]
pub(crate) fn encode_bool_override(value: Option<bool>) -> usize {
    match value {
        Some(true) => BOOL_OVERRIDE_TRUE,
        Some(false) => BOOL_OVERRIDE_FALSE,
        None => BOOL_OVERRIDE_DEFAULT,
    }
}

#[inline(always)]
pub(crate) fn encode_devfs_policy_override(value: Option<DevFsPolicyProfile>) -> usize {
    match value {
        Some(DevFsPolicyProfile::Strict) => DEVFS_POLICY_PROFILE_OVERRIDE_STRICT,
        Some(DevFsPolicyProfile::Balanced) => DEVFS_POLICY_PROFILE_OVERRIDE_BALANCED,
        Some(DevFsPolicyProfile::Dev) => DEVFS_POLICY_PROFILE_OVERRIDE_DEV,
        None => DEVFS_POLICY_PROFILE_OVERRIDE_DEFAULT,
    }
}

#[inline(always)]
pub(crate) fn decode_devfs_policy_override(
    override_value: usize,
    default: DevFsPolicyProfile,
) -> DevFsPolicyProfile {
    match override_value {
        DEVFS_POLICY_PROFILE_OVERRIDE_STRICT => DevFsPolicyProfile::Strict,
        DEVFS_POLICY_PROFILE_OVERRIDE_BALANCED => DevFsPolicyProfile::Balanced,
        DEVFS_POLICY_PROFILE_OVERRIDE_DEV => DevFsPolicyProfile::Dev,
        _ => default,
    }
}

#[inline(always)]
pub(crate) fn normalize_u16_override(override_value: usize, default: u16, max: u16) -> u16 {
    if override_value == 0 {
        default
    } else {
        (override_value as u16).min(max)
    }
}

#[inline(always)]
pub(crate) fn normalize_u32_override(override_value: usize, default: u32) -> u32 {
    if override_value == 0 {
        default
    } else {
        override_value.min(u32::MAX as usize) as u32
    }
}

#[inline(always)]
pub(crate) fn apply_profile_override<T>(
    value: Option<T>,
    apply: impl FnOnce(T),
    reset: impl FnOnce(),
) {
    if let Some(profile) = value {
        apply(profile);
    } else {
        reset();
    }
}

#[inline(always)]
pub(crate) fn load_u64_override(atom: &AtomicU64, default: u64) -> u64 {
    let override_value = atom.load(Ordering::Relaxed);
    if override_value == 0 {
        default
    } else {
        override_value
    }
}

#[inline(always)]
pub(crate) fn load_u64_override_clamped(atom: &AtomicU64, default: u64, min: u64, max: u64) -> u64 {
    load_u64_override(atom, default).clamp(min, max)
}

#[inline(always)]
pub(crate) fn load_usize_override_clamped(
    atom: &AtomicUsize,
    default: usize,
    min: usize,
    max: usize,
) -> usize {
    let override_value = atom.load(Ordering::Relaxed);
    if override_value == 0 {
        default
    } else {
        override_value.clamp(min, max)
    }
}

#[inline(always)]
pub(crate) fn load_u8_from_usize_override_clamped(
    atom: &AtomicUsize,
    default: u8,
    min: u8,
    max: u8,
) -> u8 {
    let override_value = atom.load(Ordering::Relaxed);
    if override_value == 0 {
        default
    } else {
        u8::try_from(override_value)
            .unwrap_or(default)
            .clamp(min, max)
    }
}

#[inline(always)]
pub(crate) fn store_u64_override(atom: &AtomicU64, value: Option<u64>) {
    atom.store(value.unwrap_or(0), Ordering::Relaxed);
}

#[inline(always)]
pub(crate) fn store_usize_override(atom: &AtomicUsize, value: Option<usize>) {
    atom.store(value.unwrap_or(0), Ordering::Relaxed);
}
