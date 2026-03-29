#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/common.sh"

hc_require_command cargo
hc_require_command rustc

ROOT="$(hc_repo_root)"
REPORTS="$(hc_reports_dir)/tier1"
HOST_TARGET="${HOST_TARGET:-$(hc_host_target)}"

mkdir -p "${REPORTS}"

cd "${ROOT}"

cargo nextest run --config-file .config/nextest.toml --target "${HOST_TARGET}" --test tier1
cargo clippy --all-targets --target "${HOST_TARGET}" -- -D warnings
cargo geiger --all-targets --target "${HOST_TARGET}" | tee "${REPORTS}/geiger.txt"
cargo rudra --all-targets --target "${HOST_TARGET}" | tee "${REPORTS}/rudra.txt"
