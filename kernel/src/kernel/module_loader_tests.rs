use super::*;

fn machine_code(machine: Machine) -> u16 {
    match machine {
        Machine::X86_64 => 62,
        Machine::AArch64 => 183,
        _ => 0,
    }
}

fn minimal_elf64(machine: Machine) -> [u8; ELF_HEADER_MIN_BYTES] {
    let mut image = [0u8; ELF_HEADER_MIN_BYTES];
    image[0..4].copy_from_slice(b"\x7FELF");
    image[4] = 2;
    image[5] = 1;
    image[6] = 1;
    image[16..18].copy_from_slice(&2u16.to_le_bytes());
    image[18..20].copy_from_slice(&machine_code(machine).to_le_bytes());
    image[20..24].copy_from_slice(&1u32.to_le_bytes());
    image[24..32].copy_from_slice(&0x400000u64.to_le_bytes());
    image[32..40].copy_from_slice(&0u64.to_le_bytes());
    image[40..48].copy_from_slice(&0u64.to_le_bytes());
    image[52..54].copy_from_slice(&(ELF_HEADER_MIN_BYTES as u16).to_le_bytes());
    image
}

#[test_case]
fn inspect_elf_image_reports_target_machine() {
    let image = minimal_elf64(current_target_elf_machine());
    let info = inspect_elf_image(&image).expect("inspect target machine");
    assert_eq!(info.machine, current_target_elf_machine());
}

#[test_case]
fn inspect_elf_image_rejects_wrong_machine() {
    #[cfg(target_arch = "x86_64")]
    let wrong_machine = Machine::AArch64;
    #[cfg(target_arch = "aarch64")]
    let wrong_machine = Machine::X86_64;

    let image = minimal_elf64(wrong_machine);
    assert_eq!(
        inspect_elf_image(&image).unwrap_err(),
        ModuleLoadError::UnsupportedMachine
    );
}

#[cfg(all(
    feature = "process_abstraction",
    feature = "paging_enable",
    target_arch = "x86_64"
))]
#[test_case]
fn runtime_init_trampoline_preserves_hook_order_and_final_jump() {
    let mut buf = [0u8; 128];
    let hooks = [0x1111_2222_3333_4444u64, 0x5555_6666_7777_8888u64];
    let final_entry = 0x9999_AAAA_BBBB_CCCCu64;
    let used = encode_x86_64_runtime_init_trampoline(&mut buf, &hooks, final_entry)
        .expect("encode runtime trampoline");

    assert!(used >= 40);
    assert_eq!(&buf[0..2], &[0x48, 0xB8]);
    assert_eq!(u64::from_le_bytes(buf[2..10].try_into().unwrap()), hooks[0]);
    assert_eq!(&buf[15..17], &[0xFF, 0xD0]);

    assert_eq!(&buf[17..19], &[0x48, 0xB8]);
    assert_eq!(
        u64::from_le_bytes(buf[19..27].try_into().unwrap()),
        hooks[1]
    );
    assert_eq!(&buf[32..34], &[0xFF, 0xD0]);

    assert_eq!(&buf[34..36], &[0x48, 0xB8]);
    assert_eq!(
        u64::from_le_bytes(buf[36..44].try_into().unwrap()),
        final_entry
    );
    assert_eq!(&buf[44..46], &[0xFF, 0xE0]);
}
