#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/common.sh"

SUITE="${1:-all}"
ROOT="$(hc_repo_root)"

cd "${ROOT}"

case "${SUITE}" in
  syzkaller)
    ./scripts/ci/tier3/run-syzkaller.sh
    ;;
  tla)
    ./scripts/ci/tier3/run-tla.sh
    ;;
  kani)
    ./scripts/ci/tier3/run-kani.sh
    ;;
  isabelle)
    ./scripts/ci/tier3/run-isabelle.sh
    ;;
  flamegraph)
    ./scripts/ci/tier3/run-flamegraph.sh
    ;;
  all)
    ./scripts/ci/tier3/run-tla.sh
    ./scripts/ci/tier3/run-kani.sh
    ./scripts/ci/tier3/run-isabelle.sh
    ./scripts/ci/tier3/run-syzkaller.sh
    ./scripts/ci/tier3/run-flamegraph.sh
    ;;
  *)
    printf 'unknown tier3 suite: %s\n' "${SUITE}" >&2
    exit 1
    ;;
esac
