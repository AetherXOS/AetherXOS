#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 OUTDIR FLUTTER_TAR_XZ_URL [KERNEL_IMAGE=kernel.x]"
  echo "Example: $0 /tmp/debian-rootfs 'https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_<ver>-stable.tar.xz' kernel.x"
  exit 1
fi

OUTDIR=$1
FLUTTER_URL=$2
KERNEL_IMAGE=${3:-kernel.x}

WORKDIR="${OUTDIR%/}/work"
mkdir -p "$WORKDIR"

echo "1) Create Debian rootfs in: $OUTDIR"
bash scripts/prepare_debian_rootfs.sh "$OUTDIR"

echo "2) Package Flutter into .deb"
bash scripts/package_flutter_deb.sh "$WORKDIR" "$FLUTTER_URL" flutter-sdk 1.0.0
DEB_PATH="$WORKDIR/flutter-sdk_1.0.0_amd64.deb"

echo "3) Install .deb into rootfs (chroot)"
bash scripts/install_deb_into_rootfs.sh "$OUTDIR" "$DEB_PATH"

echo "4) Produce ext4 disk image"
IMG_OUT="$WORKDIR/rootfs.img"
bash scripts/mk_rootfs_image.sh "$OUTDIR" "$IMG_OUT" 4096

echo "5) Boot kernel with QEMU"
bash scripts/run_qemu_rootfs.sh "$KERNEL_IMAGE" "$IMG_OUT" 4096
