function Initialize-ConfigEngine {
    param(
        [string]$ConfigPath,
        [string]$ScriptRoot
    )
    $resolvedConfigPath = $ConfigPath
    $resolvedScriptRoot = $ScriptRoot
    try {
        $resolvedConfigPath = (Resolve-Path -Path $ConfigPath -ErrorAction Stop).Path
    } catch {}
    try {
        $resolvedScriptRoot = (Resolve-Path -Path $ScriptRoot -ErrorAction Stop).Path
    } catch {}
    $script:HcConfigPath = $resolvedConfigPath
    $script:HcScriptRoot = $resolvedScriptRoot
    $script:ConfigFieldSpecs = @()
    $script:ConfigFieldMap = @{}
    $script:ConfigFieldMetaPath = Join-Path $resolvedScriptRoot "config/hypercore.config_fields.json"
    $script:ConfigFieldMeta = @{}
    $script:ConfigFieldPatternMeta = @()
    $script:ConfigScanCache = @{}
}

function Get-ConfigScanCacheStamp([string]$path) {
    if (-not $path) { return "missing::<empty>" }
    if (-not (Test-Path $path)) { return ("missing::{0}" -f $path) }
    try {
        $item = Get-Item -LiteralPath $path -ErrorAction Stop
        return ("{0}|{1}|{2}" -f [string]$item.FullName, [string]$item.Length, [string]$item.LastWriteTimeUtc.Ticks)
    } catch {
        return ("err::{0}" -f $path)
    }
}

function Get-OrSet-ConfigScanCache([string]$slot, [string]$stamp, [scriptblock]$producer) {
    if (-not $script:ConfigScanCache) { $script:ConfigScanCache = @{} }
    if ($script:ConfigScanCache.ContainsKey($slot)) {
        $entry = $script:ConfigScanCache[$slot]
        if ($entry -and ($entry.stamp -eq $stamp)) {
            return $entry.value
        }
    }
    $value = & $producer
    $script:ConfigScanCache[$slot] = [ordered]@{
        stamp = $stamp
        value = $value
    }
    return $value
}

function Load-AgentConfig {
    if (-not (Test-Path $script:HcConfigPath)) {
        throw "config file missing: $($script:HcConfigPath)"
    }
    $cfg = (Get-Content -Raw -Path $script:HcConfigPath | ConvertFrom-Json)
    $changed = $false
    if (-not $cfg.PSObject.Properties.Name.Contains("build")) {
        $cfg | Add-Member -NotePropertyName build -NotePropertyValue ([ordered]@{}) -Force
        $changed = $true
    }
    if (-not $cfg.build.PSObject.Properties.Name.Contains("cargo_features")) {
        $cfg.build | Add-Member -NotePropertyName cargo_features -NotePropertyValue "" -Force
        $changed = $true
    }
    if (-not $cfg.build.PSObject.Properties.Name.Contains("cargo_no_default_features")) {
        $cfg.build | Add-Member -NotePropertyName cargo_no_default_features -NotePropertyValue $false -Force
        $changed = $true
    }
    if ($changed) {
        Save-AgentConfig -cfgObj $cfg
    }
    return $cfg
}

function Save-AgentConfig($cfgObj) {
    $json = $cfgObj | ConvertTo-Json -Depth 24
    Set-Content -Path $script:HcConfigPath -Value $json -Encoding UTF8
}

function Get-NestedValue($obj, [string]$path) {
    if (-not $obj) { return $null }
    $parts = @($path -split "\.")
    $cur = $obj
    foreach ($part in $parts) {
        if (-not $cur -or -not $cur.PSObject.Properties.Name.Contains($part)) { return $null }
        $cur = $cur.$part
    }
    return $cur
}

function Set-NestedValue($obj, [string]$path, $value) {
    $parts = @($path -split "\.")
    $cur = $obj
    for ($i = 0; $i -lt ($parts.Count - 1); $i++) {
        $part = [string]$parts[$i]
        $next = $null
        if ($cur.PSObject.Properties.Name.Contains($part)) {
            $next = $cur.$part
        }
        if ($null -eq $next) {
            $next = [pscustomobject]@{}
            if ($cur.PSObject.Properties.Name.Contains($part)) {
                $cur.$part = $next
            } else {
                $cur | Add-Member -NotePropertyName $part -NotePropertyValue $next -Force
            }
        }
        $cur = $next
    }
    $leaf = [string]$parts[$parts.Count - 1]
    if ($cur.PSObject.Properties.Name.Contains($leaf)) {
        $cur.$leaf = $value
    } else {
        $cur | Add-Member -NotePropertyName $leaf -NotePropertyValue $value -Force
    }
}

