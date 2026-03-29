#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/common.sh"

hc_require_command cargo
hc_require_command rustc

ROOT="$(hc_repo_root)"
HOST_TARGET="${HOST_TARGET:-$(hc_host_target)}"

cd "${ROOT}"
cargo flamegraph --target "${HOST_TARGET}" --example kernel_config_probe
