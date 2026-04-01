/// Socket Options Integration Tests
///
/// Executable no_std integration coverage for socket option behavior.

#[cfg(test)]
mod tests {
    use super::super::integration_harness::{
        IntegrationHarness, SocketLevel, SocketOptName,
    };

    #[test_case]
    fn setsockopt_persists_reuseaddr_value() {
        let mut harness = IntegrationHarness::new();

        harness
            .setsockopt(SocketLevel::SolSocket, SocketOptName::ReuseAddr, true)
            .expect("setsockopt should succeed");

        let value = harness
            .getsockopt(SocketLevel::SolSocket, SocketOptName::ReuseAddr)
            .expect("getsockopt should succeed");

        assert!(value, "SO_REUSEADDR should persist as enabled");
    }

    #[test_case]
    fn setsockopt_persists_tcp_nodelay_value() {
        let mut harness = IntegrationHarness::new();

        harness
            .setsockopt(SocketLevel::IpProtoTcp, SocketOptName::TcpNoDelay, true)
            .expect("setsockopt should succeed");

        let value = harness
            .getsockopt(SocketLevel::IpProtoTcp, SocketOptName::TcpNoDelay)
            .expect("getsockopt should succeed");

        assert!(value, "TCP_NODELAY should persist as enabled");
    }

    #[test_case]
    fn setsockopt_persists_keepalive_value() {
        let mut harness = IntegrationHarness::new();

        harness
            .setsockopt(SocketLevel::SolSocket, SocketOptName::KeepAlive, true)
            .expect("setsockopt should succeed");

        let value = harness
            .getsockopt(SocketLevel::SolSocket, SocketOptName::KeepAlive)
            .expect("getsockopt should succeed");

        assert!(value, "SO_KEEPALIVE should persist as enabled");
    }

    #[test_case]
    fn getsockopt_returns_error_for_invalid_pairing() {
        let harness = IntegrationHarness::new();

        let res = harness.getsockopt(SocketLevel::IpProtoTcp, SocketOptName::ReuseAddr);
        assert!(res.is_err(), "invalid level/option pairing must be rejected");
    }

    #[test_case]
    fn setsockopt_can_toggle_option_off_after_enable() {
        let mut harness = IntegrationHarness::new();

        harness
            .setsockopt(SocketLevel::SolSocket, SocketOptName::ReuseAddr, true)
            .expect("enable should succeed");
        harness
            .setsockopt(SocketLevel::SolSocket, SocketOptName::ReuseAddr, false)
            .expect("disable should succeed");

        let value = harness
            .getsockopt(SocketLevel::SolSocket, SocketOptName::ReuseAddr)
            .expect("getsockopt should succeed");
        assert!(!value, "SO_REUSEADDR should reflect disabled state");
    }
}
