param(
  [ValidateSet("quick","strict")]
  [string]$Profile = "quick",
  [switch]$NoLock
)
. (Join-Path $PSScriptRoot "..\lib\QuickWrapper.Common.ps1")
$extraArgs = @()
if ($NoLock) { $extraArgs += "-NoLock" }
Invoke-HypercoreCommandWrapper -ScriptRoot $PSScriptRoot -Command "dashboard-agent-bg" -Profile $Profile -ExtraArgs $extraArgs
