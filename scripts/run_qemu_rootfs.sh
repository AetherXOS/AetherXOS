#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 KERNEL_IMAGE ROOTFS_IMG [RAM_MB=2048] [QEMU_EXTRA_ARGS...]"
  exit 1
fi

KERNEL_IMAGE=$1
ROOTFS_IMG=$2
RAM_MB=${3:-2048}
shift 3 || true

if [[ ! -f "$KERNEL_IMAGE" ]]; then
  echo "error: kernel image '$KERNEL_IMAGE' not found"
  exit 1
fi
if [[ ! -f "$ROOTFS_IMG" ]]; then
  echo "error: rootfs image '$ROOTFS_IMG' not found"
  exit 1
fi

command -v qemu-system-x86_64 >/dev/null 2>&1 || { echo "qemu-system-x86_64 required"; exit 1; }

echo "Booting QEMU: kernel=$KERNEL_IMAGE rootfs=$ROOTFS_IMG ram=${RAM_MB}MB"

QEMU_CMD=(
  qemu-system-x86_64
  -m "$RAM_MB"
  -kernel "$KERNEL_IMAGE"
  -append "root=/dev/vda rw console=ttyS0 rootwait"
  -drive file="$ROOTFS_IMG",if=virtio,format=raw
  -nographic
  -serial mon:stdio
)

if [[ $# -ne 0 ]]; then
  QEMU_CMD+=("$@")
fi

echo "Running: ${QEMU_CMD[*]}"
"${QEMU_CMD[@]}"
