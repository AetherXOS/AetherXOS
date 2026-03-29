param(
  [switch]$NoLock,
  [switch]$Offline
)
. (Join-Path $PSScriptRoot "..\lib\QuickWrapper.Common.ps1")
$extraArgs = @()
if ($NoLock) { $extraArgs += "-NoLock" }
if ($Offline) { $extraArgs += "-Offline" }
Invoke-HypercoreCommandWrapper -ScriptRoot $PSScriptRoot -Command "install-deno" -ExtraArgs $extraArgs
