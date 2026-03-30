#!/usr/bin/env bash
set -euo pipefail

# bootstrap_flutter.sh <outdir> <flutter_url> [kernel_image] [--no-boot]
OUTDIR="${1:-}" 
FLUTTER_URL="${2:-}"
KERNEL_IMAGE="${3:-kernel.x}"
# Optional fourth argument or env var SKIP_BOOT=1 will prevent this script from launching QEMU.
NO_BOOT_FLAG="${4:-}"

if [ -z "$OUTDIR" ] || [ -z "$FLUTTER_URL" ]; then
  echo "Usage: $0 <outdir> <flutter_url> [kernel_image]" >&2
  exit 2
fi

WORKDIR="$OUTDIR/work"
mkdir -p "$WORKDIR"

SUDO=""
if [ "$(id -u)" -ne 0 ]; then
  if command -v sudo >/dev/null 2>&1; then
    SUDO="sudo"
  else
    echo "Warning: not running as root and sudo not available; some operations may fail" >&2
  fi
fi

echo "==> debootstrap rootfs (requires debootstrap)"
if command -v debootstrap >/dev/null 2>&1; then
  $SUDO debootstrap --variant=minbase --arch=amd64 stable "$OUTDIR" http://deb.debian.org/debian/
else
  echo 'debootstrap not found; aborting' >&2
  exit 1
fi

echo "==> prepare chroot environment"
$SUDO mount -t proc /proc "$OUTDIR/proc" || true
$SUDO mount -t sysfs /sys "$OUTDIR/sys" || true
$SUDO mount --bind /dev "$OUTDIR/dev" || true
if [[ -f /etc/resolv.conf ]]; then $SUDO cp /etc/resolv.conf "$OUTDIR/etc/"; fi
$SUDO chroot "$OUTDIR" /bin/bash -c "apt-get update && apt-get install -y apt-transport-https ca-certificates curl gnupg"
$SUDO umount "$OUTDIR/proc" || true
$SUDO umount "$OUTDIR/sys" || true
$SUDO umount "$OUTDIR/dev" || true

echo "==> download and package Flutter into a .deb"
curl -L -o "$WORKDIR/flutter.tar.xz" "$FLUTTER_URL"
DEB_ROOT="$WORKDIR/debroot"
rm -rf "$DEB_ROOT"
mkdir -p "$DEB_ROOT/opt"
tar -xf "$WORKDIR/flutter.tar.xz" -C "$DEB_ROOT/opt"
mkdir -p "$DEB_ROOT/usr/bin"
cat > "$DEB_ROOT/usr/bin/flutter" <<'EOF'
#!/usr/bin/env bash
exec /opt/flutter/bin/flutter "$@"
EOF
chmod +x "$DEB_ROOT/usr/bin/flutter"
mkdir -p "$DEB_ROOT/DEBIAN"
cat > "$DEB_ROOT/DEBIAN/control" <<EOF
Package: flutter-sdk
Version: 1.0.0
Section: utils
Priority: optional
Architecture: amd64
Maintainer: hypercore <dev@local>
Description: Flutter SDK packaged for local apt install
EOF
fakeroot dpkg-deb --build "$DEB_ROOT" "$WORKDIR/flutter-sdk_1.0.0_amd64.deb"

echo "==> install .deb into chroot"
$SUDO mkdir -p "$OUTDIR/tmp"
$SUDO cp "$WORKDIR/flutter-sdk_1.0.0_amd64.deb" "$OUTDIR/tmp/"
$SUDO mount -t proc /proc "$OUTDIR/proc" || true
$SUDO mount -t sysfs /sys "$OUTDIR/sys" || true
$SUDO mount --bind /dev "$OUTDIR/dev" || true
if [[ -f /etc/resolv.conf ]]; then $SUDO cp /etc/resolv.conf "$OUTDIR/etc/"; fi
$SUDO chroot "$OUTDIR" /bin/bash -c "set -e; dpkg -i /tmp/flutter-sdk_1.0.0_amd64.deb || (apt-get update && apt-get -f install -y); rm -f /tmp/flutter-sdk_1.0.0_amd64.deb"
$SUDO umount "$OUTDIR/proc" || true
$SUDO umount "$OUTDIR/sys" || true
$SUDO umount "$OUTDIR/dev" || true

echo "==> create ext4 disk image from rootfs"
IMG_OUT="$WORKDIR/rootfs.img"
dd if=/dev/zero of="$IMG_OUT" bs=1M count=4096
mkfs.ext4 -F "$IMG_OUT"
LOOP=$($SUDO losetup --find --show "$IMG_OUT")
MNT=$(mktemp -d)
$SUDO mount "$LOOP" "$MNT"
$SUDO rsync -aAX --numeric-ids --delete "$OUTDIR/" "$MNT/"
$SUDO umount "$MNT"
$SUDO losetup -d "$LOOP"
rm -rf "$MNT"

if [ "${SKIP_BOOT:-0}" = "1" ] || [ "$NO_BOOT_FLAG" = "--no-boot" ] ; then
  echo "SKIP_BOOT set; created disk image at: $IMG_OUT"
  exit 0
fi

echo "==> booting QEMU (this will attach to console)"
qemu-system-x86_64 -m 4096 -kernel "$KERNEL_IMAGE" -append "root=/dev/vda rw console=tty0 rootwait" -drive file="$IMG_OUT",if=virtio,format=raw -display sdl -vga virtio
