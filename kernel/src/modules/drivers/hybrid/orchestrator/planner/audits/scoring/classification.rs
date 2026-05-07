use crate::modules::drivers::hybrid::orchestrator::{
    BackendPreference, HybridPerformanceTier, HybridRuntimeConfidence, HybridSecurityPosture,
};
use super::metrics::{score_mean, score_spread};

pub fn support_cutoff_for_scores(scores: &[u8]) -> u8 {
    if scores.is_empty() {
        return 0;
    }

    let mean = score_mean(scores);
    let spread = score_spread(scores);
    mean.saturating_sub(spread / 4)
}

pub fn confidence_cutoffs_for_scores(scores: &[u8]) -> (u8, u8) {
    if scores.is_empty() {
        return (0, 0);
    }

    let mean = score_mean(scores);
    let spread = score_spread(scores);
    let medium_floor = mean.saturating_sub(spread / 6);
    let high_floor = mean.saturating_add(spread / 3).min(100);
    (medium_floor, high_floor)
}

pub fn performance_cutoffs_for_scores(scores: &[u8]) -> (u8, u8) {
    if scores.is_empty() {
        return (0, 0);
    }

    let mean = score_mean(scores);
    let spread = score_spread(scores);
    let low_floor = mean.saturating_sub(spread / 3);
    let high_floor = mean.saturating_add(spread / 4).min(100);
    (low_floor, high_floor)
}

pub fn classify_confidence(
    score: u8,
    medium_floor: u8,
    high_floor: u8,
    supported: bool,
    degraded: bool,
) -> HybridRuntimeConfidence {
    if !supported || score < medium_floor {
        return HybridRuntimeConfidence::Low;
    }
    if degraded || score < high_floor {
        return HybridRuntimeConfidence::Medium;
    }
    HybridRuntimeConfidence::High
}

pub fn classify_performance(
    backend: BackendPreference,
    score: u8,
    low_floor: u8,
    high_floor: u8,
) -> HybridPerformanceTier {
    if score < low_floor {
        return HybridPerformanceTier::Low;
    }
    match backend {
        BackendPreference::ReactOsFirst => HybridPerformanceTier::Low,
        BackendPreference::DriverKitFirst => HybridPerformanceTier::Medium,
        BackendPreference::LibLinuxFirst | BackendPreference::SideCarFirst => {
            if score >= high_floor {
                HybridPerformanceTier::High
            } else {
                HybridPerformanceTier::Medium
            }
        }
    }
}

pub fn classify_security(
    backend: BackendPreference,
    degraded: bool,
) -> (HybridSecurityPosture, &'static str) {
    match (backend, degraded) {
        (BackendPreference::SideCarFirst, false) => (
            HybridSecurityPosture::Isolated,
            "strong isolation boundary with contained fault domain",
        ),
        (BackendPreference::SideCarFirst, true) => (
            HybridSecurityPosture::Mediated,
            "isolation present but runtime health indicates reduced assurance",
        ),
        (BackendPreference::LibLinuxFirst, false) => (
            HybridSecurityPosture::Mediated,
            "mature mediation path but shared-kernel semantics increase blast radius",
        ),
        (BackendPreference::LibLinuxFirst, true) => (
            HybridSecurityPosture::CompatibilityRisk,
            "degraded bridge path can expose semantic mismatches under fault",
        ),
        (BackendPreference::DriverKitFirst, false) => (
            HybridSecurityPosture::Mediated,
            "user-mode execution improves containment but IPC/broker surface stays wide",
        ),
        (BackendPreference::DriverKitFirst, true) => (
            HybridSecurityPosture::CompatibilityRisk,
            "health degradation indicates instability in user-mode mediation layer",
        ),
        (BackendPreference::ReactOsFirst, _) => (
            HybridSecurityPosture::CompatibilityRisk,
            "compatibility translation path carries semantic and hardening gaps",
        ),
    }
}
