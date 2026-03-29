#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/common.sh"

hc_require_command cargo

ROOT="$(hc_repo_root)"
cd "${ROOT}/fuzz"

cargo fuzz run kernel_config_bytes -- -max_total_time="${FUZZ_SECONDS:-120}"
