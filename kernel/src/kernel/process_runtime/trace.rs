#[inline(always)]
pub(super) fn early_serial(message: &str) {
    crate::hal::serial::write_raw(message);
}
