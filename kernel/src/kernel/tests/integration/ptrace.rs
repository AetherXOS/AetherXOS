use super::types::*;

impl IntegrationHarness {
    pub fn ptrace(&mut self, req: PtraceRequest, pid: u32, regs: RegisterState) -> Result<RegisterState, IntegrationError> {
        if pid == 0 {
            return Err(IntegrationError::InvalidPid);
        }

        match req {
            PtraceRequest::Attach => {
                self.ptrace_attached_pid = Some(pid);
                Ok(regs)
            }
            PtraceRequest::GetRegs => {
                if self.ptrace_attached_pid == Some(pid) {
                    Ok(regs)
                } else {
                    Err(IntegrationError::InvalidPtraceRequest)
                }
            }
            PtraceRequest::SingleStep => {
                if self.ptrace_attached_pid == Some(pid) {
                    Ok(RegisterState { rip: regs.rip + 1, ..regs })
                } else {
                    Err(IntegrationError::InvalidPtraceRequest)
                }
            }
        }
    }

    pub fn ptrace_detach(&mut self, pid: u32) -> Result<(), IntegrationError> {
        if self.ptrace_attached_pid == Some(pid) {
            self.ptrace_attached_pid = None;
            return Ok(());
        }
        Err(IntegrationError::InvalidPtraceRequest)
    }

    pub fn ptrace_peekdata(&self, pid: u32, addr: usize) -> Result<usize, IntegrationError> {
        if addr == 0 || self.ptrace_attached_pid != Some(pid) {
            return Err(IntegrationError::InvalidPtraceRequest);
        }
        Ok(0xCC00_CC00_CC00_CC00usize ^ addr)
    }

    pub fn ptrace_pokedata(&self, pid: u32, addr: usize, _data: usize) -> Result<(), IntegrationError> {
        if addr == 0 || self.ptrace_attached_pid != Some(pid) {
            return Err(IntegrationError::InvalidPtraceRequest);
        }
        Ok(())
    }

    pub fn ptrace_continue(&self, pid: u32) -> Result<(), IntegrationError> {
        if self.ptrace_attached_pid != Some(pid) {
            return Err(IntegrationError::InvalidPtraceRequest);
        }
        Ok(())
    }

    pub fn ptrace_breakpoint_cycle(&mut self, pid: u32, addr: usize) -> Result<(), IntegrationError> {
        let regs = RegisterState { rip: addr, rsp: 0x7000, rax: 0, rbx: 0 };
        self.ptrace(PtraceRequest::Attach, pid, regs)?;
        let original = self.ptrace_peekdata(pid, addr)?;
        self.ptrace_pokedata(pid, addr, original ^ 0xCC)?;
        self.ptrace_continue(pid)?;
        self.ptrace_pokedata(pid, addr, original)?;
        self.ptrace_detach(pid)?;
        Ok(())
    }

    pub fn ptrace_signal_stop_observed(&self, pid: u32, signal: u8) -> Result<bool, IntegrationError> {
        if signal == 0 || self.ptrace_attached_pid != Some(pid) {
            return Err(IntegrationError::InvalidPtraceRequest);
        }
        Ok(true)
    }

    pub fn ptrace_call_stack_depth(&self, pid: u32) -> Result<usize, IntegrationError> {
        if self.ptrace_attached_pid != Some(pid) {
            return Err(IntegrationError::InvalidPtraceRequest);
        }
        Ok(4)
    }

    pub fn ptrace_syscall_arguments(&self, pid: u32) -> Result<[usize; 6], IntegrationError> {
        if self.ptrace_attached_pid != Some(pid) {
            return Err(IntegrationError::InvalidPtraceRequest);
        }
        Ok([1, 2, 3, 4, 5, 6])
    }

    pub fn boundary_mode_ptrace_valid(&self, mode: &str) -> bool {
        matches!(mode, "strict" | "balanced" | "compat")
    }
}
