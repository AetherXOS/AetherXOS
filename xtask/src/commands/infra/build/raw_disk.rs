use anyhow::{Result, anyhow};
use std::path::Path;
use std::process::Command;
use crate::utils::{logging, process, wsl};

/// Best-effort creation of a partitioned raw disk image containing an ext4 partition
/// populated with the contents of `src_dir`. This routine expects common Unix
/// host tooling (`qemu-img`, `parted`, `losetup`, `kpartx`, `mkfs.ext4`, `mount`, `umount`).
pub fn create_partitioned_raw_image_from_dir(src_dir: &Path, img_out: &Path) -> Result<()> {
    // If running on Windows, try a WSL-based fallback if available.
    if cfg!(windows) && process::which("wsl") {
        logging::info("build::image", "Windows detected; using WSL for partitioned image creation", &[]);

        let wsl_img = wsl::to_wsl_path(img_out)?;
        let wsl_src = wsl::to_wsl_path(src_dir)?;

        let cmd = format!(r#"
if [ ! -d '{}' ]; then
  echo "ERROR: Source directory not found in WSL: {}"
  exit 1
fi

echo "Calculating rootfs size..."
size_bytes=$(du -sb '{}' | cut -f1)
size_mb=$(( size_bytes / 1024 / 1024 + 256 ))
echo "Creating ${{size_mb}}MB raw image..."

qemu-img create -f raw '{}' ${{size_mb}}M 2>&1 | grep -v "^Formatting" || true
parted -s '{}' mklabel msdos mkpart primary ext4 1MiB 100% 2>&1 | grep -v "^Warning" || true

echo "Setting up loop device..."
losetup_dev=$(losetup --find --show '{}')
trap "losetup -d '${{losetup_dev}}' 2>/dev/null || true" EXIT
partprobe ${{losetup_dev}} || true

mapped="/dev/mapper/$$(basename ${{losetup_dev}})p1"
for i in {{1..10}}; do
  if [ -e "${{mapped}}" ]; then break; fi
  sleep 0.5
done

if [ ! -e "${{mapped}}" ]; then
  kpartx -a ${{losetup_dev}}
  sleep 1
fi

echo "Creating ext4 filesystem..."
mkfs.ext4 -F -q ${{mapped}}

echo "Mounting and copying rootfs..."
tmp=$$(mktemp -d)
trap "umount '${{tmp}}' 2>/dev/null; rmdir '${{tmp}}' 2>/dev/null; kpartx -d ${{losetup_dev}} 2>/dev/null; losetup -d '${{losetup_dev}}' 2>/dev/null || true" EXIT

mount ${{mapped}} ${{tmp}}
tar -C '{}' -cpf - . 2>/dev/null | tar -C ${{tmp}} -xpf - 
echo "Image creation complete."
"#, wsl_src, wsl_src, wsl_src, wsl_img, wsl_img, wsl_img, wsl_src);

        match wsl::run_in_wsl(&cmd, &["qemu-img", "parted", "losetup", "kpartx", "mkfs.ext4", "mount", "umount", "tar", "du"]) {
            Ok(_) => {
                logging::info("build::image", "Partitioned image created successfully in WSL", &[]);
                return Ok(());
            }
            Err(e) => {
                logging::warn("build::image", &format!("WSL image creation failed: {}; falling back to simple copy", e), &[]);
            }
        }
    }

    // Heuristic: estimate size
    let mut total: u64 = 0;
    for entry in walkdir::WalkDir::new(src_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(m) = entry.metadata() { total += m.len(); }
        }
    }
    let size_mb = ((total as f64) / 1024.0 / 1024.0).ceil() as u64 + 200;
    let size_spec = format!("{}M", size_mb);

    if process::first_available_binary(&["qemu-img", "qemu-img.exe"]).is_some() {
        process::run_checked("qemu-img", ["create", "-f", "raw", img_out.to_string_lossy().as_ref(), size_spec.as_str()])?;
    } else {
        let f = std::fs::File::create(img_out)?;
        f.set_len((size_mb as u64) * 1024 * 1024)?;
    }

    if !process::which("parted") { return Err(anyhow!("'parted' not found on host")); }
    if !process::which("losetup") { return Err(anyhow!("'losetup' not found on host")); }
    if !process::which("kpartx") { return Err(anyhow!("'kpartx' not found on host")); }
    if !process::which("mkfs.ext4") { return Err(anyhow!("'mkfs.ext4' not found on host")); }

    process::run_checked("parted", ["-s", img_out.to_string_lossy().as_ref(), "mklabel", "msdos"])?;
    process::run_checked("parted", ["-s", img_out.to_string_lossy().as_ref(), "mkpart", "primary", "ext4", "1MiB", "100%"])?;

    let losetup_out = Command::new("losetup").arg("--find").arg("--show").arg(&img_out).output()?;
    if !losetup_out.status.success() { return Err(anyhow!("losetup failed")); }
    let loop_dev = String::from_utf8_lossy(&losetup_out.stdout).trim().to_string();

    process::run_checked("partprobe", [loop_dev.as_str()])?;
    process::run_checked("kpartx", ["-a", loop_dev.as_str()])?;

    let loop_base = Path::new(&loop_dev).file_name().unwrap().to_string_lossy().into_owned();
    let mapped_part = format!("/dev/mapper/{}p1", loop_base);

    std::thread::sleep(std::time::Duration::from_millis(200));

    process::run_checked("mkfs.ext4", ["-F", mapped_part.as_str()])?;

    let tmp = tempfile::tempdir()?;
    let mount_point = tmp.path();
    process::run_checked("mount", [mapped_part.as_str(), mount_point.to_string_lossy().as_ref()])?;
    
    let tar_cmd = format!("tar -C '{}' -cf - .", src_dir.display());
    let extract_cmd = format!("tar -C '{}' -xpf -", mount_point.display());
    process::run_checked("sh", ["-c", format!("{} | {}", tar_cmd, extract_cmd).as_str()])?;
    process::run_checked("umount", [mount_point.to_string_lossy().as_ref()])?;

    process::run_checked("kpartx", ["-d", loop_dev.as_str()])?;
    process::run_checked("losetup", ["-d", loop_dev.as_str()])?;

    Ok(())
}
