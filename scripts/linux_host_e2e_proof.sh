#!/usr/bin/env bash
set -euo pipefail

# Linux host only: produces a reproducible proof bundle for package-stack closure.
if [[ "$(uname -s)" != "Linux" ]]; then
    echo "[linux-host-e2e] This pipeline is Linux host only."
    exit 2
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
reports_dir="$root/reports/linux_host_e2e_proof"
mkdir -p "$reports_dir"

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
log="$reports_dir/${stamp}.log"
json="$reports_dir/${stamp}.json"
latest="$reports_dir/latest.json"

run_step() {
    local name="$1"
    shift
    echo "==> $name" | tee -a "$log"
    "$@" 2>&1 | tee -a "$log"
}

run_step "build apt-iso" cargo run -p xtask -- build apt-iso
run_step "qemu smoke" cargo run -p xtask -- ops qemu smoke
run_step "strict linux-app-compat" cargo run -p xtask -- test linux-app-compat --strict --ci --require-package-stack --require-fs-stack --desktop-smoke

cat >"$json" <<JSON
{
  "generated_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "host": "linux",
  "commands": [
    "cargo run -p xtask -- build apt-iso",
    "cargo run -p xtask -- ops qemu smoke",
    "cargo run -p xtask -- test linux-app-compat --strict --ci --require-package-stack --require-fs-stack --desktop-smoke"
  ],
  "artifacts": {
    "log": "${log#$root/}",
    "linux_app_runtime_probe": "reports/linux_app_runtime_probe_report.json",
    "linux_app_scorecard": "reports/linux_app_compat_validation_scorecard.json"
  },
  "status": "pass"
}
JSON

cp "$json" "$latest"
echo "[linux-host-e2e] proof written: ${json#$root/}"