use anyhow::{Context, Result, bail};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

/// CPIO newc magic number.
const CPIO_MAGIC: &str = "070701";

/// Files that must always have the executable bit set.
const FORCE_EXECUTABLE: &[&str] = &[
    "init",
    "usr/bin/aether_init",
    "usr/bin/aethercore-diskfs-setup",
    "usr/bin/aethercore-pivot-root",
    "usr/bin/aethercore-apt-seed",
    "usr/bin/aethercore-userspace-abi-check",
    "usr/lib/aethercore/init",
    "usr/lib/aethercore/init.elf",
    "usr/lib/aethercore/probe.elf",
    "usr/lib/aethercore/probe-linked.elf",
    "usr/lib/aethercore/console.elf",
];

/// Required directories in the initramfs layout.
const REQUIRED_DIRS: &[&str] = &[
    "bin", "dev", "etc", "proc", "run", "sys", "tmp",
    "usr", "usr/bin", "usr/lib", "usr/lib/aethercore",
    "var", "var/log",
];

/// Build a cpio newc + gzip archive from a directory tree.
pub fn build(initramfs_dir: &Path, out_path: &Path) -> Result<()> {
    if !initramfs_dir.exists() {
        bail!("Initramfs source directory not found: {}", initramfs_dir.display());
    }

    validate_layout(initramfs_dir)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    let mut cpio_buf: Vec<u8> = Vec::new();
    let mut ino: u32 = 1;

    // Walk the directory tree in sorted order for deterministic output
    let mut entries: Vec<_> = WalkDir::new(initramfs_dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by(|a, b| a.path().cmp(b.path()));

    for entry in &entries {
        let abs_path = entry.path();
        let rel = abs_path.strip_prefix(initramfs_dir)
            .context("Failed to compute relative path")?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");

        if rel_str.is_empty() {
            continue;
        }

        let (mode, data) = if abs_path.is_dir() {
            (0o040755u32, Vec::new())
        } else if abs_path.is_file() {
            let exec = FORCE_EXECUTABLE.contains(&rel_str.as_str());
            let mode = if exec { 0o100755u32 } else { 0o100644u32 };
            let data = fs::read(abs_path)
                .with_context(|| format!("Failed to read: {}", abs_path.display()))?;
            (mode, data)
        } else {
            continue; // Skip symlinks and special files for now
        };

        append_cpio_entry(&mut cpio_buf, &rel_str, ino, mode, now, &data);
        ino += 1;
    }

    // Append trailer
    append_cpio_entry(&mut cpio_buf, "TRAILER!!!", ino, 0o100000, 0, &[]);

    // Write gzip-compressed output
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let out_file = fs::File::create(out_path)
        .with_context(|| format!("Failed to create: {}", out_path.display()))?;
    let mut gz = GzEncoder::new(out_file, Compression::best());
    gz.write_all(&cpio_buf)?;
    gz.finish()?;

    println!("[initramfs] Archive written: {} ({} entries, {} bytes uncompressed)",
        out_path.display(), ino - 1, cpio_buf.len());
    Ok(())
}

/// Validate the initramfs directory has the required structure.
fn validate_layout(dir: &Path) -> Result<()> {
    // Check /init exists
    let init = dir.join("init");
    if !init.exists() {
        bail!("Initramfs is missing /init: {}", init.display());
    }

    // Check required directories
    let missing: Vec<&str> = REQUIRED_DIRS.iter()
        .filter(|d| !dir.join(d).exists())
        .copied()
        .collect();
    if !missing.is_empty() {
        bail!("Initramfs is missing required directories: {}", missing.join(", "));
    }

    // Check /etc/profile
    if !dir.join("etc/profile").exists() {
        bail!("Initramfs is missing /etc/profile");
    }

    // Check early userspace binary
    let has_aether_init = dir.join("usr/bin/aether_init").exists();
    let has_sh = dir.join("bin/sh").exists();
    if !has_aether_init && !has_sh {
        bail!("Initramfs must provide /usr/bin/aether_init or /bin/sh for early userspace");
    }

    Ok(())
}

/// Append a single CPIO newc entry to the buffer.
fn append_cpio_entry(buf: &mut Vec<u8>, name: &str, ino: u32, mode: u32, mtime: u32, data: &[u8]) {
    let name_bytes = format!("{}\0", name);
    let name_len = name_bytes.len() as u32;
    let file_size = data.len() as u32;

    // Write header (110 bytes of ASCII hex fields)
    let header = format!(
        "{}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}",
        CPIO_MAGIC,
        ino,        // inode
        mode,       // mode
        0u32,       // uid
        0u32,       // gid
        1u32,       // nlink
        mtime,      // mtime
        file_size,  // filesize
        0u32,       // devmajor
        0u32,       // devminor
        0u32,       // rdevmajor
        0u32,       // rdevminor
        name_len,   // namesize
        0u32,       // checksum
    );

    buf.extend_from_slice(header.as_bytes());
    buf.extend_from_slice(name_bytes.as_bytes());

    // Pad name to 4-byte boundary
    let name_pad = (4 - (name_bytes.len() % 4)) % 4;
    buf.extend(std::iter::repeat(0u8).take(name_pad));

    // Write data
    if !data.is_empty() {
        buf.extend_from_slice(data);
        let data_pad = (4 - (data.len() % 4)) % 4;
        buf.extend(std::iter::repeat(0u8).take(data_pad));
    }
}
