use crate::modules::drivers::hybrid::orchestrator::{
    BackendPreference, HybridGapSeverity, HybridPerformanceTier, HybridSecurityPosture,
};
use super::caps::backend_capabilities;

pub fn performance_score_for(backend: BackendPreference, tier: HybridPerformanceTier) -> u8 {
    let caps = backend_capabilities(backend);
    let tier_bias = match tier {
        HybridPerformanceTier::Low => -18,
        HybridPerformanceTier::Medium => 0,
        HybridPerformanceTier::High => 18,
    };

    (42 + caps.throughput * 2 + caps.native_semantics + tier_bias).clamp(0, 100) as u8
}

pub fn security_score_for(backend: BackendPreference, posture: HybridSecurityPosture) -> u8 {
    let caps = backend_capabilities(backend);
    let posture_bias = match posture {
        HybridSecurityPosture::Isolated => 18,
        HybridSecurityPosture::Mediated => 4,
        HybridSecurityPosture::CompatibilityRisk => -12,
    };

    (40 + caps.security_containment * 2 + caps.isolation + posture_bias).clamp(0, 100) as u8
}

pub fn maturity_score_to_gap(score: u8, peer_scores: &[u8]) -> HybridGapSeverity {
    if peer_scores.is_empty() {
        return HybridGapSeverity::Critical;
    }

    let mean = score_mean(peer_scores);
    let spread = score_spread(peer_scores);
    let info_floor = mean.saturating_add(spread / 6).min(100);
    let warning_floor = mean.saturating_sub(spread / 8);

    if score >= info_floor {
        HybridGapSeverity::Info
    } else if score >= warning_floor {
        HybridGapSeverity::Warning
    } else {
        HybridGapSeverity::Critical
    }
}

pub fn maturity_overall_score(scores: &[u8]) -> u8 {
    if scores.is_empty() {
        return 0;
    }

    (scores.iter().map(|score| *score as usize).sum::<usize>() / scores.len()) as u8
}

pub fn score_mean(scores: &[u8]) -> u8 {
    maturity_overall_score(scores)
}

pub fn score_spread(scores: &[u8]) -> u8 {
    if scores.is_empty() {
        return 0;
    }

    let mut min_score = u8::MAX;
    let mut max_score = u8::MIN;
    for score in scores {
        min_score = min_score.min(*score);
        max_score = max_score.max(*score);
    }
    max_score.saturating_sub(min_score)
}

pub fn adaptive_threshold(scores: &[u8]) -> u8 {
    if scores.is_empty() {
        return 0;
    }

    let mean = score_mean(scores);
    let spread = score_spread(scores);
    mean.saturating_add(spread / 8).min(100)
}

pub fn composite_quality_score(coverage: u8, performance: u8, security: u8) -> u8 {
    (((coverage as u16 * 2) + performance as u16 + security as u16) / 4) as u8
}
