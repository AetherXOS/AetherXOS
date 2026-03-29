<#
.SYNOPSIS
Validates local Shim EFI binaries for experimental shim+limine boot flow.

.DESCRIPTION
Checks that required files exist under -ShimDir:
  - shimx64.efi
  - mmx64.efi
Optional:
  - fbx64.efi

Note: This script does not sign binaries or enroll keys.
#>

param(
    [string]$ShimDir = "artifacts/shim/bin"
)

$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host "[setup_shim] $Message"
}

$resolved = Resolve-Path $ShimDir -ErrorAction SilentlyContinue
if (-not $resolved) {
    throw "Shim directory not found: $ShimDir"
}

$required = @("shimx64.efi", "mmx64.efi")
foreach ($name in $required) {
    $p = Join-Path $resolved $name
    if (-not (Test-Path $p)) {
        throw "Missing required shim file: $p"
    }
}

$fallback = Join-Path $resolved "fbx64.efi"
if (Test-Path $fallback) {
    Write-Step "optional fbx64.efi found"
} else {
    Write-Step "optional fbx64.efi not found (ok)"
}

Write-Step "READY -> $resolved"
Write-Step "files: shimx64.efi, mmx64.efi"
