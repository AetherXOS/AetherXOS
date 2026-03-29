#!/usr/bin/env bash
set -euo pipefail

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/common.sh"

hc_require_command java

ROOT="$(hc_repo_root)"
TLA_JAR="${TLA_JAR:-${ROOT}/tools/tla2tools.jar}"

if [[ ! -f "${TLA_JAR}" ]]; then
  printf 'set TLA_JAR to a tla2tools.jar path\n' >&2
  exit 1
fi

cd "${ROOT}/formal/tla"
java -cp "${TLA_JAR}" tlc2.TLC -config KernelConfigOverrides.cfg KernelConfigOverrides.tla
