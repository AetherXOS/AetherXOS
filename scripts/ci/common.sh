#!/usr/bin/env bash
set -euo pipefail

hc_repo_root() {
  local script_dir
  script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
  cd -- "${script_dir}/../.." >/dev/null 2>&1
  pwd
}

hc_host_target() {
  rustc -vV | awk '/^host: / { print $2 }'
}

hc_reports_dir() {
  local root
  root="$(hc_repo_root)"
  mkdir -p "${root}/reports/ci"
  printf '%s\n' "${root}/reports/ci"
}

hc_require_command() {
  local command_name="$1"
  command -v "${command_name}" >/dev/null 2>&1 || {
    printf 'missing required command: %s\n' "${command_name}" >&2
    exit 1
  }
}
