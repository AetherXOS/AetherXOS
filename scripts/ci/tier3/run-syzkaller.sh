#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/common.sh"

hc_require_command python3

ROOT="$(hc_repo_root)"
SYZ_DIR="${SYZKALLER_DIR:-}"

if [[ -z "${SYZ_DIR}" ]]; then
  printf 'set SYZKALLER_DIR before running the syzkaller lane\n' >&2
  exit 1
fi

cd "${ROOT}"
python3 "${SYZ_DIR}/tools/syz-manager" -config "formal/syzkaller/hypercore.cfg"
