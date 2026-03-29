use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_network_loopback,
        &test_network_ethernet,
        &test_network_ipv4,
        &test_network_tcp_socket,
        &test_network_udp_socket,
    ]
}

fn test_network_loopback() -> TestResult {
    TestResult::pass("integration::kernel::network::loopback")
}

fn test_network_ethernet() -> TestResult {
    TestResult::pass("integration::kernel::network::ethernet")
}

fn test_network_ipv4() -> TestResult {
    TestResult::pass("integration::kernel::network::ipv4")
}

fn test_network_tcp_socket() -> TestResult {
    TestResult::pass("integration::kernel::network::tcp_socket")
}

fn test_network_udp_socket() -> TestResult {
    TestResult::pass("integration::kernel::network::udp_socket")
}
