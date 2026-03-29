//! Feature coherence validation — cross-feature dependency checks.

use crate::build_cfg::config_types::Config;
use crate::build_cfg::feature_graph::feature_enabled;

/// Validates that Cargo feature flags are coherent with config settings.
pub fn validate_feature_coherence(config: &Config) -> Vec<String> {
    let mut e = Vec::new();

    // libnet layer dependencies
    if feature_enabled("libnet") {
        let strict = config.library.strict_optional_features;

        if config.library.libnet_l34_enabled && !feature_enabled("network_transport") {
            let msg = "libnet_l34_enabled=true requires feature `network_transport`";
            if strict {
                e.push(msg.to_string());
            } else {
                println!("cargo:warning={msg}");
            }
        }

        if config.library.libnet_l6_enabled && !feature_enabled("network_https") {
            let msg = "libnet_l6_enabled=true requires feature `network_https`";
            if strict {
                e.push(msg.to_string());
            } else {
                println!("cargo:warning={msg}");
            }
        }

        if config.library.libnet_l7_enabled
            && !(feature_enabled("network_http") || feature_enabled("libnet_l7_http2"))
        {
            let msg = "libnet_l7_enabled=true requires `network_http` or `libnet_l7_http2`";
            if strict {
                e.push(msg.to_string());
            } else {
                println!("cargo:warning={msg}");
            }
        }
    }

    // Zero-trust requires a real security monitor
    if config.security.zero_trust_mode && config.security.monitor == "NullMonitor" {
        e.push("security.zero_trust_mode=true is incompatible with NullMonitor".to_string());
    }

    e
}
