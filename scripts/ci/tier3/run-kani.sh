#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/common.sh"

ROOT="$(hc_repo_root)"
cd "${ROOT}/formal/kani"

cargo kani
