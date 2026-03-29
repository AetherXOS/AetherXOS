#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/common.sh"

hc_require_command cargo
hc_require_command rustc

ROOT="$(hc_repo_root)"
REPORTS="$(hc_reports_dir)/tier2"
HOST_TARGET="${HOST_TARGET:-$(hc_host_target)}"

mkdir -p "${REPORTS}"
cd "${ROOT}"

RUSTFLAGS='-Zsanitizer=address' cargo test -Zbuild-std --target "${HOST_TARGET}" --test tier1 --no-run
RUSTFLAGS='-Zsanitizer=undefined' cargo test -Zbuild-std --target "${HOST_TARGET}" --test tier1 --no-run

if [[ -n "${KERNEL_SANITIZER_IMAGE:-}" ]]; then
  printf 'using kernel image %s for manual kasan/kmsan hooks\n' "${KERNEL_SANITIZER_IMAGE}" | tee "${REPORTS}/kernel-sanitizers.txt"
else
  printf 'set KERNEL_SANITIZER_IMAGE to execute kasan/kmsan guest runs\n' | tee "${REPORTS}/kernel-sanitizers.txt"
fi
