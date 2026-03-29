pub mod legacy;
pub mod linux;
pub mod posix;
pub mod unix;

/// Standard labels for syscall categorization.
pub enum SyscallStandard {
    Unix,   // V6/V7/BSD legacy
    Posix,  // IEEE 1003.1
    Linux,  // Linux specific extensions
    Legacy, // Obsolescent/Deprecated
}
