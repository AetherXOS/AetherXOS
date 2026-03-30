#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 OUTDIR [SUITE=stable] [ARCH=amd64]"
  exit 1
fi

OUTDIR=$1
SUITE=${2:-stable}
ARCH=${3:-amd64}
MIRROR=${4:-http://deb.debian.org/debian}

echo "Preparing Debian rootfs: out=$OUTDIR suite=$SUITE arch=$ARCH"

if command -v debootstrap >/dev/null 2>&1; then
  sudo debootstrap --variant=minbase --arch="$ARCH" "$SUITE" "$OUTDIR" "$MIRROR"
else
  echo "debootstrap not found. Install on the host: sudo apt install debootstrap"
  exit 1
fi

echo "Binding pseudo-filesystems and copying resolv.conf"
sudo mount -t proc /proc "$OUTDIR/proc"
sudo mount -t sysfs /sys "$OUTDIR/sys"
sudo mount --bind /dev "$OUTDIR/dev"
if [[ -f /etc/resolv.conf ]]; then
  sudo cp /etc/resolv.conf "$OUTDIR/etc/"
fi

echo "Chroot: updating apt and installing basic helpers"
sudo chroot "$OUTDIR" /bin/bash -c "apt-get update && apt-get install -y apt-transport-https ca-certificates curl gnupg"

echo "Cleaning up mounts"
sudo umount "$OUTDIR/proc" || true
sudo umount "$OUTDIR/sys" || true
sudo umount "$OUTDIR/dev" || true

echo "Debian rootfs prepared at: $OUTDIR"

echo "Next: enter chroot (sudo chroot $OUTDIR /bin/bash) and install packages or create users as needed."
