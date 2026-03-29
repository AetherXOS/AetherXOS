param(
  [ValidateSet("quick","strict")]
  [string]$Profile = "quick",
  [switch]$NoLock,
  [switch]$SkipConfirm
)
. (Join-Path $PSScriptRoot "..\lib\QuickWrapper.Common.ps1")
Invoke-QuickActionWrapper -ScriptRoot $PSScriptRoot -Action "environment-audit" -Profile $Profile -NoLock:$NoLock -SkipConfirm:$SkipConfirm