function Convert-ConfigValueBySpec($spec, $rawValue) {
    $tp = [string]$spec.type
    switch ($tp) {
        "bool" {
            if ($rawValue -is [bool]) { return [bool]$rawValue }
            $s = [string]$rawValue
            if ($s -eq "true" -or $s -eq "1") { return $true }
            if ($s -eq "false" -or $s -eq "0") { return $false }
            throw ("invalid boolean value for {0}" -f [string]$spec.path)
        }
        "int" {
            $v = [int]$rawValue
            if ($spec.PSObject.Properties.Name.Contains("min") -and $v -lt [int]$spec.min) { throw ("value below min for {0}" -f [string]$spec.path) }
            if ($spec.PSObject.Properties.Name.Contains("max") -and $v -gt [int]$spec.max) { throw ("value above max for {0}" -f [string]$spec.path) }
            return $v
        }
        "float" {
            $v = [double]$rawValue
            if ($spec.PSObject.Properties.Name.Contains("min") -and $v -lt [double]$spec.min) { throw ("value below min for {0}" -f [string]$spec.path) }
            if ($spec.PSObject.Properties.Name.Contains("max") -and $v -gt [double]$spec.max) { throw ("value above max for {0}" -f [string]$spec.path) }
            return $v
        }
        "string" {
            $v = [string]$rawValue
            if ($spec.PSObject.Properties.Name.Contains("choices")) {
                $choices = @($spec.choices | ForEach-Object { [string]$_ })
                if (-not ($choices -contains $v)) {
                    throw ("value not in choices for {0}" -f [string]$spec.path)
                }
            }
            return $v
        }
        default { throw ("unsupported config field type: {0}" -f $tp) }
    }
}

function Get-ConfigGroupForPath([string]$path) {
    if (-not $path) { return "General" }
    $head = [string]($path -split "\.")[0]
    switch ($head) {
        "profiles" { return "Profiles" }
        "execution" { return "Execution" }
        "agent" { return "Agent" }
        "boot" { return "Boot" }
        "build" { return "Build" }
        "ui" { return "UI Features" }
        "paths" { return "Paths" }
        "cleanup" { return "Cleanup" }
        "install" { return "Install" }
        "diagnostics" { return "Diagnostics" }
        default { return "General" }
    }
}

function Get-ConfigLabelForPath([string]$path) {
    if (-not $path) { return "Config Field" }
    $parts = @($path -split "\.")
    $normalized = @()
    foreach ($part in $parts) {
        $s = [string]$part
        $s = ($s -replace "_", " ")
        if ($s.Length -gt 0) {
            $s = $s.Substring(0,1).ToUpperInvariant() + $s.Substring(1)
        }
        $normalized += $s
    }
    return [string]::Join(" / ", $normalized)
}

function Get-ConfigTypeForValue($value) {
    if ($value -is [bool]) { return "bool" }
    if ($value -is [int] -or $value -is [long]) { return "int" }
    if ($value -is [double] -or $value -is [decimal] -or $value -is [single]) { return "float" }
    return "string"
}

function Is-ConfigSensitivePath([string]$path) {
    $p = [string]$path
    $lower = $p.ToLowerInvariant()
    return ($lower -match "(^|\.)(token|auth_token|secret|password|private_key|api_key)($|\.)")
}

function Load-ConfigFieldMeta {
    if (-not (Test-Path $script:ConfigFieldMetaPath)) { return @{} }
    try {
        $raw = Get-Content -Raw -Path $script:ConfigFieldMetaPath | ConvertFrom-Json
        $map = @{}
        $patterns = @()
        if ($raw -and $raw.PSObject.Properties.Name.Contains("fields")) {
            foreach ($p in $raw.fields.PSObject.Properties) {
                $map[[string]$p.Name] = $p.Value
            }
        }
        if ($raw -and $raw.PSObject.Properties.Name.Contains("patterns")) {
            foreach ($p in $raw.patterns.PSObject.Properties) {
                $patterns += [ordered]@{
                    pattern = [string]$p.Name
                    meta = $p.Value
                }
            }
        }
        return [ordered]@{
            fields = $map
            patterns = $patterns
        }
    } catch {
        return [ordered]@{
            fields = @{}
            patterns = @()
        }
    }
}

