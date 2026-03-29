#!/usr/bin/env bash
set -euo pipefail

log() { printf '\n[install_powershell] %s\n' "$*"; }
err() { printf '\n[install_powershell][error] %s\n' "$*" >&2; }

run_root() {
  local cmd="$1"
  if [[ "$(id -u)" -eq 0 ]]; then
    bash -lc "$cmd"
  elif command -v sudo >/dev/null 2>&1; then
    sudo bash -lc "$cmd"
  else
    err "sudo missing; run as root"
    exit 1
  fi
}

if command -v pwsh >/dev/null 2>&1; then
  log "PowerShell already installed: $(pwsh -NoLogo -NoProfile -Command '$PSVersionTable.PSVersion.ToString()')"
  exit 0
fi

OS="$(uname -s)"
case "$OS" in
  Linux*)
    if command -v apt-get >/dev/null 2>&1; then
      log "Installing PowerShell on Debian/Ubuntu via Microsoft package repo"
      run_root "apt-get update -y && apt-get install -y wget apt-transport-https software-properties-common gpg"
      run_root "wget -q https://packages.microsoft.com/config/ubuntu/24.04/packages-microsoft-prod.deb -O /tmp/packages-microsoft-prod.deb || wget -q https://packages.microsoft.com/config/ubuntu/22.04/packages-microsoft-prod.deb -O /tmp/packages-microsoft-prod.deb"
      run_root "dpkg -i /tmp/packages-microsoft-prod.deb"
      run_root "apt-get update -y && apt-get install -y powershell"
    elif command -v dnf >/dev/null 2>&1; then
      log "Installing PowerShell via dnf"
      run_root "dnf install -y powershell"
    elif command -v pacman >/dev/null 2>&1; then
      log "Installing PowerShell via pacman"
      run_root "pacman -Sy --noconfirm powershell"
    else
      err "Unsupported Linux package manager"
      exit 1
    fi
    ;;
  Darwin*)
    if ! command -v brew >/dev/null 2>&1; then
      err "Homebrew is required on macOS. Install from https://brew.sh"
      exit 1
    fi
    log "Installing PowerShell via brew"
    brew install --cask powershell
    ;;
  *)
    err "Unsupported OS: $OS"
    exit 1
    ;;
esac

if ! command -v pwsh >/dev/null 2>&1; then
  err "PowerShell installation failed"
  exit 1
fi

log "READY: $(pwsh -NoLogo -NoProfile -Command '$PSVersionTable.PSVersion.ToString()')"
