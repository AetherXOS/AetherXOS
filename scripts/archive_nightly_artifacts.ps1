param(
    [string]$ArchiveRoot = "artifacts/nightly_runs",
    [string]$RunId = "",
    [string[]]$SourcePaths = @(
        "reports/p0_p1_nightly",
        "reports/p1_nightly",
        "reports/p1_release_acceptance",
        "reports/p1_ops_gate",
        "reports/ab_slot_flip",
        "reports/ab_boot_recovery_gate",
        "reports/reboot_recovery_gate",
        "artifacts/qemu_soak",
        "artifacts/boot_ab"
    )
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($RunId)) {
    $RunId = Get-Date -Format "yyyyMMdd_HHmmss"
}

$destRoot = Join-Path $ArchiveRoot $RunId
New-Item -ItemType Directory -Path $destRoot -Force | Out-Null

$copied = @()
$missing = @()
foreach ($source in $SourcePaths) {
    if (Test-Path $source) {
        $leaf = Split-Path -Path $source -Leaf
        $dest = Join-Path $destRoot $leaf
        Copy-Item -Path $source -Destination $dest -Recurse -Force
        $copied += $source
    } else {
        $missing += $source
    }
}

$manifest = [ordered]@{
    run_id = $RunId
    created_utc = (Get-Date).ToUniversalTime().ToString("o")
    copied = $copied
    missing = $missing
}
$manifestPath = Join-Path $destRoot "manifest.json"
$manifest | ConvertTo-Json -Depth 6 | Set-Content -Path $manifestPath -Encoding utf8

Write-Host "archive_root=$destRoot"
Write-Host "manifest=$manifestPath"