function Get-ConfigHelpForPath([string]$path, [string]$type) {
    $p = [string]$path
    $t = [string]$type
    if ($p -match "(^|\.)(token|auth_token|password|secret|api_key)($|\.)") {
        return "Sensitive auth value. Keep it private and rotate when needed."
    }
    if ($p -match "(^|\.)(port)($|\.)") {
        return "Network/listener port setting."
    }
    if ($p -match "(^|\.)(timeout|ttl|interval|delay|cooldown|retention)($|_|\.)") {
        return "Time-based tuning value. Higher values generally reduce churn but may increase reaction latency."
    }
    if ($p -match "(^|\.)(max|min|limit|size|count|capacity|budget)($|_|\.)") {
        return "Capacity/limit tuning value."
    }
    if ($p -match "(^|\.)(profile|mode|strategy|policy)($|_|\.)") {
        return "Profile/policy selector for this subsystem."
    }
    if ($t -eq "bool") {
        return "Feature toggle."
    }
    return "Auto-discovered from runtime config."
}

function Convert-PatternToRegex([string]$pattern) {
    if (-not $pattern) { return $null }
    $escaped = [System.Text.RegularExpressions.Regex]::Escape($pattern)
    $escaped = $escaped -replace "\\\*", ".*"
    return ("^" + $escaped + "$")
}

function Get-ConfigMetaForPath([string]$path) {
    if ($script:ConfigFieldMeta -and $script:ConfigFieldMeta.ContainsKey($path)) {
        return [ordered]@{
            source = "override:field"
            meta = $script:ConfigFieldMeta[$path]
        }
    }
    foreach ($entry in @($script:ConfigFieldPatternMeta)) {
        $regex = Convert-PatternToRegex -pattern ([string]$entry.pattern)
        if ($regex -and ([string]$path -match $regex)) {
            return [ordered]@{
                source = ("override:pattern:{0}" -f [string]$entry.pattern)
                meta = $entry.meta
            }
        }
    }
    return [ordered]@{
        source = "auto"
        meta = $null
    }
}

function Add-ConfigLeafSpecs($node, [string]$prefix, [ref]$specsRef) {
    if ($null -eq $node) { return }
    if ($node -is [pscustomobject]) {
        foreach ($p in $node.PSObject.Properties) {
            $path = if ($prefix) { "{0}.{1}" -f $prefix, [string]$p.Name } else { [string]$p.Name }
            Add-ConfigLeafSpecs -node $p.Value -prefix $path -specsRef $specsRef
        }
        return
    }
    if ($node -is [System.Collections.IDictionary]) {
        foreach ($k in $node.Keys) {
            $path = if ($prefix) { "{0}.{1}" -f $prefix, [string]$k } else { [string]$k }
            Add-ConfigLeafSpecs -node $node[$k] -prefix $path -specsRef $specsRef
        }
        return
    }
    if ($node -is [System.Collections.IEnumerable] -and -not ($node -is [string])) {
        return
    }
    if (-not $prefix) { return }
    if (Is-ConfigSensitivePath -path $prefix) { return }

    $tp = Get-ConfigTypeForValue -value $node
    $spec = [ordered]@{
        path = $prefix
        type = $tp
        group = (Get-ConfigGroupForPath -path $prefix)
        label = (Get-ConfigLabelForPath -path $prefix)
        help = (Get-ConfigHelpForPath -path $prefix -type $tp)
        readonly = $false
    }
    $metaBundle = Get-ConfigMetaForPath -path $prefix
    $spec.meta_source = [string]$metaBundle.source
    $meta = $metaBundle.meta
    if ($meta) {
        foreach ($pn in $meta.PSObject.Properties.Name) {
            $spec[$pn] = $meta.$pn
        }
    }
    if ($spec.PSObject.Properties.Name.Contains("hidden") -and [bool]$spec.hidden) {
        return
    }
    $specsRef.Value += $spec
}

function Initialize-ConfigFieldCatalog($cfg) {
    $metaPayload = Load-ConfigFieldMeta
    $script:ConfigFieldMeta = @{}
    $script:ConfigFieldPatternMeta = @()
    if ($metaPayload -and $metaPayload.fields) {
        $script:ConfigFieldMeta = $metaPayload.fields
    }
    if ($metaPayload -and $metaPayload.patterns) {
        $script:ConfigFieldPatternMeta = @($metaPayload.patterns)
    }
    $specs = @()
    Add-ConfigLeafSpecs -node $cfg -prefix "" -specsRef ([ref]$specs)
    $specs = @($specs | Sort-Object group, path)
    $script:ConfigFieldSpecs = $specs
    $script:ConfigFieldMap = @{}
    foreach ($fs in $script:ConfigFieldSpecs) { $script:ConfigFieldMap[[string]$fs.path] = $fs }
}

