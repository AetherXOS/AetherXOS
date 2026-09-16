xtask: Using an external guest rootfs with your kernel

Overview

This document explains how to provide an external rootfs (Ubuntu or other) and run
Aether X OS kernel + that rootfs under QEMU using `xtask`.

What was added

- `cargo run -p xtask -- build full ... --rootfs <path>` accepts either a directory or
  a tarball (.tar / .tar.gz). The contents will be staged into the image stage
  at `artifacts/boot_image/stage/boot/var/lib/hypercore/rootfs`.
- For Unix hosts (best-effort), xtask will attempt to create a partitioned raw
  disk image `artifacts/aethercore-rootfs.img` containing a single ext4
  partition populated with the provided rootfs. This requires host tools:
  `qemu-img`, `parted`, `losetup`, `kpartx`, `mkfs.ext4`, `mount`, `umount`, `tar`.
- If `artifacts/aethercore-rootfs.img` exists, xtask will automatically attach it
  to QEMU as a virtio drive when using `xtask run` / `xtask test` / `xtask` flows
  that launch QEMU.

Usage examples

1) Using a directory rootfs (local path):

```bash
cargo run -p xtask -- build full --arch x86_64 --bootloader limine --format img --release --rootfs /path/to/ubuntu-rootfs
```

2) Using a tarball rootfs (.tar.gz):

```bash
cargo run -p xtask -- build full --arch x86_64 --bootloader limine --format img --release --rootfs /path/to/ubuntu-rootfs.tar.gz
```

3) Run QEMU (interactive) after building:

```bash
cargo run -p xtask -- run live
# or for smoke test
cargo run -p xtask -- run smoke
```

If `artifacts/aethercore-rootfs.img` was produced, QEMU is launched with an extra
`-drive file=...,format=raw,if=virtio` argument so the guest sees the disk.

Notes and limitations

- Partitioned image creation is best-effort and currently only supported on
  Unix-like hosts due to required host utilities. On Windows hosts xtask will
  skip image creation and only stage files into `artifacts/boot_image/stage/boot`.
- The initramfs scripts look for partitioned devices like `/dev/vda1`. The
  partitioned image creation above creates a disk image with an msdos label and
  a single primary partition. If your host lacks the required tools, you can
  also create and prepare a disk image manually and place it at
  `artifacts/aethercore-rootfs.img`.

Manual image creation (if needed)

On a Linux host you can manually create a partitioned image, format and populate it:

```bash
# create sparse raw file
qemu-img create -f raw artifacts/aethercore-rootfs.img 2048M
# partition with parted
parted -s artifacts/aethercore-rootfs.img mklabel msdos mkpart primary ext4 1MiB 100%
# map loop device and create filesystem (requires root)
LOOP=$(losetup --find --show artifacts/aethercore-rootfs.img)
kpartx -a $LOOP
PART=/dev/mapper/$(basename $LOOP)p1
mkfs.ext4 -F $PART
mkdir -p /mnt/tmproot
mount $PART /mnt/tmproot
# copy rootfs contents
tar -C /path/to/ubuntu-rootfs -cpf - . | tar -C /mnt/tmproot -xpf -
umount /mnt/tmproot
kpartx -d $LOOP
losetup -d $LOOP
```

Questions or next steps

If you want, I can:
- (A) Improve the partitioned image creation to be more robust and support fallback
  tools.
- (B) Add an `xtask run` flag to explicitly force adding/removing the rootfs drive.
- (C) Add CI tests or a sample Ubuntu tarball fixture for quick local smoke runs.

Which of these would you like next?
