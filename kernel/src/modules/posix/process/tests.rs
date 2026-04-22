use super::*;

fn machine_code() -> u16 {
    #[cfg(target_arch = "x86_64")]
    {
        62
    }
    #[cfg(target_arch = "aarch64")]
    {
        183
    }
}

fn elf64_with_interp(interp_bytes: &[u8]) -> alloc::vec::Vec<u8> {
    const ELF_HEADER_SIZE: usize = 64;
    const PHDR_SIZE: usize = 56;
    const PHDR_OFFSET: usize = ELF_HEADER_SIZE;
    const INTERP_OFFSET: usize = ELF_HEADER_SIZE + PHDR_SIZE;

    let mut image = alloc::vec![0u8; INTERP_OFFSET + interp_bytes.len()];
    image[0..4].copy_from_slice(b"\x7FELF");
    image[4] = 2;
    image[5] = 1;
    image[6] = 1;
    image[16..18].copy_from_slice(&2u16.to_le_bytes());
    image[18..20].copy_from_slice(&machine_code().to_le_bytes());
    image[20..24].copy_from_slice(&1u32.to_le_bytes());
    image[32..40].copy_from_slice(&(PHDR_OFFSET as u64).to_le_bytes());
    image[52..54].copy_from_slice(&(ELF_HEADER_SIZE as u16).to_le_bytes());
    image[54..56].copy_from_slice(&(PHDR_SIZE as u16).to_le_bytes());
    image[56..58].copy_from_slice(&1u16.to_le_bytes());

    image[PHDR_OFFSET..PHDR_OFFSET + 4].copy_from_slice(&3u32.to_le_bytes());
    image[PHDR_OFFSET + 8..PHDR_OFFSET + 16].copy_from_slice(&(INTERP_OFFSET as u64).to_le_bytes());
    image[PHDR_OFFSET + 32..PHDR_OFFSET + 40].copy_from_slice(&(interp_bytes.len() as u64).to_le_bytes());
    image[PHDR_OFFSET + 40..PHDR_OFFSET + 48].copy_from_slice(&(interp_bytes.len() as u64).to_le_bytes());
    image[PHDR_OFFSET + 48..PHDR_OFFSET + 56].copy_from_slice(&1u64.to_le_bytes());

    image[INTERP_OFFSET..INTERP_OFFSET + interp_bytes.len()].copy_from_slice(interp_bytes);
    image
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
#[test_case]
fn resolve_interp_path_accepts_absolute_interp() {
    let image = elf64_with_interp(b"/lib/ld-linux-x86-64.so.2\0");
    let interp = resolve_interp_path(&image).expect("resolve interp");
    assert_eq!(interp.as_deref(), Some("/lib/ld-linux-x86-64.so.2"));
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
#[test_case]
fn resolve_interp_path_accepts_lib64_glibc_loader() {
    let image = elf64_with_interp(b"/lib64/ld-linux-x86-64.so.2\0");
    let interp = resolve_interp_path(&image).expect("resolve interp");
    assert_eq!(interp.as_deref(), Some("/lib64/ld-linux-x86-64.so.2"));
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
#[test_case]
fn resolve_interp_path_rejects_oversized_interp_segment() {
    let image = elf64_with_interp(&alloc::vec![b'a'; 4097]);
    assert_eq!(resolve_interp_path(&image), Err(PosixErrno::Invalid));
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
#[test_case]
fn resolve_interp_path_rejects_non_nul_terminated_interp() {
    let image = elf64_with_interp(b"/lib/ld-linux-x86-64.so.2");
    assert_eq!(resolve_interp_path(&image), Err(PosixErrno::Invalid));
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
#[test_case]
fn resolve_interp_path_rejects_non_zero_padding_after_nul() {
    let image = elf64_with_interp(b"/lib/ld.so\0garbage");
    assert_eq!(resolve_interp_path(&image), Err(PosixErrno::Invalid));
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
#[test_case]
fn resolve_interp_path_rejects_non_loader_interp() {
    let image = elf64_with_interp(b"/tmp/custom-loader.so\0");
    assert_eq!(resolve_interp_path(&image), Err(PosixErrno::Invalid));
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
#[test_case]
fn resolve_interp_path_allows_non_system_loader_when_policy_relaxed() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_exec_elf_enforce_system_loader_paths(Some(false));

    let image = elf64_with_interp(b"/tmp/custom-loader.so\0");
    let interp = resolve_interp_path(&image).expect("resolve interp");
    assert_eq!(interp.as_deref(), Some("/tmp/custom-loader.so"));

    crate::config::KernelConfig::reset_runtime_overrides();
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
#[test_case]
fn resolve_interp_path_rejects_parent_traversal_segments() {
    let image = elf64_with_interp(b"/lib/../ld-linux-x86-64.so.2\0");
    assert_eq!(resolve_interp_path(&image), Err(PosixErrno::Invalid));
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
#[test_case]
fn resolve_interp_path_allows_relative_interp_when_absolute_policy_relaxed() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_exec_elf_require_absolute_interp_path(Some(false));
    crate::config::KernelConfig::set_exec_elf_enforce_system_loader_paths(Some(false));

    let image = elf64_with_interp(b"ld-linux-x86-64.so.2\0");
    let interp = resolve_interp_path(&image).expect("resolve interp");
    assert_eq!(interp.as_deref(), Some("ld-linux-x86-64.so.2"));

    crate::config::KernelConfig::reset_runtime_overrides();
}

#[cfg(all(feature = "process_abstraction", feature = "vfs", feature = "posix_fs"))]
#[test_case]
fn posix_spawn_from_image_rejects_relative_interp_path() {
    let image = elf64_with_interp(b"ld-linux-x86-64.so.2\0");
    assert_eq!(
        posix_spawn_from_image(b"app", &image, 10, 0, 0, 0),
        Err(PosixErrno::Invalid)
    );
}
