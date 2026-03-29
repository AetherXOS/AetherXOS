#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/common.sh"

hc_require_command python3

ROOT="$(hc_repo_root)"
REPORTS="$(hc_reports_dir)/tier2"

mkdir -p "${REPORTS}"
cd "${ROOT}"

python3 -m pip install --user virtme-ng >/dev/null

if [[ -z "${VIRTME_KERNEL_IMAGE:-}" ]]; then
  printf 'set VIRTME_KERNEL_IMAGE to boot the integration lane\n' | tee "${REPORTS}/virtme-ng.txt"
  exit 0
fi

virtme-run --kimg "${VIRTME_KERNEL_IMAGE}" --script-sh 'uname -a'
