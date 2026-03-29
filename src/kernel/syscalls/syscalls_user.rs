use crate::kernel::syscalls::syscalls_consts::*;

#[inline(always)]
pub(crate) fn user_range_valid(ptr: usize, len: usize) -> bool {
    if ptr < USER_SPACE_BOTTOM_INCLUSIVE || len == 0 {
        return false;
    }
    let Some(end) = ptr.checked_add(len) else {
        return false;
    };

    ptr < USER_SPACE_TOP_EXCLUSIVE && end <= USER_SPACE_TOP_EXCLUSIVE && end > ptr
}

#[inline(always)]
pub(crate) fn user_word_aligned(ptr: usize) -> bool {
    ptr % core::mem::align_of::<usize>() == 0
}

#[derive(Clone, Copy)]
pub(crate) enum UserAccessMode {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum UserAccessFault {
    InvalidRange,
    Overflow,
    HhdmMissing,
    PageTableUnavailable,
    NotPresent,
    NotUserAccessible,
    NotWritable,
}
