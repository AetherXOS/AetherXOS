param(
  [ValidateSet("quick","strict")]
  [string]$Profile = "quick",
  [switch]$NoLock,
  [switch]$SkipConfirm
)
. (Join-Path $PSScriptRoot "lib/QuickWrapper.Common.ps1")
Invoke-QuickActionWrapper -ScriptRoot $PSScriptRoot -Action "guided-bootstrap" -Profile $Profile -NoLock:$NoLock -SkipConfirm:$SkipConfirm
