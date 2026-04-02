#[inline(always)]
pub(crate) fn map_error<T, E>(result: Result<T, E>, context: &'static str) -> Result<T, &'static str> {
    result.map_err(|_| context)
}

#[inline(always)]
pub(crate) fn require_some<T>(value: Option<T>, context: &'static str) -> Result<T, &'static str> {
    value.ok_or(context)
}