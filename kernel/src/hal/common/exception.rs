#[derive(Debug, Clone, Copy)]
pub struct ExceptionSnapshot<'a> {
    pub trace_label: &'a str,
    pub dump_label: &'a str,
    pub frame_bytes: &'a [u8],
    pub instruction_pointer: u64,
    pub stack_pointer: u64,
    pub fault_or_code: u64,
    pub status_or_flags: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ExceptionDescriptor<Id> {
    pub id: Id,
    pub label: &'static str,
}

impl<Id: Copy> ExceptionDescriptor<Id> {
    pub const fn new(id: Id, label: &'static str) -> Self {
        Self { id, label }
    }
}

#[inline(always)]
pub fn classify_exception<Id: Copy + Eq>(
    id: Id,
    table: &[ExceptionDescriptor<Id>],
    default_label: &'static str,
) -> ExceptionDescriptor<Id> {
    for descriptor in table {
        if descriptor.id == id {
            return *descriptor;
        }
    }

    ExceptionDescriptor::new(id, default_label)
}

#[inline(always)]
pub fn record_exception_snapshot(snapshot: ExceptionSnapshot<'_>) {
    if crate::config::KernelConfig::is_advanced_debug_enabled() {
        crate::hal::serial::write_dump_bytes(snapshot.dump_label, snapshot.frame_bytes);
    }

    crate::kernel::debug_trace::record_register_snapshot(
        snapshot.trace_label,
        snapshot.instruction_pointer,
        snapshot.stack_pointer,
        snapshot.fault_or_code,
        snapshot.status_or_flags,
    );
}
