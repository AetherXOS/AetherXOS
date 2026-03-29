function Register-HcPluginCommand {
    param(
        [Parameter(Mandatory=$true)][string]$Name,
        [string]$Description = "",
        [string]$Version = "0.1.0",
        [string]$MinApiVersion = "1.0",
        [Parameter(Mandatory=$true)][scriptblock]$Action
    )
    $script:PluginRegistry[$Name] = [ordered]@{
        name = $Name
        description = $Description
        version = $Version
        min_api_version = $MinApiVersion
        action = $Action
    }
}

function Load-Plugins {
    param($Settings)
    $dir = $Settings.plugin_dir
    if (-not (Test-Path $dir)) { return }
    $files = Get-ChildItem -Path $dir -Filter "*.ps1" -File | Sort-Object Name
    foreach ($f in $files) {
        $manifestPath = [System.IO.Path]::ChangeExtension($f.FullName, ".json")
        if (-not (Test-Path $manifestPath)) {
            Fail-Hc -Code "plugin_invalid" -Message ("plugin manifest missing: {0}" -f $manifestPath)
        }
        $m = Get-HcJsonFile -Path $manifestPath
        foreach ($req in @("name","version","min_api_version","checksum_sha256")) {
            if (-not $m.PSObject.Properties.Name.Contains($req)) {
                Fail-Hc -Code "plugin_invalid" -Message ("plugin manifest missing field {0}: {1}" -f $req, $manifestPath)
            }
        }
        $hash = (Get-FileHash -Algorithm SHA256 -Path $f.FullName).Hash.ToLowerInvariant()
        $expected = ([string]$m.checksum_sha256).ToLowerInvariant()
        if ($hash -ne $expected) {
            Fail-Hc -Code "plugin_invalid" -Message ("plugin checksum mismatch: {0}" -f $f.Name)
        }
        . $f.FullName
    }
}

function Run-Plugins {
    param($Settings)
    Load-Plugins -Settings $Settings
    if ($script:PluginRegistry.Count -eq 0) {
        Write-HcStep "hypercore:$Command" (Get-Msg "plugins_none_found")
        return
    }
    foreach ($k in ($script:PluginRegistry.Keys | Sort-Object)) {
        $p = $script:PluginRegistry[$k]
        $compatible = ([version]$script:PluginApiVersion -ge [version]$p.min_api_version)
        $tag = if ($compatible) { "compatible" } else { "incompatible" }
        Write-Host ("- {0} v{1} ({2}): {3}" -f $p.name, $p.version, $tag, $p.description)
    }
}

function Run-PluginByName {
    param($Settings, [string]$Name)
    if (-not $Name) { Fail-Hc -Code "plugin_invalid" -Message "run-plugin requires -PluginName" }
    Load-Plugins -Settings $Settings
    if (-not $script:PluginRegistry.ContainsKey($Name)) {
        Fail-Hc -Code "plugin_invalid" -Message ("plugin not found: {0}" -f $Name)
    }
    $p = $script:PluginRegistry[$Name]
    if ([version]$script:PluginApiVersion -lt [version]$p.min_api_version) {
        Fail-Hc -Code "plugin_invalid" -Message ("plugin {0} requires api {1}, current {2}" -f $p.name, $p.min_api_version, $script:PluginApiVersion)
    }
    & $p.action
}
