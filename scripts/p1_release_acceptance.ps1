param(
    [switch]$SkipHostTests,
    [switch]$AllowBaselineBootstrap,
    [ValidateSet("direct","iso")]
    [string]$QemuBootMode = "direct",
    [string]$QemuIsoPath = "",
    [switch]$QemuBuildIso,
    [string]$QemuLimineBinDir = "",
    [string]$QemuIsoName = "hypercore.iso",
    [switch]$QemuAutoFetchLimine,
    [string]$QemuLimineVersion = "latest",
    [string]$QemuLimineCacheDir = "artifacts/limine/cache",
    [string]$QemuLimineOutDir = "artifacts/limine/bin",
    [switch]$QemuAllowBuildLimine,
    [switch]$QemuAllowTimeoutSuccess,
    [string]$AbStateJson = "artifacts/boot_ab/state.json",
    [string]$OutDir = "reports/p1_release_acceptance",
    [int]$SoakRounds = 60,
    [int]$SoakTimeoutSec = 300,
    [int]$QemuRounds = 20,
    [string]$QemuMemoryMb = "1024,2048",
    [string]$QemuCores = "2,4",
    [double]$QemuChaosRate = 0.35,
    [int]$QemuTimeoutSec = 60,
    [switch]$QemuDryRun
)

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "==> P1 Release Acceptance" -ForegroundColor Cyan
Write-Host "OutDir: $OutDir"
Write-Host "SoakRounds: $SoakRounds, QemuRounds: $QemuRounds"

$cmd = @(
    "scripts/p1_ops_gate.py",
    "--soak-rounds", "$SoakRounds",
    "--soak-timeout-sec", "$SoakTimeoutSec",
    "--run-qemu-soak",
    "--qemu-rounds", "$QemuRounds",
    "--qemu-boot-mode", "$QemuBootMode",
    "--qemu-memory-mb", "$QemuMemoryMb",
    "--qemu-cores", "$QemuCores",
    "--qemu-chaos-rate", "$QemuChaosRate",
    "--qemu-timeout-sec", "$QemuTimeoutSec",
    "--auto-baseline",
    "--update-baseline-on-success",
    "--auto-qemu-baseline",
    "--update-qemu-baseline-on-success",
    "--auto-reboot-baseline",
    "--update-reboot-baseline-on-success",
    "--run-ab-recovery-gate",
    "--ab-state-json", "$AbStateJson",
    "--max-failure-increase", "0",
    "--max-failure-rate-increase-pctpoint", "0",
    "--max-avg-duration-increase-pct", "35",
    "--max-p95-duration-increase-pct", "45",
    "--max-max-duration-increase-pct", "60",
    "--max-qemu-failed-rounds-increase", "0",
    "--max-qemu-expected-success-drop", "0",
    "--max-reboot-failures-increase", "0",
    "--max-reboot-successful-rounds-drop", "0",
    "--out-dir", "$OutDir"
)
if (-not [string]::IsNullOrWhiteSpace($QemuIsoPath)) {
    $cmd += "--qemu-iso-path"
    $cmd += "$QemuIsoPath"
}
if ($QemuBuildIso) {
    $cmd += "--qemu-build-iso"
    if (-not [string]::IsNullOrWhiteSpace($QemuLimineBinDir)) {
        $cmd += "--qemu-limine-bin-dir"
        $cmd += "$QemuLimineBinDir"
    }
    if (-not [string]::IsNullOrWhiteSpace($QemuIsoName)) {
        $cmd += "--qemu-iso-name"
        $cmd += "$QemuIsoName"
    }
}
if ($QemuAutoFetchLimine) {
    $cmd += "--qemu-auto-fetch-limine"
    $cmd += "--qemu-limine-version"
    $cmd += "$QemuLimineVersion"
    $cmd += "--qemu-limine-cache-dir"
    $cmd += "$QemuLimineCacheDir"
    $cmd += "--qemu-limine-out-dir"
    $cmd += "$QemuLimineOutDir"
    if ($QemuAllowBuildLimine) {
        $cmd += "--qemu-allow-build-limine"
    }
}
if ($QemuAllowTimeoutSuccess) {
    $cmd += "--qemu-allow-timeout-success"
}

if ($SkipHostTests) {
    $cmd += "--skip-host-tests"
}
if ($QemuDryRun) {
    $cmd += "--qemu-dry-run"
} elseif (-not $AllowBaselineBootstrap) {
    $cmd += "--require-qemu-baseline"
    $cmd += "--require-reboot-baseline"
}
if (-not $QemuDryRun) {
    $cmd += "--require-ab-recovery-gate"
    $cmd += "--require-ab-pending-cleared"
}

& python @cmd
if ($LASTEXITCODE -ne 0) {
    throw "p1_release_acceptance failed"
}

Write-Host "P1 release acceptance completed." -ForegroundColor Green