function Get-CargoFeatureSummary {
    $repoRoot = Split-Path -Parent $script:HcScriptRoot
    $cargoPath = Join-Path $repoRoot "Cargo.toml"
    $stamp = Get-ConfigScanCacheStamp -path $cargoPath
    return Get-OrSet-ConfigScanCache -slot "cargo_feature_summary" -stamp $stamp -producer {
    if (-not (Test-Path $cargoPath)) {
        return [ordered]@{ path = $cargoPath; exists = $false; features = @(); default_features = @() }
    }

    $featureGraph = @{}
    $inFeatures = $false
    $pendingKey = ""
    $pendingValue = ""
    foreach ($line in (Get-Content -Path $cargoPath)) {
        $clean = ([string]$line -replace "#.*$", "").Trim()
        if (-not $clean) { continue }
        if ($clean -match "^\[(.+)\]$") {
            $pendingKey = ""
            $pendingValue = ""
            $section = [string]$Matches[1]
            $inFeatures = ($section -eq "features")
            continue
        }
        if (-not $inFeatures) { continue }

        if ($pendingKey) {
            $pendingValue = ($pendingValue + " " + $clean).Trim()
            if ($pendingValue -match "\]") {
                $featureGraph[$pendingKey] = Get-CargoFeatureDepsFromRaw -rawValue $pendingValue
                $pendingKey = ""
                $pendingValue = ""
            }
            continue
        }

        if ($clean -match "^([A-Za-z0-9_\-]+)\s*=\s*(.+)$") {
            $k = [string]$Matches[1]
            $v = [string]$Matches[2]
            if (($v -match "^\[") -and -not ($v -match "\]")) {
                $pendingKey = $k
                $pendingValue = $v
                continue
            }
            $featureGraph[$k] = Get-CargoFeatureDepsFromRaw -rawValue $v
        }
    }

    $defaultFeatures = @()
    if ($featureGraph.ContainsKey("default")) {
        $defaultFeatures = @($featureGraph["default"])
    }

    $rows = @()
    foreach ($name in @($featureGraph.Keys | Sort-Object)) {
        $deps = @()
        if ($featureGraph.ContainsKey($name)) { $deps = @($featureGraph[$name]) }
        $group = Get-CargoFeaturePrimaryGroup -featureName $name -featureGraph $featureGraph
        $rows += [ordered]@{
            name = [string]$name
            primary_group = [string]$group
            depends_on = @($deps | Sort-Object -Unique)
            enabled_default = ($defaultFeatures -contains [string]$name)
        }
    }

    return [ordered]@{
        path = $cargoPath
        exists = $true
        default_features = @($defaultFeatures | Sort-Object -Unique)
        features = @($rows)
    }
    }
}

function Get-CargoFeatureDepsFromRaw([string]$rawValue) {
    $deps = @()
    $matches = [System.Text.RegularExpressions.Regex]::Matches([string]$rawValue, '"([^"]+)"')
    foreach ($m in $matches) {
        $raw = [string]$m.Groups[1].Value
        $dep = Normalize-CargoFeatureRef -rawRef $raw
        if ($dep) { $deps += $dep }
    }
    return @($deps | Sort-Object -Unique)
}

function Normalize-CargoFeatureRef([string]$rawRef) {
    $s = [string]$rawRef
    if (-not $s) { return "" }
    $s = $s.Trim()
    if (-not $s) { return "" }
    if ($s.StartsWith("dep:")) { $s = $s.Substring(4) }
    if ($s.Contains("/")) { $s = ($s -split "/", 2)[0] }
    if ($s.EndsWith("?")) { $s = $s.Substring(0, $s.Length - 1) }
    return $s.Trim()
}

