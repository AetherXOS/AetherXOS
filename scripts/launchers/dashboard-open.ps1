param(
  [ValidateSet("quick","strict")]
  [string]$Profile = "quick",
  [ValidateSet("auto","ui","html")]
  [string]$DashboardTarget = "auto",
  [switch]$NoLock,
  [switch]$SkipConfirm
)
. (Join-Path $PSScriptRoot "..\lib\QuickWrapper.Common.ps1")
Invoke-QuickActionWrapper -ScriptRoot $PSScriptRoot -Action "dashboard-open" -Profile $Profile -NoLock:$NoLock -SkipConfirm:$SkipConfirm -ExtraArgs @("-DashboardTarget", $DashboardTarget)
