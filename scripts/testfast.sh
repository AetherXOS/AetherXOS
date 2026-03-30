#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
host="$(rustc -vV | sed -n 's/^host: //p')"

run() {
    printf '==> %s\n' "$1"
    shift
    "$@"
}

optional() {
    local gate="$1"
    shift
    if [[ "${!gate:-0}" == "1" ]]; then
        run "$@"
    else
        printf '==> skip %s\n' "$1"
    fi
}

run nextest cargo nextest run --config-file "$root/.config/nextest.toml" --target "$host" --features kernel_test_mode --test fast
run clippy cargo clippy --manifest-path "$root/Cargo.toml" --lib --target "$host" --features kernel_test_mode -- \
    -A unused \
    -A dead_code \
    -A unused_imports \
    -A unused_variables \
    -A unused_mut \
    -A unsafe_op_in_unsafe_fn \
    -A clippy::all
run rustfmt cargo fmt --manifest-path "$root/Cargo.toml" --all --check

if cargo geiger --help >/dev/null 2>&1; then
    optional HYPERCORE_ENABLE_GEIGER geiger cargo geiger --manifest-path "$root/Cargo.toml" --all-targets --target "$host" --features kernel_test_mode
else
    printf '==> skip geiger\n'
fi

if cargo rudra --help >/dev/null 2>&1; then
    optional HYPERCORE_ENABLE_RUDRA rudra cargo rudra --manifest-path "$root/Cargo.toml" --all-targets --target "$host" --features kernel_test_mode
else
    printf '==> skip rudra\n'
fi

if cargo audit --help >/dev/null 2>&1; then
    optional HYPERCORE_ENABLE_AUDIT audit cargo audit
else
    printf '==> skip audit\n'
fi
