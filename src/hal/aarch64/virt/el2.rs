#[cfg(target_arch = "aarch64")]
fn read_current_el() -> u64 {
    let current_el: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CurrentEL",
            out(reg) current_el,
            options(nomem, nostack)
        );
    }
    current_el
}

#[cfg(not(target_arch = "aarch64"))]
fn read_current_el() -> u64 {
    0
}

#[cfg(target_arch = "aarch64")]
fn read_pfr0() -> u64 {
    let pfr0: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, id_aa64pfr0_el1",
            out(reg) pfr0,
            options(nomem, nostack)
        );
    }
    pfr0
}

#[cfg(not(target_arch = "aarch64"))]
fn read_pfr0() -> u64 {
    0
}

pub(super) fn el2_active() -> bool {
    (read_current_el() >> 2) == 2
}

pub(super) fn el2_supported() -> bool {
    ((read_pfr0() >> 8) & 0xF) != 0xF
}

#[cfg(test)]
mod tests {
    #[test]
    fn el2_field_decodes_reserved_absence() {
        let no_el2 = 0xF_u64 << 8;
        assert_eq!(((no_el2 >> 8) & 0xF) != 0xF, false);
    }

    #[test]
    fn el2_field_decodes_present_values() {
        let present = 0x1_u64 << 8;
        assert!(((present >> 8) & 0xF) != 0xF);
    }
}
