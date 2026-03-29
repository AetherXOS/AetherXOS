#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TARGET="${TARGET:-x86_64-unknown-none}"
TOOLCHAIN="${TOOLCHAIN:-nightly}"
RUN_BUILD=1
RUN_TESTS=0
RUN_HOST_CHECK=0
STRICT_OPTIONAL_CHECKS=0
INSTALL_PACKAGES=1

usage() {
  cat <<'USAGE'
HyperCore agent/bootstrap setup script.

This script prepares a reproducible Rust + target environment and validates
that HyperCore can be built in one command.

Usage:
  scripts/setup/setup_agent_env.sh [options]

Options:
  --target <triple>        Rust target triple (default: x86_64-unknown-none)
  --toolchain <name>       Rust toolchain channel (default: nightly)
  --with-host-check        Also run host `cargo check` (best-effort)
  --with-tests             Also run host `cargo test` (best-effort)
  --strict-optional-checks Fail if host check/tests fail
  --skip-build             Skip cargo build steps
  --no-install             Do not attempt system package installation
  -h, --help               Show this help

Environment variables:
  TARGET, TOOLCHAIN        Alternative way to set defaults
USAGE
}

log() { printf '\n[setup] %s\n' "$*"; }
warn() { printf '\n[setup][warn] %s\n' "$*" >&2; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      TARGET="$2"; shift 2 ;;
    --toolchain)
      TOOLCHAIN="$2"; shift 2 ;;
    --with-host-check)
      RUN_HOST_CHECK=1; shift ;;
    --with-tests)
      RUN_TESTS=1; shift ;;
    --strict-optional-checks)
      STRICT_OPTIONAL_CHECKS=1; shift ;;
    --skip-build)
      RUN_BUILD=0; shift ;;
    --no-install)
      INSTALL_PACKAGES=0; shift ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1 ;;
  esac
done

install_qemu_if_possible() {
  if command -v qemu-system-x86_64 >/dev/null 2>&1; then
    log "qemu-system-x86_64 already installed"
    return
  fi

  if [[ "$INSTALL_PACKAGES" -eq 0 ]]; then
    warn "QEMU not installed and --no-install was used. Emulator validation will be unavailable."
    return
  fi

  if command -v apt-get >/dev/null 2>&1; then
    log "Installing qemu-system-x86 via apt-get"
    sudo apt-get update -y && sudo apt-get install -y qemu-system-x86 || warn "Unable to install qemu via apt-get"
  elif command -v dnf >/dev/null 2>&1; then
    log "Installing qemu-system-x86 via dnf"
    sudo dnf install -y qemu-system-x86 || warn "Unable to install qemu via dnf"
  elif command -v pacman >/dev/null 2>&1; then
    log "Installing qemu-system-x86 via pacman"
    sudo pacman -Sy --noconfirm qemu-system-x86 || warn "Unable to install qemu via pacman"
  else
    warn "Unsupported package manager. Install qemu-system-x86_64 manually if needed."
  fi
}

ensure_rustup() {
  if command -v rustup >/dev/null 2>&1; then
    return
  fi
  echo "rustup is required but was not found in PATH." >&2
  echo "Install rustup from https://rustup.rs and run this script again." >&2
  exit 1
}


ensure_hyper_config() {
  if [[ -f "$ROOT_DIR/hyper_config.toml" ]]; then
    return
  fi

  if [[ -f "$ROOT_DIR/hyper_config.toml.example" ]]; then
    log "hyper_config.toml missing; creating from hyper_config.toml.example"
    cp "$ROOT_DIR/hyper_config.toml.example" "$ROOT_DIR/hyper_config.toml"
    return
  fi

  echo "hyper_config.toml is missing and no hyper_config.toml.example is available." >&2
  echo "Create hyper_config.toml before running build validation." >&2
  exit 1
}

ensure_toolchain_and_components() {
  log "Ensuring Rust toolchain: $TOOLCHAIN"
  rustup toolchain install "$TOOLCHAIN"
  rustup override set "$TOOLCHAIN"

  log "Adding required components for $TOOLCHAIN"
  rustup component add rust-src llvm-tools-preview --toolchain "$TOOLCHAIN"

  log "Adding target: $TARGET"
  rustup target add "$TARGET" --toolchain "$TOOLCHAIN"
}

run_optional_or_warn() {
  local description="$1"
  shift

  if "$@"; then
    return 0
  fi

  if [[ "$STRICT_OPTIONAL_CHECKS" -eq 1 ]]; then
    echo "[setup][error] Optional validation failed in strict mode: $description" >&2
    return 1
  fi

  warn "$description failed (non-blocking). Re-run with --strict-optional-checks to fail hard."
  return 0
}

run_validation() {
  export CARGO_INCREMENTAL=0

  if [[ "$RUN_BUILD" -eq 1 ]]; then
    log "Running cross build: cargo +$TOOLCHAIN build --target $TARGET"
    cargo +"$TOOLCHAIN" build --target "$TARGET"

    log "Running cross release build: cargo +$TOOLCHAIN build --release --target $TARGET"
    cargo +"$TOOLCHAIN" build --release --target "$TARGET"
  fi

  local host_triple
  host_triple="$(rustc -vV | sed -n 's/^host: //p')"

  if [[ "$RUN_HOST_CHECK" -eq 1 ]]; then
    if [[ -n "$host_triple" ]]; then
      log "Running host check (optional): cargo +$TOOLCHAIN check --target $host_triple"
      run_optional_or_warn "Host cargo check" cargo +"$TOOLCHAIN" check --target "$host_triple"
    else
      warn "Unable to detect host triple; skipping host check"
    fi
  fi

  if [[ "$RUN_TESTS" -eq 1 ]]; then
    if [[ -n "$host_triple" ]]; then
      log "Running host tests (optional): cargo +$TOOLCHAIN test --target $host_triple"
      run_optional_or_warn "Host cargo test" cargo +"$TOOLCHAIN" test --target "$host_triple"
    else
      warn "Unable to detect host triple; skipping host tests"
    fi
  fi
}

main() {
  log "Preparing HyperCore environment in $ROOT_DIR"
  ensure_rustup
  ensure_toolchain_and_components
  ensure_hyper_config
  install_qemu_if_possible
  run_validation
  log "Environment setup and validation completed successfully"
}

main
