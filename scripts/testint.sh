#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
host="$(rustc -vV | sed -n 's/^host: //p')"

run() {
    printf '==> %s\n' "$1"
    shift
    "$@"
}

runif() {
    local gate="$1"
    shift
    if [[ "${!gate:-0}" == "1" ]]; then
        run "$@"
    else
        printf '==> skip %s\n' "$1"
    fi
}

run nextest cargo nextest run --config-file "$root/.config/nextest.toml" --target "$host" --test integration_tests

runif HYPERCORE_RUN_KASAN kasan cargo test --manifest-path "$root/host_rust_tests/Cargo.toml" --tests
runif HYPERCORE_RUN_KMSAN kmsan cargo test --manifest-path "$root/host_tools/scheduler_host_tests/Cargo.toml" --tests
runif HYPERCORE_RUN_UBSAN ubsan cargo test --manifest-path "$root/agent/Cargo.toml" --tests

if command -v vng >/dev/null 2>&1; then
    runif HYPERCORE_RUN_VIRTME virtme vng --version
else
    printf '==> skip virtme\n'
fi

if cargo fuzz --help >/dev/null 2>&1; then
    runif HYPERCORE_RUN_FUZZ cargofuzz cargo fuzz build kernel_config_bytes --manifest-path "$root/fuzz/Cargo.toml"
else
    printf '==> skip cargofuzz\n'
fi
