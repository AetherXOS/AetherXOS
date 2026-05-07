pub mod vm;
pub mod instructions;
pub mod verifier;

pub use vm::{BpfVm, BpfContext, BpfResult};

pub enum BpfProgramKind {
    Seccomp,
    SocketFilter,
    Tracepoint,
}

pub struct BpfProgram {
    pub kind: BpfProgramKind,
    pub instructions: alloc::vec::Vec<u64>,
    pub is_verified: bool,
}
