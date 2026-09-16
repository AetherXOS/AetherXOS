# 🐧 AetherCore: Guest OS & Distribution Support Guide

This comprehensive guide covers running your custom AetherCore kernel with full Linux distributions using `xtask`. Whether you're a beginner or advanced user, you'll find step-by-step instructions for every scenario.

**New:** Distribution metadata is now managed in [distro-registry.json](distro-registry.json). See [DISTRO_REGISTRY_GUIDE.md](DISTRO_REGISTRY_GUIDE.md) for how to add your own distributions without code changes!

---

## Quick Start (5 minutes)

### Option 1: Download & Run Ubuntu (Automated)

```bash
# Download Ubuntu 24.04 and launch interactive session
cargo run -p xtask -- run guest --distro ubuntu-24.04 --download
```

**That's it!** xtask will:
- ✓ Detect your OS and download Ubuntu
- ✓ Cache the download for future runs
- ✓ Build your kernel with the rootfs
- ✓ Launch QEMU interactively

### Option 2: Use Your Own Rootfs (Local File)

```bash
# Use a local tarball
cargo run -p xtask -- run guest --rootfs /path/to/my-rootfs.tar.gz
```

### Option 3: Direct URL

```bash
# Provide any direct download URL
cargo run -p xtask -- run guest --distro "https://example.com/custom-rootfs.tar.gz" --download
```

---

## Supported Distributions

xtask includes built-in URLs for these major distributions:

| Distro | Keys | Status |
|--------|------|--------|
| **Ubuntu** | `ubuntu`, `ubuntu-24.04`, `ubuntu-22.04`, `ubuntu-20.04`, `ubuntu-lts` | ✓ Tested |
| **Debian** | `debian`, `debian-12`, `debian-11` | ✓ Tested |
| **Fedora** | `fedora`, `fedora-40`, `fedora-39` | ✓ Cloud images |
| **CentOS/RHEL** | `centos-stream-9`, `centos-stream-8`, `almalinux-9`, `almalinux-8` | ✓ Enterprise |
| **Alpine** | `alpine`, `alpine-3.19`, `alpine-3.18` | ✓ Minimal |
| **Arch Linux** | `arch` | ✓ Rolling |
| **openSUSE** | `opensuse-leap-15`, `opensuse-tumbleweed` | ✓ Alternative |

---

## Full Command Reference

### Run a Guest

```bash
cargo run -p xtask -- run guest [OPTIONS]
```

**Options:**

| Flag | Purpose | Example |
|------|---------|---------|
| `--distro <KEY>` | Select distribution | `--distro ubuntu-24.04` |
| `--rootfs <PATH>` | Use local rootfs file | `--rootfs ~/my-rootfs.tar.gz` |
| `--download` | Download from registry | `--download` |
| `--cache` | Use cached downloads | (default: yes) |
| `--refresh` | Re-download even if cached | `--refresh` |
| `--attach` | Explicitly attach rootfs disk | `--attach` |
| `--firmware <TYPE>` | Firmware type | `--firmware bios` |

**Priority order:**
1. `--rootfs` (explicit local file)
2. `--distro` + cache (if already downloaded)
3. `--distro` + `--download` (fetch from registry)
4. Kernel-only boot (no userspace)

### Examples

```bash
# Example 1: Ubuntu LTS with auto-download
cargo run -p xtask -- run guest --distro ubuntu-lts --download

# Example 2: Use cached Fedora (must have downloaded once)
cargo run -p xtask -- run guest --distro fedora-40

# Example 3: Alpine with refresh (re-download even if cached)
cargo run -p xtask -- run guest --distro alpine --download --refresh

# Example 4: Local tarball
cargo run -p xtask -- run guest --rootfs /home/user/debian-12.tar.gz

# Example 5: Custom URL
cargo run -p xtask -- run guest --distro https://mirrors.example.com/rootfs.tar.gz --download

# Example 6: BIOS firmware (not UEFI)
cargo run -p xtask -- run guest --distro ubuntu-22.04 --download --firmware bios

# Example 7: Skip cache, always download fresh
cargo run -p xtask -- run guest --distro debian --download --refresh --no-cache
```

