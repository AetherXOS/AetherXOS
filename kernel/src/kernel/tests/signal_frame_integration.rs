/// Signal Frame Integration Tests
///
/// Executable no_std integration coverage for signal frame delivery.

#[cfg(test)]
mod tests {
    use super::super::integration_harness::{IntegrationHarness, RegisterState};

    #[test_case]
    fn signal_frame_is_16_byte_aligned() {
        let harness = IntegrationHarness::new();
        let regs = RegisterState { rip: 0x1000, rsp: 0x8000, rax: 1, rbx: 2 };

        let frame = harness
            .deliver_signal(10, 0x7fff_f000, 512, regs)
            .expect("signal delivery should succeed");

        assert_eq!(frame.frame_addr % 16, 0, "frame must be 16-byte aligned");
    }

    #[test_case]
    fn signal_frame_preserves_register_state() {
        let harness = IntegrationHarness::new();
        let regs = RegisterState { rip: 0x1234, rsp: 0x9000, rax: 11, rbx: 22 };

        let frame = harness
            .deliver_signal(12, 0x7fff_f000, 512, regs)
            .expect("signal delivery should succeed");

        assert_eq!(frame.regs, regs, "registers must be preserved in signal frame");
    }

    #[test_case]
    fn signal_delivery_rejects_zero_signal() {
        let harness = IntegrationHarness::new();
        let regs = RegisterState { rip: 0x1, rsp: 0x2, rax: 3, rbx: 4 };

        let result = harness.deliver_signal(0, 0x7fff_f000, 512, regs);
        assert!(result.is_err(), "zero signal must be rejected");
    }

    #[test_case]
    fn signal_delivery_rejects_unaligned_restorer() {
        let harness = IntegrationHarness::new();
        let regs = RegisterState { rip: 0x20, rsp: 0x40, rax: 1, rbx: 2 };

        let result = harness.deliver_signal(10, 0x7fff_f003, 512, regs);
        assert!(result.is_err(), "unaligned restorer must be rejected");
    }

    #[test_case]
    fn signal_delivery_rejects_too_small_frame() {
        let harness = IntegrationHarness::new();
        let regs = RegisterState { rip: 0x80, rsp: 0x100, rax: 9, rbx: 10 };

        let result = harness.deliver_signal(10, 0x7fff_f000, 64, regs);
        assert!(result.is_err(), "too-small signal frame should fail validation");
    }
}
