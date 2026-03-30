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

run nextest env RUSTFLAGS="-A warnings" cargo nextest run --config-file "$root/.config/nextest.toml" --target "$host" --features kernel_test_mode --test nightly
run clippy cargo clippy --manifest-path "$root/Cargo.toml" --lib --target "$host" --features kernel_test_mode -- \
    -A warnings \
    -A unused \
    -A dead_code \
    -A unused_imports \
    -A unused_variables \
    -A unused_mut \
    -A unsafe_op_in_unsafe_fn \
    -A clippy::all

if cargo kani --help >/dev/null 2>&1; then
    runif HYPERCORE_RUN_KANI kani cargo kani --manifest-path "$root/formal/kani/Cargo.toml"
else
    printf '==> skip kani\n'
fi

if command -v tlc >/dev/null 2>&1; then
    runif HYPERCORE_RUN_TLAPLUS tlaplus tlc "$root/formal/tla/KernelConfigOverrides.tla"
else
    printf '==> skip tlaplus\n'
fi

if command -v isabelle >/dev/null 2>&1; then
    runif HYPERCORE_RUN_ISABELLE isabelle isabelle build -D "$root/formal/isabelle"
else
    printf '==> skip isabelle\n'
fi

if command -v syz-manager >/dev/null 2>&1; then
    runif HYPERCORE_RUN_SYZKALLER syzkaller syz-manager -config "$root/formal/syzkaller/hypercore.cfg"
else
    printf '==> skip syzkaller\n'
fi

if cargo flamegraph --help >/dev/null 2>&1; then
    runif HYPERCORE_RUN_FLAMEGRAPH flamegraph cargo flamegraph --manifest-path "$root/host_tools/scheduler_host_tests/Cargo.toml" --test scheduler_runtime
else
    printf '==> skip flamegraph\n'
fi
