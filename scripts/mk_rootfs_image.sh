#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 ROOTFS_DIR OUT_IMG [SIZE_MB=2048]"
  exit 1
fi

ROOTFS_DIR=$1
OUT_IMG=$2
SIZE_MB=${3:-2048}

if [[ ! -d "$ROOTFS_DIR" ]]; then
  echo "error: ROOTFS_DIR '$ROOTFS_DIR' not found or not a directory"
  exit 1
fi

command -v dd >/dev/null 2>&1 || { echo "dd required"; exit 1; }
command -v mkfs.ext4 >/dev/null 2>&1 || { echo "mkfs.ext4 required (e2fsprogs)"; exit 1; }
command -v losetup >/dev/null 2>&1 || { echo "losetup required"; exit 1; }
command -v rsync >/dev/null 2>&1 || { echo "rsync required"; exit 1; }

echo "Creating ext4 image '$OUT_IMG' ($SIZE_MB MB) from '$ROOTFS_DIR'"

dd if=/dev/zero of="$OUT_IMG" bs=1M count="$SIZE_MB"
mkfs.ext4 -F "$OUT_IMG"

LOOP=$(sudo losetup --find --show "$OUT_IMG")
MNT=$(mktemp -d)
cleanup() {
  set +e
  if mountpoint -q "$MNT"; then sudo umount "$MNT"; fi
  if [[ -n "${LOOP:-}" ]]; then sudo losetup -d "$LOOP"; fi
  rm -rf "$MNT"
}
trap cleanup EXIT

sudo mount "$LOOP" "$MNT"
sudo rsync -aAX --numeric-ids --delete "$ROOTFS_DIR"/ "$MNT"/
sudo umount "$MNT"
sudo losetup -d "$LOOP"
trap - EXIT
rm -rf "$MNT"

echo "Image created: $OUT_IMG"
