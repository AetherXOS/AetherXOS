<#
.SYNOPSIS
Install and validate Rust toolchain for HyperCore on Windows.
#>

param(
    [string]$Toolchain = "nightly",
    [string]$Target = "x86_64-unknown-none",
    [switch]$Offline
)

$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host "[setup_rust] $Message"
}

function Invoke-Checked {
    param(
        [string]$FilePath,
        [string[]]$Arguments
    )
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "command failed ($LASTEXITCODE): $FilePath $($Arguments -join ' ')"
    }
}

function Ensure-CargoPath {
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (-not (Test-Path $cargoBin)) { return }
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @()
    if ($userPath) { $parts = $userPath.Split(";") | Where-Object { $_ -ne "" } }
    if (-not ($parts -contains $cargoBin)) {
        $parts += $cargoBin
        [Environment]::SetEnvironmentVariable("Path", ($parts -join ";"), "User")
        $env:Path = "$cargoBin;$env:Path"
        Write-Step "added $cargoBin to user PATH"
    }
}

function Install-Rustup {
    if (Get-Command rustup -ErrorAction SilentlyContinue) { return }
    if ($Offline) {
        throw "rustup missing and offline mode enabled"
    }
    if (Get-Command winget -ErrorAction SilentlyContinue) {
        Write-Step "installing rustup via winget"
        & winget install --id Rustlang.Rustup --exact --accept-package-agreements --accept-source-agreements
        if ($LASTEXITCODE -eq 0) { Ensure-CargoPath; return }
    }
    if (Get-Command choco -ErrorAction SilentlyContinue) {
        Write-Step "installing rustup via choco"
        & choco install rustup.install -y
        if ($LASTEXITCODE -eq 0) { Ensure-CargoPath; return }
    }
    throw "rustup not found and installation failed (winget/choco)."
}

Write-Step "starting"
Install-Rustup
Ensure-CargoPath

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    throw "rustup still unavailable in current shell"
}

Write-Step "configuring toolchain=$Toolchain target=$Target"
if ($Offline) {
    Invoke-Checked -FilePath "rustup" -Arguments @("default", $Toolchain)
} else {
    Invoke-Checked -FilePath "rustup" -Arguments @("toolchain", "install", $Toolchain)
    Invoke-Checked -FilePath "rustup" -Arguments @("default", $Toolchain)
    Invoke-Checked -FilePath "rustup" -Arguments @("component", "add", "rust-src", "llvm-tools-preview", "--toolchain", $Toolchain)
    Invoke-Checked -FilePath "rustup" -Arguments @("target", "add", $Target, "--toolchain", $Toolchain)
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo not found after rustup setup"
}

Write-Step "READY"
& rustc -V
& cargo -V
