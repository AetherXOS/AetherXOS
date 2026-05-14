/// Emit a raw serial trace message.
///
/// **Only active in debug builds** (`debug_assertions`).
/// In release builds this function is a complete no-op and will be
/// elided by the compiler — zero runtime cost.
///
/// Use `crate::hal::serial::write_raw` directly only for pre-heap
/// boot paths where the structured log system is unavailable.
#[inline(always)]
pub(super) fn early_serial(message: &str) {
    #[cfg(debug_assertions)]
    crate::hal::serial::write_raw(message);
    #[cfg(not(debug_assertions))]
    let _ = message;
}
