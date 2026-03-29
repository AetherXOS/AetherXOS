#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/common.sh"

SUITE="${1:-all}"
ROOT="$(hc_repo_root)"

cd "${ROOT}"

case "${SUITE}" in
  sanitizers)
    ./scripts/ci/tier2/run-sanitizers.sh
    ;;
  virtme-ng)
    ./scripts/ci/tier2/run-virtme-ng.sh
    ;;
  fuzz)
    ./scripts/ci/tier2/run-fuzz.sh
    ;;
  all)
    ./scripts/ci/tier2/run-sanitizers.sh
    ./scripts/ci/tier2/run-virtme-ng.sh
    ./scripts/ci/tier2/run-fuzz.sh
    ;;
  *)
    printf 'unknown tier2 suite: %s\n' "${SUITE}" >&2
    exit 1
    ;;
esac
