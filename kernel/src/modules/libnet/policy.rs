#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibNetPolicyError {
    NetworkSurfaceDisabled,
    L2Disabled,
    L34Disabled,
    L6Disabled,
    L7Disabled,
}

impl LibNetPolicyError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NetworkSurfaceDisabled => "network library surface disabled by policy",
            Self::L2Disabled => "libnet l2 disabled by policy",
            Self::L34Disabled => "libnet l3/l4 disabled by policy",
            Self::L6Disabled => "libnet l6 disabled by policy",
            Self::L7Disabled => "libnet l7 disabled by policy",
        }
    }
}

pub fn ensure_network_surface() -> Result<(), &'static str> {
    if !crate::config::KernelConfig::is_network_library_api_exposed() {
        return Err(LibNetPolicyError::NetworkSurfaceDisabled.as_str());
    }
    Ok(())
}

pub fn ensure_l2_enabled() -> Result<(), &'static str> {
    ensure_network_surface()?;
    if !crate::config::KernelConfig::libnet_l2_enabled() {
        return Err(LibNetPolicyError::L2Disabled.as_str());
    }
    Ok(())
}

pub fn ensure_l34_enabled() -> Result<(), &'static str> {
    ensure_network_surface()?;
    if !crate::config::KernelConfig::libnet_l34_enabled() {
        return Err(LibNetPolicyError::L34Disabled.as_str());
    }
    Ok(())
}

pub fn ensure_l6_enabled() -> Result<(), &'static str> {
    ensure_network_surface()?;
    if !crate::config::KernelConfig::libnet_l6_enabled() {
        return Err(LibNetPolicyError::L6Disabled.as_str());
    }
    Ok(())
}

pub fn ensure_l7_enabled() -> Result<(), &'static str> {
    ensure_network_surface()?;
    if !crate::config::KernelConfig::libnet_l7_enabled() {
        return Err(LibNetPolicyError::L7Disabled.as_str());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn policy_error_as_str_returns_expected_messages() {
        assert_eq!(
            LibNetPolicyError::NetworkSurfaceDisabled.as_str(),
            "network library surface disabled by policy"
        );
        assert_eq!(
            LibNetPolicyError::L2Disabled.as_str(),
            "libnet l2 disabled by policy"
        );
        assert_eq!(
            LibNetPolicyError::L34Disabled.as_str(),
            "libnet l3/l4 disabled by policy"
        );
        assert_eq!(
            LibNetPolicyError::L6Disabled.as_str(),
            "libnet l6 disabled by policy"
        );
        assert_eq!(
            LibNetPolicyError::L7Disabled.as_str(),
            "libnet l7 disabled by policy"
        );
    }

    #[test_case]
    fn policy_guards_match_kernel_config() {
        assert_eq!(
            ensure_network_surface().is_ok(),
            crate::config::KernelConfig::is_network_library_api_exposed()
        );

        assert_eq!(
            ensure_l2_enabled().is_ok(),
            crate::config::KernelConfig::libnet_l2_enabled()
        );

        assert_eq!(
            ensure_l34_enabled().is_ok(),
            crate::config::KernelConfig::libnet_l34_enabled()
        );

        assert_eq!(
            ensure_l6_enabled().is_ok(),
            crate::config::KernelConfig::libnet_l6_enabled()
        );

        assert_eq!(
            ensure_l7_enabled().is_ok(),
            crate::config::KernelConfig::libnet_l7_enabled()
        );
    }
}