function Get-CargoFeaturePrimaryGroup([string]$featureName, $featureGraph) {
    $roots = @{}
    foreach ($k in @($featureGraph.Keys)) {
        $deps = @($featureGraph[$k])
        if ($deps.Count -eq 0) {
            $roots[[string]$k] = $true
        }
    }

    $seen = @{}
    $stack = @([string]$featureName)
    $reachable = @{}
    while ($stack.Count -gt 0) {
        $cur = [string]$stack[0]
        if ($stack.Count -gt 1) {
            $stack = @($stack[1..($stack.Count - 1)])
        } else {
            $stack = @()
        }
        if ($seen.ContainsKey($cur)) { continue }
        $seen[$cur] = $true
        if ($roots.ContainsKey($cur)) { $reachable[$cur] = $true }
        if ($featureGraph.ContainsKey($cur)) {
            foreach ($dep in @($featureGraph[$cur])) {
                if ($featureGraph.ContainsKey([string]$dep)) {
                    $stack += [string]$dep
                }
            }
        }
    }

    $candidates = @($reachable.Keys | Sort-Object)
    if ($candidates.Count -gt 0) { return [string]$candidates[0] }
    if ([string]$featureName -match "^([A-Za-z0-9]+)_") { return [string]$Matches[1] }
    return [string]$featureName
}

function Parse-CargoFeatureCsv([string]$raw) {
    if (-not $raw) { return @() }
    $out = @()
    foreach ($part in @($raw -split ",")) {
        $s = [string]$part
        $s = $s.Trim()
        if ($s) { $out += $s }
    }
    return @($out | Sort-Object -Unique)
}

function Resolve-CargoFeatureClosure($rows, [string[]]$seedFeatures) {
    $index = @{}
    foreach ($row in @($rows)) {
        $nm = [string]$row.name
        if ($nm) { $index[$nm] = $row }
    }
    $selected = @{}
    $queue = New-Object System.Collections.Generic.Queue[string]
    foreach ($f in @($seedFeatures)) {
        $name = [string]$f
        if (-not $name -or $name -eq "default") { continue }
        if (-not $selected.ContainsKey($name)) {
            $selected[$name] = $true
            $queue.Enqueue($name)
        }
    }
    while ($queue.Count -gt 0) {
        $cur = $queue.Dequeue()
        if (-not $index.ContainsKey($cur)) { continue }
        $deps = @($index[$cur].depends_on)
        foreach ($dep in $deps) {
            $dn = [string]$dep
            if (-not $dn -or $dn -eq "default") { continue }
            if (-not $selected.ContainsKey($dn)) {
                $selected[$dn] = $true
                $queue.Enqueue($dn)
            }
        }
    }
    return @($selected.Keys | Sort-Object)
}

function Get-BuildGoalTokens([string]$goal) {
    $g = [string]$goal
    switch ($g) {
        "boot_min" { return @("core", "kernel", "boot", "scheduler", "library") }
        "linux_full" { return @("linux", "posix", "syscall", "vfs", "network", "driver", "ipc", "security", "telemetry", "scheduler", "library") }
        "release_hardening" { return @("security", "telemetry", "watchdog", "driver", "network", "vfs", "linux", "scheduler", "library") }
        default { return @("core", "kernel", "scheduler", "library", "vfs", "linux") }
    }
}

function Select-BuildFeatureProfile([string]$goal = "linux_full", [bool]$minimal = $false) {
    $cargo = Get-CargoFeatureSummary
    $rows = @($cargo.features)
    $tokens = Get-BuildGoalTokens -goal $goal
    $seed = @()
    foreach ($row in $rows) {
        $name = [string]$row.name
        if (-not $name -or $name -eq "default") { continue }
        $group = [string]$row.primary_group
        $probe = ("{0} {1}" -f $name, $group).ToLowerInvariant()
        $match = $false
        foreach ($tk in $tokens) {
            $t = [string]$tk
            if (-not $t) { continue }
            if ($probe.Contains($t.ToLowerInvariant())) {
                $match = $true
                break
            }
        }
        if ($match) { $seed += $name }
    }
    if (@($seed).Count -eq 0) {
        $seed = @($cargo.default_features | Where-Object { [string]$_ -ne "default" })
    }
    $resolved = Resolve-CargoFeatureClosure -rows $rows -seedFeatures @($seed)
    if ($minimal -and @($resolved).Count -gt 24) {
        $resolved = @($resolved | Select-Object -First 24)
    }
    return [ordered]@{
        goal = $goal
        minimal = [bool]$minimal
        no_default_features = [bool]$minimal
        selected_features = @($resolved | Sort-Object)
        selected_count = @($resolved).Count
        available_count = @($rows).Count
        rationale = @(
            "Auto-selected by goal token matching + dependency closure.",
            "Set minimal=true for smaller feature footprint and --no-default-features."
        )
    }
}

