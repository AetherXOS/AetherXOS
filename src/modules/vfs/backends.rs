use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum FsBackendKind {
    RamFs = 1,
    FatFs = 2,
    LittleFs = 3,
    Ext4 = 4,
    SquashFs = 5,
}

#[derive(Debug, Clone, Copy)]
pub struct BackendMatrix {
    pub fatfs_enabled: bool,
    pub littlefs_enabled: bool,
    pub ext4_enabled: bool,
    pub squashfs_enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct BackendProbeStats {
    pub fatfs_probe_calls: u64,
    pub littlefs_probe_calls: u64,
    pub ext4_probe_calls: u64,
    pub squashfs_probe_calls: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct BackendDescriptor {
    pub kind: FsBackendKind,
    pub name: &'static str,
    pub enabled: bool,
}

static FATFS_PROBE_CALLS: AtomicU64 = AtomicU64::new(0);
static LITTLEFS_PROBE_CALLS: AtomicU64 = AtomicU64::new(0);
static EXT4_PROBE_CALLS: AtomicU64 = AtomicU64::new(0);
static SQUASHFS_PROBE_CALLS: AtomicU64 = AtomicU64::new(0);

pub fn supported_backends() -> BackendMatrix {
    BackendMatrix {
        fatfs_enabled: cfg!(feature = "vfs_fatfs"),
        littlefs_enabled: cfg!(feature = "vfs_littlefs"),
        ext4_enabled: cfg!(feature = "vfs_ext4"),
        squashfs_enabled: cfg!(feature = "vfs_squashfs"),
    }
}

pub fn probe_backend(kind: FsBackendKind) -> bool {
    match kind {
        FsBackendKind::RamFs => true,
        FsBackendKind::FatFs => probe_fatfs(),
        FsBackendKind::LittleFs => probe_littlefs(),
        FsBackendKind::Ext4 => probe_ext4(),
        FsBackendKind::SquashFs => probe_squashfs(),
    }
}

#[cfg(all(feature = "vfs_fatfs", not(target_os = "none")))]
fn probe_fatfs() -> bool {
    FATFS_PROBE_CALLS.fetch_add(1, Ordering::Relaxed);
    let _ = fatfs::FsOptions::new();
    true
}

#[cfg(any(not(feature = "vfs_fatfs"), target_os = "none"))]
fn probe_fatfs() -> bool {
    FATFS_PROBE_CALLS.fetch_add(1, Ordering::Relaxed);
    false
}

#[cfg(feature = "vfs_littlefs")]
fn probe_littlefs() -> bool {
    LITTLEFS_PROBE_CALLS.fetch_add(1, Ordering::Relaxed);
    let _ = core::mem::size_of::<littlefs2_core::Metadata>();
    true
}

#[cfg(not(feature = "vfs_littlefs"))]
fn probe_littlefs() -> bool {
    LITTLEFS_PROBE_CALLS.fetch_add(1, Ordering::Relaxed);
    false
}

#[cfg(feature = "vfs_ext4")]
fn probe_ext4() -> bool {
    EXT4_PROBE_CALLS.fetch_add(1, Ordering::Relaxed);
    let _ = core::mem::size_of::<ext4_view::Ext4>();
    true
}

#[cfg(not(feature = "vfs_ext4"))]
fn probe_ext4() -> bool {
    EXT4_PROBE_CALLS.fetch_add(1, Ordering::Relaxed);
    false
}

#[cfg(feature = "vfs_squashfs")]
fn probe_squashfs() -> bool {
    SQUASHFS_PROBE_CALLS.fetch_add(1, Ordering::Relaxed);
    true
}

#[cfg(not(feature = "vfs_squashfs"))]
fn probe_squashfs() -> bool {
    SQUASHFS_PROBE_CALLS.fetch_add(1, Ordering::Relaxed);
    false
}

pub fn backend_probe_stats() -> BackendProbeStats {
    BackendProbeStats {
        fatfs_probe_calls: FATFS_PROBE_CALLS.load(Ordering::Relaxed),
        littlefs_probe_calls: LITTLEFS_PROBE_CALLS.load(Ordering::Relaxed),
        ext4_probe_calls: EXT4_PROBE_CALLS.load(Ordering::Relaxed),
        squashfs_probe_calls: SQUASHFS_PROBE_CALLS.load(Ordering::Relaxed),
    }
}

pub fn backend_inventory() -> [BackendDescriptor; 5] {
    let support = supported_backends();
    [
        BackendDescriptor {
            kind: FsBackendKind::RamFs,
            name: "RamFs",
            enabled: true,
        },
        BackendDescriptor {
            kind: FsBackendKind::FatFs,
            name: "FatFs",
            enabled: support.fatfs_enabled,
        },
        BackendDescriptor {
            kind: FsBackendKind::LittleFs,
            name: "LittleFs",
            enabled: support.littlefs_enabled,
        },
        BackendDescriptor {
            kind: FsBackendKind::Ext4,
            name: "Ext4",
            enabled: support.ext4_enabled,
        },
        BackendDescriptor {
            kind: FsBackendKind::SquashFs,
            name: "SquashFs",
            enabled: support.squashfs_enabled,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn backend_inventory_contains_all_known_kinds() {
        let inv = backend_inventory();
        assert_eq!(inv.len(), 5);
        assert!(inv.iter().any(|d| d.kind == FsBackendKind::RamFs));
        assert!(inv.iter().any(|d| d.kind == FsBackendKind::FatFs));
        assert!(inv.iter().any(|d| d.kind == FsBackendKind::LittleFs));
        assert!(inv.iter().any(|d| d.kind == FsBackendKind::Ext4));
        assert!(inv.iter().any(|d| d.kind == FsBackendKind::SquashFs));
    }

    #[test_case]
    fn backend_probe_stats_increments_on_probe() {
        let before = backend_probe_stats();
        let _ = probe_backend(FsBackendKind::FatFs);
        let after = backend_probe_stats();
        assert_eq!(after.fatfs_probe_calls, before.fatfs_probe_calls + 1);
    }
}
