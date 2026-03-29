#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/common.sh"

hc_require_command isabelle

ROOT="$(hc_repo_root)"
cd "${ROOT}"

isabelle build -D formal/isabelle
