param(
  [ValidateSet("quick","strict")]
  [string]$Profile = "quick",
  [switch]$NoLock
)
. (Join-Path $PSScriptRoot "..\lib\QuickWrapper.Common.ps1")
Invoke-QuickActionWrapper -ScriptRoot $PSScriptRoot -Action "agent-service-start-unsafe" -Profile $Profile -NoLock:$NoLock
