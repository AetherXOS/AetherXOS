/// Syscall Semantic Parity Tests
///
/// Models errno and side-effect ordering rules that Linux userspace relies on.

#[cfg(test)]
mod tests {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Step {
        SideEffect,
        Enoent,
        Efault,
        Einval,
        AbiWrite,
    }

    #[derive(Default)]
    struct Trace {
        steps: [Option<Step>; 8],
        len: usize,
    }

    impl Trace {
        fn push(&mut self, step: Step) {
            if self.len < self.steps.len() {
                self.steps[self.len] = Some(step);
                self.len += 1;
            }
        }

        fn as_slice(&self) -> [Option<Step>; 8] {
            self.steps
        }
    }

    fn semantic_open(path_exists: bool, write_allowed: bool, trace: &mut Trace) -> Result<(), Step> {
        trace.push(Step::SideEffect);
        if !path_exists {
            return Err(Step::Enoent);
        }
        if !write_allowed {
            return Err(Step::Einval);
        }
        Ok(())
    }

    fn semantic_read_user(ptr_valid: bool, trace: &mut Trace) -> Result<(), Step> {
        if !ptr_valid {
            trace.push(Step::Efault);
            return Err(Step::Efault);
        }
        trace.push(Step::SideEffect);
        Ok(())
    }

    fn semantic_write_then_error(ptr_valid: bool, trace: &mut Trace) -> Result<(), Step> {
        trace.push(Step::SideEffect);
        if !ptr_valid {
            return Err(Step::Efault);
        }
        Err(Step::Enoent)
    }

    fn semantic_get_abi_info(buf_len: usize, required_len: usize, trace: &mut Trace) -> Result<usize, Step> {
        if buf_len < required_len {
            return Err(Step::Einval);
        }

        trace.push(Step::AbiWrite);
        Ok(required_len)
    }

    #[test_case]
    fn errno_is_reported_after_required_side_effects() {
        let mut trace = Trace::default();
        let result = semantic_open(false, true, &mut trace);
        assert_eq!(result, Err(Step::Enoent));
        assert_eq!(trace.as_slice()[0], Some(Step::SideEffect));
    }

    #[test_case]
    fn invalid_pointer_short_circuits_with_efault() {
        let mut trace = Trace::default();
        let result = semantic_read_user(false, &mut trace);
        assert_eq!(result, Err(Step::Efault));
        assert_eq!(trace.as_slice()[0], Some(Step::Efault));
    }

    #[test_case]
    fn side_effect_precedes_failing_errno_on_write_path() {
        let mut trace = Trace::default();
        let result = semantic_write_then_error(false, &mut trace);
        assert_eq!(result, Err(Step::Efault));
        assert_eq!(trace.as_slice()[0], Some(Step::SideEffect));
    }

    #[test_case]
    fn success_path_keeps_ordering_deterministic() {
        let mut trace = Trace::default();
        let result = semantic_open(true, true, &mut trace);
        assert_eq!(result, Ok(()));
        assert_eq!(trace.as_slice()[0], Some(Step::SideEffect));
    }

    #[test_case]
    fn abi_info_requires_sufficient_userspace_buffer() {
        let mut trace = Trace::default();
        let result = semantic_get_abi_info(16, 56, &mut trace);
        assert_eq!(result, Err(Step::Einval));
        assert_eq!(trace.as_slice()[0], None);
    }

    #[test_case]
    fn abi_info_writes_only_after_validation_passes() {
        let mut trace = Trace::default();
        let result = semantic_get_abi_info(64, 56, &mut trace);
        assert_eq!(result, Ok(56));
        assert_eq!(trace.as_slice()[0], Some(Step::AbiWrite));
    }

    #[test_case]
    fn open_rejects_disallowed_write_after_side_effect() {
        let mut trace = Trace::default();
        let result = semantic_open(true, false, &mut trace);
        assert_eq!(result, Err(Step::Einval));
        assert_eq!(trace.as_slice()[0], Some(Step::SideEffect));
    }

    #[test_case]
    fn read_user_success_keeps_side_effect_path() {
        let mut trace = Trace::default();
        let result = semantic_read_user(true, &mut trace);
        assert_eq!(result, Ok(()));
        assert_eq!(trace.as_slice()[0], Some(Step::SideEffect));
    }

    #[test_case]
    fn write_path_reports_enoent_when_pointer_is_valid() {
        let mut trace = Trace::default();
        let result = semantic_write_then_error(true, &mut trace);
        assert_eq!(result, Err(Step::Enoent));
        assert_eq!(trace.as_slice()[0], Some(Step::SideEffect));
    }
}