function Build-ConfigDriftReport($cfgObj) {
    $cfg = $cfgObj
    if ($null -eq $cfg) { $cfg = Load-AgentConfig }
    $currentRaw = ""
    $currentNoDefault = $false
    if ($cfg.PSObject.Properties.Name.Contains("build")) {
        if ($cfg.build.PSObject.Properties.Name.Contains("cargo_features")) {
            $currentRaw = [string]$cfg.build.cargo_features
        }
        if ($cfg.build.PSObject.Properties.Name.Contains("cargo_no_default_features")) {
            $currentNoDefault = [bool]$cfg.build.cargo_no_default_features
        }
    }
    $currentSet = @{}
    foreach ($f in @(Parse-CargoFeatureCsv -raw $currentRaw)) { $currentSet[[string]$f] = $true }

    $goals = @("boot_min", "linux_full", "release_hardening")
    $rows = @()
    foreach ($goal in $goals) {
        $rec = Select-BuildFeatureProfile -goal $goal -minimal $false
        $recommendedSet = @{}
        foreach ($rf in @($rec.selected_features)) { $recommendedSet[[string]$rf] = $true }
        $missing = @()
        $extra = @()
        foreach ($rf in @($recommendedSet.Keys)) {
            if (-not $currentSet.ContainsKey([string]$rf)) { $missing += [string]$rf }
        }
        foreach ($cf in @($currentSet.Keys)) {
            if (-not $recommendedSet.ContainsKey([string]$cf)) { $extra += [string]$cf }
        }
        $rows += [ordered]@{
            goal = $goal
            recommended_count = @($rec.selected_features).Count
            current_count = @($currentSet.Keys).Count
            missing_count = @($missing).Count
            extra_count = @($extra).Count
            no_default_features_recommended = [bool]$rec.no_default_features
            no_default_features_current = [bool]$currentNoDefault
            missing = @($missing | Sort-Object)
            extra = @($extra | Sort-Object)
        }
    }
    return [ordered]@{
        generated_utc = [DateTime]::UtcNow.ToString("o")
        current = [ordered]@{
            cargo_features = @($currentSet.Keys | Sort-Object)
            cargo_no_default_features = [bool]$currentNoDefault
        }
        goals = $rows
    }
}

function Get-RustRuntimeConfigSummary {
    $repoRoot = Split-Path -Parent $script:HcScriptRoot
    $runtimeKeyPath = Join-Path $repoRoot "src/config/runtime_key_autogen.rs"
    $stamp = Get-ConfigScanCacheStamp -path $runtimeKeyPath
    return Get-OrSet-ConfigScanCache -slot "rust_runtime_key_summary" -stamp $stamp -producer {
    if (-not (Test-Path $runtimeKeyPath)) {
        return [ordered]@{
            path = $runtimeKeyPath
            exists = $false
            categories = @()
            keys = @()
        }
    }

    $raw = Get-Content -Raw -Path $runtimeKeyPath
    $categoryMatches = [System.Text.RegularExpressions.Regex]::Matches($raw, "AUTO_RUNTIME_CONFIG_CATEGORIES:\s*&\[\s*&str\s*\]\s*=\s*&\[(.*?)\];", [System.Text.RegularExpressions.RegexOptions]::Singleline)
    $categories = @()
    if ($categoryMatches.Count -gt 0) {
        $catsBlob = [string]$categoryMatches[0].Groups[1].Value
        foreach ($m in [System.Text.RegularExpressions.Regex]::Matches($catsBlob, '"([^"]+)"')) {
            $categories += [string]$m.Groups[1].Value
        }
    }
    $categorySet = @{}
    foreach ($c in $categories) { $categorySet[[string]$c] = $true }

    $keys = @()
    $keyMatches = [System.Text.RegularExpressions.Regex]::Matches(
        $raw,
        'ConfigKeySpec\s*\{\s*key:\s*"([^"]+)",\s*value_kind:\s*ConfigValueKind::([A-Za-z0-9_]+),\s*description:\s*"([^"]*)"\s*\}'
    )
    foreach ($m in $keyMatches) {
        $key = [string]$m.Groups[1].Value
        $kind = [string]$m.Groups[2].Value
        $desc = [string]$m.Groups[3].Value
        $prefix = if ($key.Contains("_")) { ($key -split "_", 2)[0] } else { $key }
        $cat = if ($categorySet.ContainsKey($prefix)) { $prefix } else { "other" }
        $keys += [ordered]@{
            key = $key
            value_kind = $kind
            category = $cat
            description = $desc
        }
    }
    $keys = @($keys | Sort-Object category, key)
    return [ordered]@{
        path = $runtimeKeyPath
        exists = $true
        categories = @($categories | Sort-Object -Unique)
        keys = $keys
    }
    }
}

