# Run host-safe validation for the host toolchain.
# This repo's Rust tests primarily target bare metal, so host validation uses
# Rust semantic compilation plus PowerShell contract tests.
# Usage examples:
#   .\scripts\run_tests_host.ps1                    # run all tests on detected host
#   .\scripts\run_tests_host.ps1 -Filter Tooling     # run matching Pester test names
#   .\scripts\run_tests_host.ps1 -Release           # run release-mode tests

param(
    [string]$Filter = "",
    [switch]$Release
)

function Get-HostTriple {
    $rustInfo = & rustc -vV 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $rustInfo) {
        return $null
    }
    $m = $rustInfo | Select-String -Pattern '^host:\s*(.+)$' -Quiet:$false
    if ($m) {
        return ($m -replace '^host:\s*', '')
    }
    return $null
}

$hostTriple = Get-HostTriple
if (-not $hostTriple) {
    Write-Warning "Unable to determine host triple from rustc; falling back to building/running default host tests."
    $hostTriple = $null
}

Write-Host "Running tests on host" -ForegroundColor Cyan

$env:CARGO_INCREMENTAL = "0"

function Invoke-CargoCheckVariant {
    param(
        [string]$Label,
        [string]$Features = ""
    )

    $cargoArgs = @("check", "--lib")
    if ($Features -ne "") {
        $cargoArgs += "--features"
        $cargoArgs += $Features
    }
    if ($hostTriple) { $cargoArgs += "--target"; $cargoArgs += $hostTriple }
    if ($Release) { $cargoArgs += "--release" }

    Write-Host "cargo $($cargoArgs -join ' ')" -ForegroundColor DarkGray
    & cargo @cargoArgs

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Host $Label validation failed (exit code $LASTEXITCODE)"
        exit $LASTEXITCODE
    }
}

Invoke-CargoCheckVariant -Label "default Rust"
Invoke-CargoCheckVariant -Label "linux_compat feature matrix" -Features "linux_compat telemetry"
Invoke-CargoCheckVariant -Label "vfs feature matrix" -Features "vfs telemetry"
Invoke-CargoCheckVariant -Label "posix process feature matrix" -Features "posix_process telemetry"
Invoke-CargoCheckVariant -Label "posix process/signal minimal matrix" -Features "posix_process posix_signal posix_time telemetry"
Invoke-CargoCheckVariant -Label "posix net feature matrix" -Features "posix_net telemetry"
Invoke-CargoCheckVariant -Label "posix fs/net feature matrix" -Features "posix_fs posix_net telemetry"
Invoke-CargoCheckVariant -Label "vfs fs feature matrix" -Features "vfs posix_fs telemetry"
Invoke-CargoCheckVariant -Label "posix process/signal feature matrix" -Features "vfs posix_fs posix_process posix_signal posix_time telemetry"
Invoke-CargoCheckVariant -Label "posix time feature matrix" -Features "posix_time telemetry"
Invoke-CargoCheckVariant -Label "integrated posix feature matrix" -Features "vfs posix_fs posix_net posix_process posix_signal posix_time telemetry"

$pester = Get-Command Invoke-Pester -ErrorAction SilentlyContinue
if (-not $pester) {
    Write-Warning "Pester not found; skipping PowerShell host contract tests."
    Write-Host "Host validation completed successfully." -ForegroundColor Green
    exit 0
}

$pesterPath = Join-Path $PSScriptRoot "..\tests\powershell"
$pesterParams = @{
    Path = $pesterPath
    PassThru = $true
}

if ($Filter -ne "") {
    $pesterParams["TestName"] = "*$Filter*"
}

Write-Host "Invoke-Pester $pesterPath" -ForegroundColor DarkGray
$pesterResult = Invoke-Pester @pesterParams
if (-not $pesterResult -or $pesterResult.FailedCount -gt 0) {
    Write-Error "Host PowerShell contract tests failed."
    exit 1
}

Write-Host "Host validation completed successfully." -ForegroundColor Green
