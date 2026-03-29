param(
    [switch]$SkipHostTests,
    [string]$SoakProfile = "release",
    [int]$SoakRounds = 12,
    [int]$SoakMemoryMb = 1024,
    [int]$SoakCores = 2,
    [double]$SoakChaosRate = 0.30,
    [int]$SoakRoundTimeoutSec = 75,
    [ValidateSet("direct","iso")]
    [string]$SoakBootMode = "direct",
    [string]$SoakIsoPath = "",
    [switch]$SoakBuildIso,
    [string]$SoakLimineBinDir = "",
    [string]$SoakIsoName = "hypercore.iso",
    [switch]$SoakAutoFetchLimine,
    [string]$SoakLimineVersion = "latest",
    [string]$SoakLimineCacheDir = "artifacts/limine/cache",
    [string]$SoakLimineOutDir = "artifacts/limine/bin",
    [switch]$SoakAllowBuildLimine,
    [switch]$SoakAllowTimeoutSuccess,
    [int]$RecoveryMinSuccessfulBoots = 8,
    [int]$RecoveryWindow = 5
)

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "==> P0 Release Acceptance" -ForegroundColor Cyan
Write-Host "SoakProfile: $SoakProfile, SoakRounds: $SoakRounds"

$cmd = @(
    "-ExecutionPolicy", "Bypass",
    "-File", ".\scripts\p0_readiness_gate.ps1",
    "-RunSoakMatrix",
    "-RequireSoakArtifacts",
    "-StrictRecoveryGate",
    "-SoakProfile", "$SoakProfile",
    "-SoakRounds", "$SoakRounds",
    "-SoakMemoryMb", "$SoakMemoryMb",
    "-SoakCores", "$SoakCores",
    "-SoakChaosRate", "$SoakChaosRate",
    "-SoakRoundTimeoutSec", "$SoakRoundTimeoutSec",
    "-SoakBootMode", "$SoakBootMode",
    "-RecoveryMinSuccessfulBoots", "$RecoveryMinSuccessfulBoots",
    "-RecoveryWindow", "$RecoveryWindow"
)
if (-not [string]::IsNullOrWhiteSpace($SoakIsoPath)) {
    $cmd += "-SoakIsoPath"
    $cmd += "$SoakIsoPath"
}
if ($SoakBuildIso) {
    $cmd += "-SoakBuildIso"
    if (-not [string]::IsNullOrWhiteSpace($SoakLimineBinDir)) {
        $cmd += "-SoakLimineBinDir"
        $cmd += "$SoakLimineBinDir"
    }
    if (-not [string]::IsNullOrWhiteSpace($SoakIsoName)) {
        $cmd += "-SoakIsoName"
        $cmd += "$SoakIsoName"
    }
}
if ($SoakAutoFetchLimine) {
    $cmd += "-SoakAutoFetchLimine"
    if (-not [string]::IsNullOrWhiteSpace($SoakLimineVersion)) {
        $cmd += "-SoakLimineVersion"
        $cmd += "$SoakLimineVersion"
    }
    if (-not [string]::IsNullOrWhiteSpace($SoakLimineCacheDir)) {
        $cmd += "-SoakLimineCacheDir"
        $cmd += "$SoakLimineCacheDir"
    }
    if (-not [string]::IsNullOrWhiteSpace($SoakLimineOutDir)) {
        $cmd += "-SoakLimineOutDir"
        $cmd += "$SoakLimineOutDir"
    }
    if ($SoakAllowBuildLimine) {
        $cmd += "-SoakAllowBuildLimine"
    }
}
if ($SoakAllowTimeoutSuccess) {
    $cmd += "-SoakAllowTimeoutSuccess"
}

if ($SkipHostTests) {
    $cmd += "-SkipHostTests"
}

& powershell @cmd
if ($LASTEXITCODE -ne 0) {
    throw "p0_release_acceptance failed"
}

Write-Host "P0 release acceptance completed." -ForegroundColor Green