function Get-RustCompileConstSummary {
    $repoRoot = Split-Path -Parent $script:HcScriptRoot
    $constPath = Join-Path $repoRoot "src/generated_consts.rs"
    $stamp = Get-ConfigScanCacheStamp -path $constPath
    return Get-OrSet-ConfigScanCache -slot "rust_compile_const_summary" -stamp $stamp -producer {
    if (-not (Test-Path $constPath)) {
        return [ordered]@{
            path = $constPath
            exists = $false
            categories = @()
            consts = @()
            total = 0
            skipped_complex = 0
        }
    }

    $rows = @()
    $skippedComplex = 0
    $lineRegex = '^\s*pub const ([A-Z0-9_]+):\s*([^=;]+?)\s*=\s*(.+);\s*$'
    foreach ($line in (Get-Content -Path $constPath)) {
        $m = [System.Text.RegularExpressions.Regex]::Match([string]$line, $lineRegex)
        if (-not $m.Success) { continue }

        $name = [string]$m.Groups[1].Value
        $valueType = [string]$m.Groups[2].Value.Trim()
        $valueRaw = [string]$m.Groups[3].Value.Trim()
        if (-not $name) { continue }

        if ($valueType -match "\[.*\]") {
            $skippedComplex += 1
            continue
        }

        $prefix = (($name -split "_")[0] | ForEach-Object { [string]$_ })
        $category = if ($prefix) { $prefix.ToLowerInvariant() } else { "other" }
        if ($category -eq "sched") { $category = "scheduler" }
        if ($category -eq "meta") { $category = "build" }
        if ($category -eq "target") { $category = "kernel" }
        if ($category -eq "time" -or $category -eq "stack" -or $category -eq "kernel" -or $category -eq "interrupt") {
            $category = "kernel"
        }

        $value = $valueRaw
        if ($value.Length -gt 128) {
            $value = ($value.Substring(0, 125) + "...")
        }

        $rows += [ordered]@{
            name = $name
            value_type = $valueType
            value = $value
            category = $category
        }
    }

    $rows = @($rows | Sort-Object category, name)
    $categories = @($rows | ForEach-Object { [string]$_.category } | Sort-Object -Unique)
    return [ordered]@{
        path = $constPath
        exists = $true
        categories = $categories
        consts = $rows
        total = $rows.Count
        skipped_complex = $skippedComplex
    }
    }
}

function Build-ConfigPayload {
    $cfg = Load-AgentConfig
    Initialize-ConfigFieldCatalog -cfg $cfg
    $values = [ordered]@{}
    foreach ($spec in $script:ConfigFieldSpecs) {
        $p = [string]$spec.path
        $values[$p] = Get-NestedValue -obj $cfg -path $p
    }
    return [ordered]@{
        config_path = $script:HcConfigPath
        generated_utc = [DateTime]::UtcNow.ToString("o")
        values = $values
        fields = $script:ConfigFieldSpecs
        cargo = (Get-CargoFeatureSummary)
        kernel_runtime = (Get-RustRuntimeConfigSummary)
        kernel_compile = (Get-RustCompileConstSummary)
    }
}

function Apply-ConfigUpdates($updates) {
    $cfg = Load-AgentConfig
    Initialize-ConfigFieldCatalog -cfg $cfg
    $applied = @()
    foreach ($u in @($updates)) {
        $path = [string]$u.path
        if (-not $script:ConfigFieldMap.ContainsKey($path)) {
            throw ("unsupported config path: {0}" -f $path)
        }
        $spec = $script:ConfigFieldMap[$path]
        if ($spec.PSObject.Properties.Name.Contains("readonly") -and [bool]$spec.readonly) {
            throw ("readonly config path: {0}" -f $path)
        }
        $converted = Convert-ConfigValueBySpec -spec $spec -rawValue $u.value
        Set-NestedValue -obj $cfg -path $path -value $converted
        $applied += [ordered]@{ path = $path; value = $converted }
    }
    Save-AgentConfig -cfgObj $cfg
    return $applied
}

