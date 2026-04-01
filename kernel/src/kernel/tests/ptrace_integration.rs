/// Ptrace Integration Tests
///
/// Executable no_std integration coverage for ptrace attach/register flows.

#[cfg(test)]
mod tests {
    use super::super::integration_harness::{
        IntegrationHarness, PtraceRequest, RegisterState,
    };

    #[test_case]
    fn ptrace_attach_then_getregs_succeeds() {
        let mut harness = IntegrationHarness::new();
        let regs = RegisterState { rip: 0x2000, rsp: 0x3000, rax: 7, rbx: 9 };
        let pid = 777;

        harness
            .ptrace(PtraceRequest::Attach, pid, regs)
            .expect("attach should succeed");

        let read = harness
            .ptrace(PtraceRequest::GetRegs, pid, regs)
            .expect("getregs should succeed after attach");

        assert_eq!(read, regs, "ptrace getregs should return register snapshot");
    }

    #[test_case]
    fn ptrace_singlestep_advances_instruction_pointer() {
        let mut harness = IntegrationHarness::new();
        let regs = RegisterState { rip: 0x5000, rsp: 0x3000, rax: 1, rbx: 2 };
        let pid = 888;

        harness
            .ptrace(PtraceRequest::Attach, pid, regs)
            .expect("attach should succeed");

        let stepped = harness
            .ptrace(PtraceRequest::SingleStep, pid, regs)
            .expect("single step should succeed after attach");

        assert_eq!(stepped.rip, regs.rip + 1, "single step should move RIP by one");
    }

    #[test_case]
    fn ptrace_getregs_without_attach_fails() {
        let mut harness = IntegrationHarness::new();
        let regs = RegisterState { rip: 0x10, rsp: 0x20, rax: 0, rbx: 0 };

        let res = harness.ptrace(PtraceRequest::GetRegs, 999, regs);
        assert!(res.is_err(), "getregs without attach must fail");
    }

    #[test_case]
    fn ptrace_attach_rejects_pid_zero() {
        let mut harness = IntegrationHarness::new();
        let regs = RegisterState { rip: 0x10, rsp: 0x20, rax: 0, rbx: 0 };

        let res = harness.ptrace(PtraceRequest::Attach, 0, regs);
        assert!(res.is_err(), "ptrace attach must reject pid zero");
    }

    #[test_case]
    fn ptrace_getregs_rejects_different_pid_than_attached() {
        let mut harness = IntegrationHarness::new();
        let regs = RegisterState { rip: 0x60, rsp: 0x70, rax: 1, rbx: 2 };

        harness
            .ptrace(PtraceRequest::Attach, 1001, regs)
            .expect("attach should succeed");

        let res = harness.ptrace(PtraceRequest::GetRegs, 1002, regs);
        assert!(res.is_err(), "getregs must fail for non-attached pid");
    }
}