---

## Step-by-Step Tutorials

### Tutorial 1: Run Ubuntu for the First Time

**Goal:** Boot your kernel with a full Ubuntu system.

**Steps:**

1. Open terminal in your OS repo:
   ```bash
   cd ~/Desktop/OS
   ```

2. Run xtask with download flag:
   ```bash
   cargo run -p xtask -- run guest --distro ubuntu-24.04 --download
   ```

3. Wait for download and build (first time: ~5-10 minutes depending on internet):
   ```
   📥 Resolving distro URLs for: ubuntu-24.04
   Trying URL [1/2]: https://cloud-images.ubuntu.com/...
   [████████████████████] 500 MB downloaded
   ✓ Downloaded ubuntu-24.04 (500 MB)
   Building kernel and boot image...
   🚀 Launching QEMU...
   ```

4. QEMU window appears. You're now running AetherCore kernel + Ubuntu!

5. Exit QEMU:
   - Press `Ctrl+A` then `X`
   - Or close the QEMU window

**Tips:**
- First run downloads and caches. Second run uses cache (instant).
- Check `artifacts/boot_image/` for generated boot images.
- Check `artifacts/guest_cache/` for cached downloads.

---

### Tutorial 2: Switch Between Distributions

**Goal:** Try different Linux distributions with the same kernel.

**Scenario:** You want to test kernel compatibility across distros.

```bash
# Try Ubuntu
cargo run -p xtask -- run guest --distro ubuntu-24.04 --download
# Ctrl-A X to exit

# Try Fedora
cargo run -p xtask -- run guest --distro fedora-40 --download
# Ctrl-A X to exit

# Try Debian (uses cache from before if available)
cargo run -p xtask -- run guest --distro debian-12 --download
```

**Result:** Each distro tests your kernel with different userspace tools, packages, and configurations.

---

### Tutorial 3: Create Your Own Custom Rootfs

**Goal:** Package your own rootfs and use it with AetherCore.

**Steps:**

1. Prepare your rootfs directory structure:
   ```bash
   mkdir -p ~/my-custom-rootfs/{bin,sbin,etc,usr,var,proc,sys,dev,home,tmp}
   ```

2. Add essential binaries and configs:
   ```bash
   # Copy or create essential files
   # (This depends on your custom needs)
   ```

3. Create a tarball:
   ```bash
   cd ~/my-custom-rootfs
   tar -czf ~/my-rootfs.tar.gz .
   ```

4. Run with xtask:
   ```bash
   cargo run -p xtask -- run guest --rootfs ~/my-rootfs.tar.gz
   ```

---

### Tutorial 4: Pre-Download and Cache Distros

**Goal:** Prepare multiple distros offline, then run without internet.

**Steps:**

1. Download distros ahead of time:
   ```bash
   cargo run -p xtask -- run guest --distro ubuntu-24.04 --download
   # Wait for download to complete, then exit QEMU
   
   cargo run -p xtask -- run guest --distro fedora-40 --download
   # Wait, then exit
   
   cargo run -p xtask -- run guest --distro debian-12 --download
   # Done
   ```

2. Check cache:
   ```bash
   ls -lh artifacts/guest_cache/
   ```

3. Now run offline (no internet needed):
   ```bash
   cargo run -p xtask -- run guest --distro ubuntu-24.04
   # Uses cache instantly, no download
   ```

---

## Platform-Specific Notes

### Linux Hosts

✓ Full support for all features.

```bash
# Automatic partitioned disk image creation
cargo run -p xtask -- run guest --distro ubuntu-24.04 --download

# Your kernel boots with full rootfs on /dev/vda1
```

**Requirements:** `qemu-img`, `parted`, `losetup`, `kpartx`, `mkfs.ext4` (usually pre-installed or via package manager).

---

### macOS Hosts

✓ Works with QEMU/brew installation.

```bash
# Install QEMU if needed
brew install qemu

# Run guest
cargo run -p xtask -- run guest --distro ubuntu-24.04 --download
```

**Note:** Partitioned image creation may require additional tools (`parted`, `kpartx`). If not available, xtask falls back to staged-only mode (kernel boots but rootfs is limited to initramfs).