function Export-ConfigProfile([string]$profileName = "default") {
    $payload = Build-ConfigPayload
    return [ordered]@{
        schema = "hypercore.config.profile.v1"
        profile_name = $profileName
        generated_utc = [DateTime]::UtcNow.ToString("o")
        source_config_path = $payload.config_path
        values = $payload.values
        field_count = @($payload.fields).Count
    }
}

function Export-ConfigFieldOverrideTemplate([string]$mode = "minimal") {
    $cfg = Load-AgentConfig
    Initialize-ConfigFieldCatalog -cfg $cfg

    $m = [string]$mode
    if (-not $m) { $m = "minimal" }
    $m = $m.ToLowerInvariant()
    if (($m -ne "minimal") -and ($m -ne "full")) {
        throw "invalid mode; use minimal|full"
    }

    $fieldMap = [ordered]@{}
    if ($m -eq "full") {
        foreach ($spec in @($script:ConfigFieldSpecs)) {
            $path = [string]$spec.path
            if (-not $path) { continue }
            $fieldMap[$path] = [ordered]@{
                group = [string]$spec.group
                label = [string]$spec.label
                help = [string]$spec.help
                type = [string]$spec.type
            }
        }
    }

    $patterns = [ordered]@{
        "agent.*" = [ordered]@{
            group = "Agent"
            help = "Agent-related settings."
        }
        "profiles.*.round_timeout_sec" = [ordered]@{
            group = "Profiles"
            min = 5
            max = 1800
            help = "Round timeout safety range."
        }
        "*.token*" = [ordered]@{
            hidden = $true
            help = "Hide token-like paths from UI forms."
        }
    }

    return [ordered]@{
        schema = "hypercore.config_fields.override.v1"
        mode = $m
        generated_utc = [DateTime]::UtcNow.ToString("o")
        target_config_path = $script:HcConfigPath
        target_meta_path = $script:ConfigFieldMetaPath
        notes = @(
            "This file is optional. Auto-discovery works without it.",
            "Use fields.<exact_path> for precise overrides.",
            "Use patterns.<glob> to override many paths at once."
        )
        fields = $fieldMap
        patterns = $patterns
        discovered_field_count = @($script:ConfigFieldSpecs).Count
    }
}

function Import-ConfigProfile($profileObj) {
    if ($null -eq $profileObj) { throw "profile payload missing" }
    $values = $null
    if ($profileObj.PSObject.Properties.Name.Contains("values")) {
        $values = $profileObj.values
    } else {
        $values = $profileObj
    }
    $updates = @()
    foreach ($p in $values.PSObject.Properties) {
        $updates += [ordered]@{
            path = [string]$p.Name
            value = $p.Value
        }
    }
    if ($updates.Count -eq 0) { throw "profile has no values" }
    return (Apply-ConfigUpdates -updates $updates)
}

function Build-AutoPresetUpdates([string]$mode) {
    switch ($mode) {
        "balanced" {
            return @(
                @{ path = "profiles.quick.rounds"; value = 1 },
                @{ path = "profiles.quick.round_timeout_sec"; value = 30 },
                @{ path = "profiles.strict.rounds"; value = 10 },
                @{ path = "profiles.strict.round_timeout_sec"; value = 45 },
                @{ path = "execution.retry_count"; value = 2 },
                @{ path = "agent.max_concurrency"; value = 1 }
            )
        }
        "fast_dev" {
            return @(
                @{ path = "profiles.quick.rounds"; value = 1 },
                @{ path = "profiles.quick.round_timeout_sec"; value = 20 },
                @{ path = "profiles.quick.chaos_rate"; value = 0.0 },
                @{ path = "profiles.strict.rounds"; value = 4 },
                @{ path = "profiles.strict.round_timeout_sec"; value = 35 },
                @{ path = "execution.retry_count"; value = 1 },
                @{ path = "agent.max_concurrency"; value = 2 }
            )
        }
        "reliable_ci" {
            return @(
                @{ path = "profiles.quick.rounds"; value = 2 },
                @{ path = "profiles.quick.round_timeout_sec"; value = 45 },
                @{ path = "profiles.strict.rounds"; value = 12 },
                @{ path = "profiles.strict.round_timeout_sec"; value = 90 },
                @{ path = "profiles.strict.chaos_rate"; value = 0.1 },
                @{ path = "execution.retry_count"; value = 3 },
                @{ path = "execution.retry_delay_sec"; value = 3 },
                @{ path = "agent.max_concurrency"; value = 1 }
            )
        }
        default {
            throw "invalid preset mode; use balanced|fast_dev|reliable_ci"
        }
    }
}
