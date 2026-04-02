use crate::constants;

pub fn kernel_boot_args(
    memory_mb: u32,
    cores: u32,
    kernel: &str,
    initramfs: &str,
    append: &str,
    nographic: bool,
) -> Vec<String> {
    let mut args = Vec::with_capacity(12);
    if nographic {
        args.push("-nographic".to_string());
    }
    args.extend([
        "-m".to_string(),
        memory_mb.to_string(),
        "-smp".to_string(),
        cores.to_string(),
        "-kernel".to_string(),
        kernel.to_string(),
        "-initrd".to_string(),
        initramfs.to_string(),
        "-append".to_string(),
        append.to_string(),
    ]);
    args
}

pub fn iso_boot_args(memory_mb: u32, cores: u32, iso: &str, nographic: bool) -> Vec<String> {
    let mut args = Vec::with_capacity(10);
    if nographic {
        args.push("-nographic".to_string());
    }
    args.extend([
        "-m".to_string(),
        memory_mb.to_string(),
        "-smp".to_string(),
        cores.to_string(),
        "-cdrom".to_string(),
        iso.to_string(),
        "-boot".to_string(),
        "d".to_string(),
    ]);
    args
}

pub fn smoke_timeout_sec() -> u64 {
    std::env::var("AETHERCORE_QEMU_SMOKE_TIMEOUT_SEC")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(constants::defaults::run::QEMU_SMOKE_TIMEOUT_SEC)
}

#[cfg(test)]
mod tests {
    use super::{iso_boot_args, kernel_boot_args};

    #[test]
    fn kernel_boot_args_contains_expected_segments() {
        let args = kernel_boot_args(
            512,
            2,
            "kernel.elf",
            "initramfs.cpio",
            "console=ttyS0",
            true,
        );
        assert_eq!(args.first().map(|s| s.as_str()), Some("-nographic"));
        assert!(args.windows(2).any(|w| w == ["-m", "512"]));
        assert!(args.windows(2).any(|w| w == ["-smp", "2"]));
        assert!(args.windows(2).any(|w| w == ["-kernel", "kernel.elf"]));
        assert!(args.windows(2).any(|w| w == ["-initrd", "initramfs.cpio"]));
        assert!(args.windows(2).any(|w| w == ["-append", "console=ttyS0"]));
    }

    #[test]
    fn iso_boot_args_contains_expected_segments() {
        let args = iso_boot_args(1024, 4, "boot.iso", false);
        assert!(!args.iter().any(|s| s == "-nographic"));
        assert!(args.windows(2).any(|w| w == ["-m", "1024"]));
        assert!(args.windows(2).any(|w| w == ["-smp", "4"]));
        assert!(args.windows(2).any(|w| w == ["-cdrom", "boot.iso"]));
        assert!(args.windows(2).any(|w| w == ["-boot", "d"]));
    }
}
