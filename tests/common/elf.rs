use xmas_elf::header::Machine;

const ELF64_HEADER_BYTES: usize = 64;
const ELF64_PROGRAM_HEADER_BYTES: usize = 56;
const PT_LOAD: u32 = 1;
const PF_R: u32 = 0x4;
const PF_X: u32 = 0x1;
const PAGE_ALIGN: u64 = 0x1000;
const ENTRY_VADDR: u64 = 0x401000;
const SEGMENT_OFFSET: u64 = 0x1000;
const SEGMENT_SIZE: usize = 1;

pub fn minimal_loadable_image() -> Vec<u8> {
    let machine = match current_machine() {
        Machine::X86_64 => 62u16,
        Machine::AArch64 => 183u16,
        other => panic!("unsupported target machine for test fixture: {other:?}"),
    };

    let file_size = SEGMENT_OFFSET as usize + SEGMENT_SIZE;
    let mut image = vec![0u8; file_size];

    image[0..4].copy_from_slice(b"\x7FELF");
    image[4] = 2;
    image[5] = 1;
    image[6] = 1;
    image[16..18].copy_from_slice(&2u16.to_le_bytes());
    image[18..20].copy_from_slice(&machine.to_le_bytes());
    image[20..24].copy_from_slice(&1u32.to_le_bytes());
    image[24..32].copy_from_slice(&ENTRY_VADDR.to_le_bytes());
    image[32..40].copy_from_slice(&(ELF64_HEADER_BYTES as u64).to_le_bytes());
    image[40..48].copy_from_slice(&0u64.to_le_bytes());
    image[48..52].copy_from_slice(&0u32.to_le_bytes());
    image[52..54].copy_from_slice(&(ELF64_HEADER_BYTES as u16).to_le_bytes());
    image[54..56].copy_from_slice(&(ELF64_PROGRAM_HEADER_BYTES as u16).to_le_bytes());
    image[56..58].copy_from_slice(&1u16.to_le_bytes());
    image[58..60].copy_from_slice(&0u16.to_le_bytes());
    image[60..62].copy_from_slice(&0u16.to_le_bytes());
    image[62..64].copy_from_slice(&0u16.to_le_bytes());

    let ph = ELF64_HEADER_BYTES;
    image[ph..ph + 4].copy_from_slice(&PT_LOAD.to_le_bytes());
    image[ph + 4..ph + 8].copy_from_slice(&(PF_R | PF_X).to_le_bytes());
    image[ph + 8..ph + 16].copy_from_slice(&SEGMENT_OFFSET.to_le_bytes());
    image[ph + 16..ph + 24].copy_from_slice(&ENTRY_VADDR.to_le_bytes());
    image[ph + 24..ph + 32].copy_from_slice(&ENTRY_VADDR.to_le_bytes());
    image[ph + 32..ph + 40].copy_from_slice(&(SEGMENT_SIZE as u64).to_le_bytes());
    image[ph + 40..ph + 48].copy_from_slice(&(SEGMENT_SIZE as u64).to_le_bytes());
    image[ph + 48..ph + 56].copy_from_slice(&PAGE_ALIGN.to_le_bytes());

    image[SEGMENT_OFFSET as usize] = 0xC3;
    image
}

fn current_machine() -> Machine {
    #[cfg(target_arch = "x86_64")]
    {
        Machine::X86_64
    }

    #[cfg(target_arch = "aarch64")]
    {
        Machine::AArch64
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        panic!("unsupported target architecture for test fixture")
    }
}
