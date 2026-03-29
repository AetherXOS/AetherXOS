param()

$ErrorActionPreference = "Stop"
$env:CARGO_INCREMENTAL = "0"

function Get-HostTriple {
    $rustInfo = & rustc -vV 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $rustInfo) {
        throw "rustc not available"
    }

    $m = $rustInfo | Select-String -Pattern '^host:\s*(.+)$' -Quiet:$false
    if (-not $m) {
        throw "unable to determine rust host triple"
    }

    return ($m -replace '^host:\s*', '')
}

function Invoke-CargoStep {
    param(
        [string]$Label,
        [string[]]$CargoArgs
    )

    Write-Host "==> $Label" -ForegroundColor Cyan
    Write-Host "cargo $($CargoArgs -join ' ')" -ForegroundColor DarkGray
    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) {
        throw "command failed: cargo $($CargoArgs -join ' ')"
    }
}

$hostTriple = Get-HostTriple

Invoke-CargoStep -Label "Tier 1 / nextest" -CargoArgs @("nextest", "run", "--config-file", ".config/nextest.toml", "--target", $hostTriple, "--test", "tier1")
Invoke-CargoStep -Label "Tier 1 / clippy" -CargoArgs @("clippy", "--all-targets", "--target", $hostTriple, "--", "-D", "warnings")
Invoke-CargoStep -Label "Tier 1 / cargo-geiger" -CargoArgs @("geiger", "--all-targets", "--target", $hostTriple)
Invoke-CargoStep -Label "Tier 1 / rudra" -CargoArgs @("rudra", "--all-targets", "--target", $hostTriple)

Write-Host "Tier 1 validation completed successfully." -ForegroundColor Green
