#!/usr/bin/env bash
# ==============================================================================
# AetherCore Bootstrap Script: Debian + Flutter SDK
# Description: Provisions a Debian rootfs, packages Flutter as a .deb, 
#              and creates a bootable ext4 disk image for QEMU.
# ==============================================================================

set -euo pipefail

# --- Configuration ---
OUTDIR="${1:-}" 
FLUTTER_URL="${2:-}"
KERNEL_IMAGE="${3:-kernel.x}"
NO_BOOT_FLAG="${4:-}"
DISK_SIZE="8G" # 8GB Recommended for Flutter + Debian + Tools

# Directory logic: WORKDIR is placed outside OUTDIR to prevent rsync recursion
WORKDIR="$(dirname "$(realpath "$OUTDIR")")/hc_bootstrap_work"

# --- Usage Check ---
if [ -z "$OUTDIR" ] || [ -z "$FLUTTER_URL" ]; then
    echo "Error: Missing arguments." >&2
    echo "Usage: $0 <outdir> <flutter_url> [kernel_image] [--no-boot]" >&2
    exit 2
fi

mkdir -p "$WORKDIR" "$OUTDIR"

# --- Sudo Elevation ---
SUDO=""
[ "$(id -u)" -ne 0 ] && command -v sudo >/dev/null 2>&1 && SUDO="sudo"

# --- Cleanup Trap ---
# Ensures mounts are cleaned up even if the script fails midway
cleanup() {
    echo "==> Finalizing: Cleaning up mount points..."
    $SUDO umount "$OUTDIR/proc" "$OUTDIR/sys" "$OUTDIR/dev" 2>/dev/null || true
    if [ -d "${MNT:-}" ]; then
        $SUDO umount "$MNT" 2>/dev/null || true
        rm -rf "$MNT"
    fi
}
trap cleanup EXIT

# --- Helper: Chroot Execution ---
run_in_chroot() {
    local target=$1
    local cmd=$2
    echo "==> Executing in chroot: $cmd"
    $SUDO mount -t proc /proc "$target/proc" || true
    $SUDO mount -t sysfs /sys "$target/sys" || true
    $SUDO mount --bind /dev "$target/dev" || true
    [[ -f /etc/resolv.conf ]] && $SUDO cp /etc/resolv.conf "$target/etc/"
    
    # Run command and capture exit code
    set +e
    $SUDO chroot "$target" /bin/bash -c "$cmd"
    local exit_code=$?
    set -e

    $SUDO umount "$target/proc" || true
    $SUDO umount "$target/sys" || true
    $SUDO umount "$target/dev" || true
    return $exit_code
}

# 1. Debootstrap (Idempotent)
if [ ! -f "$OUTDIR/etc/debian_version" ]; then
    echo "==> Step 1: Provisioning Debian rootfs via debootstrap..."
    $SUDO debootstrap --variant=minbase --arch=amd64 stable "$OUTDIR" http://deb.debian.org/debian/
else
    echo "==> Step 1: Debian rootfs already exists. Skipping debootstrap."
fi

# 2. System Dependencies
echo "==> Step 2: Updating system and installing base dependencies..."
run_in_chroot "$OUTDIR" "apt-get update && apt-get install -y apt-transport-https ca-certificates curl gnupg libglu1-mesa"

# 3. Flutter SDK Packaging
FLUTTER_TAR="$WORKDIR/flutter.tar.xz"
FLUTTER_DEB="$WORKDIR/flutter-sdk_1.0.0_amd64.deb"

if [ ! -f "$FLUTTER_DEB" ]; then
    echo "==> Step 3: Packaging Flutter SDK into a .deb file..."
    if [ ! -f "$FLUTTER_TAR" ]; then
        echo "    Downloading Flutter SDK..."
        curl -L -o "$FLUTTER_TAR" "$FLUTTER_URL"
    fi
    
    DEB_ROOT="$WORKDIR/debroot"
    rm -rf "$DEB_ROOT" && mkdir -p "$DEB_ROOT/opt" "$DEB_ROOT/usr/bin" "$DEB_ROOT/DEBIAN"
    
    echo "    Extracting SDK..."
    tar -xf "$FLUTTER_TAR" -C "$DEB_ROOT/opt"
    
    echo -e '#!/usr/bin/env bash\nexec /opt/flutter/bin/flutter "$@"' > "$DEB_ROOT/usr/bin/flutter"
    chmod +x "$DEB_ROOT/usr/bin/flutter"
    
    cat > "$DEB_ROOT/DEBIAN/control" <<EOF
Package: flutter-sdk
Version: 1.0.0
Section: utils
Priority: optional
Architecture: amd64
Maintainer: aethercore <dev@local>
Description: Flutter SDK packaged for AetherCore/VelOS
EOF
    fakeroot dpkg-deb --build "$DEB_ROOT" "$FLUTTER_DEB"
    rm -rf "$DEB_ROOT"
else
    echo "==> Step 3: Flutter .deb package already exists. Skipping."
fi

# 4. Flutter Installation in Chroot
if ! run_in_chroot "$OUTDIR" "dpkg -l | grep -q flutter-sdk"; then
    echo "==> Step 4: Installing Flutter SDK into the rootfs..."
    $SUDO cp "$FLUTTER_DEB" "$OUTDIR/tmp/f.deb"
    run_in_chroot "$OUTDIR" "dpkg -i /tmp/f.deb || apt-get -f install -y; rm /tmp/f.deb"
else
    echo "==> Step 4: Flutter SDK is already installed in the rootfs."
fi

# 5. Disk Image Creation
IMG_OUT="$WORKDIR/rootfs.img"
echo "==> Step 5: Generating ext4 disk image ($DISK_SIZE)..."

# Use fallocate for instant sparse file creation (much faster than dd)
$SUDO fallocate -l "$DISK_SIZE" "$IMG_OUT" || $SUDO dd if=/dev/zero of="$IMG_OUT" bs=1M count=8192

$SUDO mkfs.ext4 -F "$IMG_OUT"
LOOP=$($SUDO losetup --find --show "$IMG_OUT")
MNT=$(mktemp -d)

$SUDO mount "$LOOP" "$MNT"
echo "    Synchronizing files to disk..."
# Exclude work directories and temporary files to prevent "No space left" errors
$SUDO rsync -aAX --numeric-ids --delete \
    --exclude='/hc_bootstrap_work' \
    --exclude='/work' \
    --exclude='/tmp/*' \
    --exclude='/proc/*' \
    --exclude='/sys/*' \
    --exclude='/dev/*' \
    "$OUTDIR/" "$MNT/"

$SUDO umount "$MNT"
$SUDO losetup -d "$LOOP"
rm -rf "$MNT"

# 6. Finalization & Boot
if [ "${SKIP_BOOT:-0}" = "1" ] || [ "$NO_BOOT_FLAG" = "--no-boot" ] ; then
    echo "----------------------------------------------------------------"
    echo "SUCCESS: Disk image created at: $IMG_OUT"
    echo "You can now boot this with QEMU manually."
    echo "----------------------------------------------------------------"
    exit 0
fi

echo "==> Step 6: Launching QEMU Virtual Machine..."
qemu-system-x86_64 -m 4096 -kernel "$KERNEL_IMAGE" \
    -append "root=/dev/vda rw console=tty0 rootwait" \
    -drive file="$IMG_OUT",if=virtio,format=raw \
    -display sdl -vga virtio