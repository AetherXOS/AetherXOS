<#
.SYNOPSIS
Downloads/builds Limine binaries required by ISO boot pipeline.

.DESCRIPTION
Runs scripts/tools/ensure_limine_binaries.py and verifies that:
  - limine-bios-cd.bin
  - limine-uefi-cd.bin
  - BOOTX64.EFI
exist in the output directory.

On Windows, optional -InstallWslDeps installs required Ubuntu packages in WSL.
#>

param(
    [string]$Version = "latest",
    [string]$OutDir = "artifacts/limine/bin",
    [string]$CacheDir = "artifacts/limine/cache",
    [switch]$InstallWslDeps,
    [string]$WslDistro = "Ubuntu-24.04",
    [switch]$SkipNativeBuild,
    [switch]$NoWslFallback,
    [switch]$Offline
)

$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host "[setup_limine] $Message"
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

function Resolve-WslDistro {
    param([string]$Preferred)
    if (-not (Get-Command wsl.exe -ErrorAction SilentlyContinue)) {
        return $null
    }
    $listRaw = & wsl.exe -l -q
    $distros = @($listRaw | ForEach-Object { ($_ -replace "`0","").Trim() } | Where-Object { $_ })
    if ($Preferred -and ($distros -contains $Preferred)) { return $Preferred }
    foreach ($name in @("Ubuntu-24.04", "Ubuntu")) {
        if ($distros -contains $name) { return $name }
    }
    if ($distros.Count -gt 0) { return $distros[0] }
    return $null
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

Write-Step "repo_root=$repoRoot"

if (-not (Get-Command python -ErrorAction SilentlyContinue)) {
    throw "python not found in PATH"
}

if ($InstallWslDeps) {
    if (-not (Get-Command wsl.exe -ErrorAction SilentlyContinue)) {
        throw "wsl.exe not found; disable -InstallWslDeps or install WSL"
    }

    $resolvedDistro = Resolve-WslDistro -Preferred $WslDistro
    if (-not $resolvedDistro) {
        throw "No WSL distro found. Install Ubuntu first, then rerun."
    }

    Write-Step "installing WSL dependencies on distro=$resolvedDistro"
    $wslInstallLine = 'set -euo pipefail; export DEBIAN_FRONTEND=noninteractive; if [ $(id -u) -eq 0 ]; then apt-get update; apt-get install -y build-essential autoconf automake libtool clang lld llvm nasm mtools xorriso; else sudo apt-get update; sudo apt-get install -y build-essential autoconf automake libtool clang lld llvm nasm mtools xorriso; fi'
    $installCmd = @(
        "-d", $resolvedDistro, "--",
        "bash", "-lc",
        $wslInstallLine
    )
    Invoke-Checked -FilePath "wsl.exe" -Arguments $installCmd
}

$args = @(
    "scripts/tools/ensure_limine_binaries.py",
    "--version", $Version,
    "--out-dir", $OutDir,
    "--cache-dir", $CacheDir
)

if (-not $SkipNativeBuild) {
    $args += "--allow-build"
}
if (-not $NoWslFallback) {
    $args += "--allow-wsl-build"
}
if ($Offline) {
    $args += "--offline"
}

Write-Step "running ensure_limine_binaries.py"
Invoke-Checked -FilePath "python" -Arguments $args

$resolvedOut = Resolve-Path $OutDir
$required = @("limine-bios-cd.bin", "limine-uefi-cd.bin", "BOOTX64.EFI")
foreach ($name in $required) {
    $p = Join-Path $resolvedOut $name
    if (-not (Test-Path $p)) {
        throw "missing required artifact: $p"
    }
}

Write-Step "READY -> $resolvedOut"
Write-Step "files: $($required -join ', ')"
