# PowerShell script to ensure ISO creation tools are available
param([switch]$Force = $false)

$ErrorActionPreference = "Continue"

function Test-Command {
    param($Cmd)
    try {
        $null = & $Cmd --version 2>$null
        return $true
    } catch {
        return $false
    }
}

# Check if xorriso exists
if (Test-Command xorriso) {
    Write-Host "[ISO] xorriso found" -ForegroundColor Green
    exit 0
}

# Try common paths
$paths = @(
    "C:\msys64\usr\bin\xorriso.exe",
    "C:\Program Files\Git\usr\bin\xorriso.exe"
)

foreach ($path in $paths) {
    if (Test-Path $path) {
        Write-Host "[ISO] xorriso found at $path" -ForegroundColor Green
        exit 0
    }
}

# Check if mkisofs exists
if (Test-Command mkisofs) {
    Write-Host "[ISO] mkisofs found" -ForegroundColor Green
    exit 0
}

# Try to install via scoop
Write-Host "[ISO] No ISO tools found. Attempting scoop install..." -ForegroundColor Yellow

if (Test-Command scoop) {
    Write-Host "[ISO] Installing xorriso via scoop..." -ForegroundColor Cyan
    & scoop install xorriso --global 2>&1
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "[ISO] xorriso installed successfully" -ForegroundColor Green
        exit 0
    }
}

Write-Host "[ISO] Scoop not available or install failed" -ForegroundColor Yellow
Write-Host "[ISO] Please install xorriso manually:" -ForegroundColor Cyan
Write-Host "    scoop install xorriso" -ForegroundColor White
Write-Host "    OR download from: https://www.gnu.org/software/xorriso/" -ForegroundColor White
exit 1