---

### Windows Hosts (WSL2)

✓ Full support via WSL2 integration.

```bash
# Ensure WSL2 is installed and has Linux distro
# Then run as normal
cargo run -p xtask -- run guest --distro ubuntu-24.04 --download
```

**What happens:**
1. xtask detects Windows
2. Uses WSL2 to execute partitioned image creation
3. Automatically translates paths (C:\... → /mnt/c/...)
4. Returns to Windows, launches QEMU

**Windows-specific requirements:**
- WSL2 with a Linux distro (Ubuntu, Debian, etc.)
- Inside WSL2: `qemu-img`, `parted`, `losetup`, `kpartx`, `mkfs.ext4` (one-time setup)

**Setup on WSL2:**
```bash
# Inside WSL2 terminal
wsl.exe
sudo apt-get update
sudo apt-get install -y qemu-utils parted kpartx e2fsprogs

# Verify
which qemu-img losetup kpartx mkfs.ext4
```

After setup, use normally:
```bash
cargo run -p xtask -- run guest --distro ubuntu-24.04 --download
```

---

## Cache Management

### View Cached Distros

```bash
ls -lh artifacts/guest_cache/
```

Output example:
```
ubuntu-24.04.tar.gz    (512 MB)
fedora-40.tar.gz       (700 MB)
debian-12.tar.gz       (450 MB)
```

### Clear Cache

```bash
# Clear specific distro
rm artifacts/guest_cache/ubuntu-24.04.tar.gz

# Clear all caches
rm -rf artifacts/guest_cache/
```

### Force Re-Download (even if cached)

```bash
cargo run -p xtask -- run guest --distro ubuntu-24.04 --download --refresh
```

---

## Troubleshooting

### Issue: "Download failed for all attempted URLs"

**Cause:** Internet connection or curl/wget not available.

**Solutions:**
1. Check internet: `ping 8.8.8.8`
2. Install curl/wget:
   ```bash
   # Ubuntu/Debian
   sudo apt install curl wget
   # macOS
   brew install curl wget
   # Windows (download wget from gnuwin32)
   ```
3. Use local file instead:
   ```bash
   cargo run -p xtask -- run guest --rootfs ~/my-existing-rootfs.tar.gz
   ```

---

### Issue: "No known URLs for distro 'xyz'"

**Cause:** Distro key not recognized.

**Solutions:**
1. Use a supported distro key (see list above)
2. Provide a direct URL:
   ```bash
   cargo run -p xtask -- run guest --distro "https://example.com/my-rootfs.tar.gz" --download
   ```
3. Use local file:
   ```bash
   cargo run -p xtask -- run guest --rootfs ~/my-rootfs.tar.gz
   ```

---

### Issue: "Partitioned image creation failed" (Windows)

**Cause:** WSL2 not available or missing required tools.

**Solutions:**
1. Install WSL2:
   ```powershell
   wsl --install
   ```
2. Install tools in WSL2:
   ```bash
   wsl.exe
   sudo apt-get install -y qemu-utils parted kpartx e2fsprogs
   ```
3. Or provide pre-made image:
   ```bash
   # Place pre-created image at
   cp /path/to/prepared.img artifacts/aethercore-rootfs.img
   cargo run -p xtask -- run guest --attach
   ```

---

### Issue: "QEMU exited unexpectedly"

**Cause:** Missing QEMU or kernel panic.

**Solutions:**
1. Verify QEMU installed:
   ```bash
   qemu-system-x86_64 --version
   ```
2. Check kernel logs (QEMU output before exit)
3. Try simpler distro: `alpine` is minimal and good for debugging
4. Build without rootfs to isolate issue:
   ```bash
   cargo run -p xtask -- build full --arch x86_64 --bootloader limine --format iso --release
   cargo run -p xtask -- run smoke
   ```

---

### Issue: QEMU not starting / black screen

**Cause:** Possible race condition or initialization issue.

**Solutions:**
1. Give QEMU more time (wait 30 seconds)
2. Check if QEMU window is behind other windows
3. Use `--no-cache` to force fresh build:
   ```bash
   cargo run -p xtask -- run guest --distro ubuntu-24.04 --download --refresh
   ```
