use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkDriverPolicy {
    PreferVirtIo,
    PreferE1000,
    VirtIoOnly,
    E1000Only,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkRemediationProfile {
    Conservative,
    Balanced,
    Aggressive,
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkRemediationTuning {
    pub breach_streak_threshold: u64,
    pub cooldown_base_samples: u64,
    pub cooldown_jitter_mask: u64,
    pub rebind_before_failover: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkDriverPolicySnapshot {
    pub active_policy: NetworkDriverPolicy,
    pub set_calls: u64,
    pub remediation_profile: NetworkRemediationProfile,
    pub remediation_profile_set_calls: u64,
    pub remediation_tuning: NetworkRemediationTuning,
}

static NETWORK_DRIVER_POLICY_RAW: AtomicU64 =
    AtomicU64::new(policy_to_raw(NetworkDriverPolicy::PreferVirtIo));
static NETWORK_DRIVER_POLICY_SET_CALLS: AtomicU64 = AtomicU64::new(0);
static NETWORK_REMEDIATION_PROFILE_RAW: AtomicU64 = AtomicU64::new(remediation_profile_to_raw(
    NetworkRemediationProfile::Balanced,
));
static NETWORK_REMEDIATION_PROFILE_SET_CALLS: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
const fn policy_to_raw(policy: NetworkDriverPolicy) -> u64 {
    match policy {
        NetworkDriverPolicy::PreferVirtIo => 0,
        NetworkDriverPolicy::PreferE1000 => 1,
        NetworkDriverPolicy::VirtIoOnly => 2,
        NetworkDriverPolicy::E1000Only => 3,
    }
}

#[inline(always)]
const fn raw_to_policy(raw: u64) -> NetworkDriverPolicy {
    match raw {
        1 => NetworkDriverPolicy::PreferE1000,
        2 => NetworkDriverPolicy::VirtIoOnly,
        3 => NetworkDriverPolicy::E1000Only,
        _ => NetworkDriverPolicy::PreferVirtIo,
    }
}

#[inline(always)]
const fn remediation_profile_to_raw(profile: NetworkRemediationProfile) -> u64 {
    match profile {
        NetworkRemediationProfile::Conservative => 0,
        NetworkRemediationProfile::Balanced => 1,
        NetworkRemediationProfile::Aggressive => 2,
    }
}

#[inline(always)]
const fn raw_to_remediation_profile(raw: u64) -> NetworkRemediationProfile {
    match raw {
        0 => NetworkRemediationProfile::Conservative,
        2 => NetworkRemediationProfile::Aggressive,
        _ => NetworkRemediationProfile::Balanced,
    }
}

pub const fn remediation_tuning_for_profile(
    profile: NetworkRemediationProfile,
) -> NetworkRemediationTuning {
    match profile {
        NetworkRemediationProfile::Conservative => NetworkRemediationTuning {
            breach_streak_threshold: 4,
            cooldown_base_samples: 6,
            cooldown_jitter_mask: 0x3,
            rebind_before_failover: true,
        },
        NetworkRemediationProfile::Balanced => NetworkRemediationTuning {
            breach_streak_threshold: 3,
            cooldown_base_samples: 4,
            cooldown_jitter_mask: 0x3,
            rebind_before_failover: true,
        },
        NetworkRemediationProfile::Aggressive => NetworkRemediationTuning {
            breach_streak_threshold: 2,
            cooldown_base_samples: 2,
            cooldown_jitter_mask: 0x1,
            rebind_before_failover: false,
        },
    }
}

pub fn set_network_driver_policy(policy: NetworkDriverPolicy) {
    NETWORK_DRIVER_POLICY_SET_CALLS.fetch_add(1, Ordering::Relaxed);
    NETWORK_DRIVER_POLICY_RAW.store(policy_to_raw(policy), Ordering::Relaxed);
}

pub fn network_driver_policy() -> NetworkDriverPolicy {
    raw_to_policy(NETWORK_DRIVER_POLICY_RAW.load(Ordering::Relaxed))
}

pub fn set_network_remediation_profile(profile: NetworkRemediationProfile) {
    NETWORK_REMEDIATION_PROFILE_SET_CALLS.fetch_add(1, Ordering::Relaxed);
    NETWORK_REMEDIATION_PROFILE_RAW.store(remediation_profile_to_raw(profile), Ordering::Relaxed);
}

pub fn network_remediation_profile() -> NetworkRemediationProfile {
    raw_to_remediation_profile(NETWORK_REMEDIATION_PROFILE_RAW.load(Ordering::Relaxed))
}

pub fn network_driver_policy_snapshot() -> NetworkDriverPolicySnapshot {
    let remediation_profile = network_remediation_profile();
    NetworkDriverPolicySnapshot {
        active_policy: network_driver_policy(),
        set_calls: NETWORK_DRIVER_POLICY_SET_CALLS.load(Ordering::Relaxed),
        remediation_profile,
        remediation_profile_set_calls: NETWORK_REMEDIATION_PROFILE_SET_CALLS
            .load(Ordering::Relaxed),
        remediation_tuning: remediation_tuning_for_profile(remediation_profile),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn policy_set_roundtrip_updates_snapshot() {
        let before = network_driver_policy_snapshot();
        set_network_driver_policy(NetworkDriverPolicy::PreferE1000);
        let after = network_driver_policy_snapshot();
        assert_eq!(after.active_policy, NetworkDriverPolicy::PreferE1000);
        assert!(after.set_calls >= before.set_calls + 1);
        assert_eq!(after.remediation_profile, before.remediation_profile);

        set_network_driver_policy(NetworkDriverPolicy::PreferVirtIo);
    }

    #[test_case]
    fn remediation_profile_roundtrip_updates_snapshot() {
        let before = network_driver_policy_snapshot();
        set_network_remediation_profile(NetworkRemediationProfile::Aggressive);
        let after = network_driver_policy_snapshot();
        assert_eq!(
            after.remediation_profile,
            NetworkRemediationProfile::Aggressive
        );
        assert!(after.remediation_profile_set_calls >= before.remediation_profile_set_calls + 1);
        assert!(after.remediation_tuning.breach_streak_threshold <= 2);

        set_network_remediation_profile(NetworkRemediationProfile::Balanced);
    }
}
