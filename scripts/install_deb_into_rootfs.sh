#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 ROOTFS_DIR FLUTTER_DEB"
  exit 1
fi

ROOTFS_DIR=$1
DEB_FILE=$2

if [[ ! -d "$ROOTFS_DIR" ]]; then
  echo "error: ROOTFS_DIR not found: $ROOTFS_DIR"
  exit 1
fi
if [[ ! -f "$DEB_FILE" ]]; then
  echo "error: DEB file not found: $DEB_FILE"
  exit 1
fi

echo "Copying $DEB_FILE to $ROOTFS_DIR/tmp/"
sudo mkdir -p "$ROOTFS_DIR/tmp"
sudo cp "$DEB_FILE" "$ROOTFS_DIR/tmp/"

echo "Mounting pseudo-filesystems into chroot"
sudo mount -t proc /proc "$ROOTFS_DIR/proc" || true
sudo mount -t sysfs /sys "$ROOTFS_DIR/sys" || true
sudo mount --bind /dev "$ROOTFS_DIR/dev" || true
if [[ -f /etc/resolv.conf ]]; then sudo cp /etc/resolv.conf "$ROOTFS_DIR/etc/"; fi

echo "Installing .deb inside chroot"
sudo chroot "$ROOTFS_DIR" /bin/bash -c "set -e; dpkg -i /tmp/$(basename "$DEB_FILE") || (apt-get update && apt-get -f install -y); rm -f /tmp/$(basename "$DEB_FILE")"

echo "Cleaning up mounts"
sudo umount "$ROOTFS_DIR/proc" || true
sudo umount "$ROOTFS_DIR/sys" || true
sudo umount "$ROOTFS_DIR/dev" || true

echo "Installation complete: $DEB_FILE -> $ROOTFS_DIR"