4. Check disk space: `df -h` (need at least 5GB free)

---

## Advanced Usage

### Use Different Architectures

(When supported by kernel and qemu):
```bash
# ARM64 (if kernel supports)
cargo run -p xtask -- run guest --distro ubuntu-24.04 --download --arch arm64
```

### Custom Firmware

```bash
# BIOS mode (legacy)
cargo run -p xtask -- run guest --distro ubuntu-24.04 --firmware bios

# UEFI mode (default)
cargo run -p xtask -- run guest --distro ubuntu-24.04 --firmware uefi
```

### Multiple Storage Backends

```bash
# Use ISO (default)
cargo run -p xtask -- build full --rootfs ~/rootfs.tar.gz --format iso
cargo run -p xtask -- run smoke

# Use raw disk image
cargo run -p xtask -- build full --rootfs ~/rootfs.tar.gz --format raw
```

---

## Performance Tips

1. **Cache distros locally** (avoid repeated downloads)
2. **Use Alpine for testing** (smallest, fastest)
3. **Pre-allocate QEMU memory**:
   ```bash
   # Modify qemu.rs or set via env
   export QEMU_MEM=2048
   ```
4. **Use `-release` flag for optimized kernel**:
   ```bash
   cargo run -p xtask -- build full --release --rootfs ~/rootfs.tar.gz
   ```

---

## Manual Image Creation (Advanced)

If xtask's automatic creation fails, create the image manually:

### Linux/macOS

```bash
# 1. Create sparse raw file
qemu-img create -f raw artifacts/aethercore-rootfs.img 2048M

# 2. Partition with parted
parted -s artifacts/aethercore-rootfs.img mklabel msdos mkpart primary ext4 1MiB 100%

# 3. Set up loop device
LOOP=$(losetup --find --show artifacts/aethercore-rootfs.img)
kpartx -a $LOOP
PART=/dev/mapper/$(basename $LOOP)p1

# 4. Create filesystem
mkfs.ext4 -F $PART

# 5. Mount and copy
mkdir -p /mnt/tmproot
mount $PART /mnt/tmproot
tar -C ~/my-rootfs -cpf - . | tar -C /mnt/tmproot -xpf -

# 6. Unmount and clean up
umount /mnt/tmproot
kpartx -d $LOOP
losetup -d $LOOP

# 7. Run with xtask
cargo run -p xtask -- run guest --attach
```

### Windows (in WSL2)

```bash
# Same as Linux, but run inside WSL2:
wsl.exe
# ... run the Linux commands above ...
```

---

## FAQ

**Q: Does my kernel need to support the distro?**
A: Your kernel boots the distro's init system. Some kernels may lack drivers or features certain distros expect. Test with the distro you target in production.

**Q: Can I modify the distro before booting?**
A: Yes, extract the tarball, modify files, re-tar, then use `--rootfs /path/to/modified.tar.gz`.

**Q: How much disk space do I need?**
A: Roughly 2x the distro size (for download cache + image). Ubuntu: ~1GB cache, Fedora: ~1.5GB, etc.

**Q: Can I run multiple guests simultaneously?**
A: Yes, but each needs its own image file and QEMU instance. Run from different terminals.

**Q: Does this work with other hypervisors (Hyper-V, VMware)?**
A: xtask currently targets QEMU. Adapting to other hypervisors would require additional code.

**Q: How do I share files between host and guest?**
A: QEMU supports 9pfs mounts. Kernel and distro must support this; add via boot parameters.

---

## Getting Help

- Check logs: `artifacts/build_log.txt`, `artifacts/check_output.txt`
- Enable verbose logging: (add to xtask if needed)
- Ask in repo issues with output of `cargo run -p xtask -- run guest --distro ubuntu-24.04`

---

## Next Steps

- **Automate CI:** Add GitHub Actions to test multiple distros
- **Customize distros:** Create your own minimal rootfs
- **Measure performance:** Benchmark kernel syscalls across distros
- **Add drivers:** Expand kernel support for specialized distros

Enjoy running your AetherCore kernel! 🎉
