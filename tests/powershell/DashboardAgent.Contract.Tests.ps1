Describe "Dashboard Agent Contract" {
    BeforeAll {
        $script:RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
        $script:AgentScript = Join-Path $script:RepoRoot "scripts\dashboard_agent.ps1"
        $script:CfgTemplate = Join-Path $script:RepoRoot "scripts\config\hypercore.defaults.json"
        $script:TempCfg = Join-Path $env:TEMP ("hc_agent_contract_{0}.json" -f [guid]::NewGuid().ToString("N"))
        $script:StdOutLog = Join-Path $env:TEMP ("hc_agent_contract_out_{0}.log" -f [guid]::NewGuid().ToString("N"))
        $script:StdErrLog = Join-Path $env:TEMP ("hc_agent_contract_err_{0}.log" -f [guid]::NewGuid().ToString("N"))

        $script:Port = 17500 + (Get-Random -Minimum 0 -Maximum 700)
        $script:BaseUrl = "http://127.0.0.1:$script:Port"
        $script:ViewerToken = "viewer-contract-token"
        $script:OperatorToken = "operator-contract-token"
        $script:AdminToken = "admin-contract-token"

        function Invoke-AgentJson {
            param(
                [string]$Method,
                [string]$Path,
                [hashtable]$Headers = @{},
                [string]$Body = ""
            )
            $uri = "$script:BaseUrl/$Path".TrimEnd("/")
            $args = @{
                Method = $Method
                Uri = $uri
                Headers = $Headers
                TimeoutSec = 8
            }
            if ((Get-Command Invoke-WebRequest).Parameters.ContainsKey("SkipHttpErrorCheck")) {
                $args["SkipHttpErrorCheck"] = $true
            }
            if ((Get-Command Invoke-WebRequest).Parameters.ContainsKey("UseBasicParsing")) {
                $args["UseBasicParsing"] = $true
            }
            if ($Body) {
                $args["Body"] = $Body
                $args["ContentType"] = "application/json"
            }

            $lastErr = ""
            for ($attempt = 0; $attempt -lt 3; $attempt++) {
                try {
                    $resp = Invoke-WebRequest @args
                    $json = $null
                    if ($resp.Content) {
                        try { $json = $resp.Content | ConvertFrom-Json } catch {}
                    }
                    return [pscustomobject]@{
                        ok = $true
                        status = [int]$resp.StatusCode
                        json = $json
                        raw = $resp.Content
                    }
                } catch {
                    $webResp = $null
                    try { $webResp = $_.Exception.Response } catch {}
                    if ($webResp) {
                        $statusCode = 0
                        try { $statusCode = [int]$webResp.StatusCode } catch {}
                        $rawBody = ""
                        try {
                            $stream = $webResp.GetResponseStream()
                            if ($stream) {
                                $reader = New-Object System.IO.StreamReader($stream)
                                $rawBody = $reader.ReadToEnd()
                                $reader.Dispose()
                                $stream.Dispose()
                            }
                        } catch {}
                        $jsonErr = $null
                        if ($rawBody) {
                            try { $jsonErr = $rawBody | ConvertFrom-Json } catch {}
                        }
                        return [pscustomobject]@{
                            ok = $true
                            status = $statusCode
                            json = $jsonErr
                            raw = $rawBody
                        }
                    }
                    $lastErr = [string]$_.Exception.Message
                    Start-Sleep -Milliseconds (120 * ($attempt + 1))
                }
            }
            return [pscustomobject]@{
                ok = $false
                status = 0
                json = $null
                raw = $lastErr
            }
        }
        $script:InvokeAgentJson = ${function:Invoke-AgentJson}

        $cfg = Get-Content -Raw -Path $script:CfgTemplate | ConvertFrom-Json
        $cfg.agent.port = $script:Port
        $cfg.agent.auth_token = $script:AdminToken
        $cfg.agent.tokens.viewer = $script:ViewerToken
        $cfg.agent.tokens.operator = $script:OperatorToken
        $cfg.agent.tokens.admin = $script:AdminToken
        $cfg.agent.hosts = @(
            @{
                id = "local"
                name = "Localhost"
                url = $script:BaseUrl
                enabled = $true
                role_hint = "admin"
            }
        )
        $cfg | ConvertTo-Json -Depth 20 | Set-Content -Path $script:TempCfg -Encoding UTF8

        $args = @(
            "-ExecutionPolicy", "Bypass",
            "-File", $script:AgentScript,
            "-Port", [string]$script:Port,
            "-ConfigPath", $script:TempCfg,
            "-NoLock"
        )
        $script:AgentProcess = Start-Process -FilePath "powershell" -ArgumentList $args -PassThru -WindowStyle Hidden -RedirectStandardOutput $script:StdOutLog -RedirectStandardError $script:StdErrLog

        $healthy = $false
        for ($i = 0; $i -lt 40; $i++) {
            $r = & $script:InvokeAgentJson "GET" "health" @{} ""
            if ($r.ok -and $r.status -eq 200 -and $r.json -and $r.json.ok -eq $true) {
                $healthy = $true
                break
            }
            if ($script:AgentProcess.HasExited) { break }
            Start-Sleep -Milliseconds 300
        }

        if (-not $healthy) {
            $errTail = if (Test-Path $script:StdErrLog) { (Get-Content -Path $script:StdErrLog | Select-Object -Last 80) -join "`n" } else { "" }
            $outTail = if (Test-Path $script:StdOutLog) { (Get-Content -Path $script:StdOutLog | Select-Object -Last 80) -join "`n" } else { "" }
            throw "Dashboard agent failed to become healthy on $script:BaseUrl`nSTDERR:`n$errTail`nSTDOUT:`n$outTail"
        }
    }

    AfterAll {
        if ($script:AgentProcess -and -not $script:AgentProcess.HasExited) {
            try { Stop-Process -Id $script:AgentProcess.Id -Force -ErrorAction Stop } catch {}
        }
        foreach ($p in @($script:TempCfg, $script:StdOutLog, $script:StdErrLog)) {
            if ($p -and (Test-Path $p)) {
                Remove-Item -Force -Path $p -ErrorAction SilentlyContinue
            }
        }
    }

    It "exposes health/status with anonymous read access and standard fields" {
        $health = & $script:InvokeAgentJson "GET" "health" @{} ""
        $health.ok | Should Be $true
        $health.status | Should Be 200
        $health.json.ok | Should Be $true
        [string]$health.json.code | Should Be "ok"
        [string]$health.json.role | Should Be "anonymous"
    }

    It "rejects mutating endpoints without token (401 unauthorized)" {
        $r = & $script:InvokeAgentJson "POST" "run_async" @{} '{"action":"doctor"}'
        $r.ok | Should Be $true
        $r.status | Should Be 401
        $r.json.ok | Should Be $false
        [string]$r.json.code | Should Be "unauthorized"
        [string]$r.json.error | Should Be "unauthorized"
    }

    It "rejects viewer token for operator actions (403 forbidden_role)" {
        $headers = @{ "X-HyperCore-Token" = $script:ViewerToken }
        $r = & $script:InvokeAgentJson "POST" "run_async" $headers '{"action":"doctor"}'
        $r.ok | Should Be $true
        $r.status | Should Be 403
        $r.json.ok | Should Be $false
        [string]$r.json.code | Should Be "forbidden_role"
    }

    It "validates unknown action ids with operator token (400 unknown_action)" {
        $headers = @{ "X-HyperCore-Token" = $script:OperatorToken }
        $r = & $script:InvokeAgentJson "POST" "run_async" $headers '{"action":"not_real_action"}'
        $r.ok | Should Be $true
        $r.status | Should Be 400
        $r.json.ok | Should Be $false
        [string]$r.json.code | Should Be "unknown_action"
    }

    It "requires confirmation for HIGH risk actions" {
        $headers = @{ "X-HyperCore-Token" = $script:OperatorToken }

        $run = & $script:InvokeAgentJson "POST" "run_async" $headers '{"action":"build_iso","priority":"high"}'
        $run.ok | Should Be $true
        $run.status | Should Be 409
        [string]$run.json.code | Should Be "confirmation_required"

        $confirm = & $script:InvokeAgentJson "POST" "confirm/request" $headers '{"action":"build_iso"}'
        $confirm.ok | Should Be $true
        $confirm.status | Should Be 202
        [string]$confirm.json.code | Should Be "ok"
        [string]$confirm.json.confirmation.id | Should Not BeNullOrEmpty
    }

    It "returns aggregated host status rows" {
        $r = & $script:InvokeAgentJson "GET" "status/hosts" @{} ""
        $r.ok | Should Be $true
        $r.status | Should Be 200
        $r.json.ok | Should Be $true
        @($r.json.hosts).Count | Should BeGreaterThan 0
        [string]$r.json.hosts[0].id | Should Not BeNullOrEmpty
    }

    It "dispatches local async action via orchestration endpoint" {
        $headers = @{ "X-HyperCore-Token" = $script:OperatorToken }
        $r = & $script:InvokeAgentJson "POST" "dispatch/run_async" $headers '{"host_id":"local","action":"doctor","priority":"normal"}'
        $r.ok | Should Be $true
        $r.status | Should Be 202
        $r.json.ok | Should Be $true
        [string]$r.json.dispatched | Should Be "local"
        [string]$r.json.job.id | Should Not BeNullOrEmpty
        $script:LastDispatchedJob = [string]$r.json.job.id
    }

    It "lists local dispatched jobs through federation endpoint" {
        $r = & $script:InvokeAgentJson "GET" "dispatch/jobs?host_id=local" @{} ""
        $r.ok | Should Be $true
        $r.status | Should Be 200
        $r.json.ok | Should Be $true
        [string]$r.json.host_id | Should Be "local"
        $r.json.PSObject.Properties.Name -contains "jobs" | Should Be $true
    }

    It "returns 404 for unknown local dispatched job id" {
        $r = & $script:InvokeAgentJson "GET" "dispatch/job?host_id=local&id=missing_job" @{} ""
        $r.ok | Should Be $true
        $r.status | Should Be 404
        [string]$r.json.code | Should Be "job_not_found"
    }

    It "validates dispatch cancel payload" {
        $headers = @{ "X-HyperCore-Token" = $script:AdminToken }
        $r = & $script:InvokeAgentJson "POST" "dispatch/job/cancel" $headers '{"host_id":"local"}'
        $r.ok | Should Be $true
        $r.status | Should Be 400
        [string]$r.json.code | Should Be "invalid_payload"
    }

    It "applies role policy and blocks operator on admin-only actions" {
        $headers = @{ "X-HyperCore-Token" = $script:OperatorToken }
        $r = & $script:InvokeAgentJson "POST" "run_async" $headers '{"action":"doctor_fix"}'
        $r.ok | Should Be $true
        $r.status | Should Be 403
        [string]$r.json.code | Should Be "forbidden_action_policy"
    }

    It "returns plugin health payload with summary contract" {
        $plugins = & $script:InvokeAgentJson "GET" "plugins/health" @{} ""
        if (-not $plugins.ok) {
            $stderr = if (Test-Path $script:StdErrLog) { (Get-Content $script:StdErrLog | Select-Object -Last 60) -join "`n" } else { "" }
            $stdout = if (Test-Path $script:StdOutLog) { (Get-Content $script:StdOutLog | Select-Object -Last 60) -join "`n" } else { "" }
            throw "plugins/health request failed; process_exited=$($script:AgentProcess.HasExited); raw=$($plugins.raw)`nSTDERR:`n$stderr`nSTDOUT:`n$stdout"
        }
        $plugins.ok | Should Be $true
        $plugins.status | Should Be 200
        $plugins.json.ok | Should Be $true
        $plugins.json.plugins.PSObject.Properties.Name -contains "summary" | Should Be $true
        $plugins.json.plugins.summary.PSObject.Properties.Name -contains "total" | Should Be $true
        $plugins.json.plugins.summary.PSObject.Properties.Name -contains "ok" | Should Be $true
        $plugins.json.plugins.summary.PSObject.Properties.Name -contains "fail" | Should Be $true
    }

    It "restricts job cancel endpoint to admin role" {
        $viewerHeaders = @{ "X-HyperCore-Token" = $script:ViewerToken }
        $r = & $script:InvokeAgentJson "POST" "job/cancel" $viewerHeaders '{"id":"missing"}'
        if (-not $r.ok) {
            $stderr = if (Test-Path $script:StdErrLog) { (Get-Content $script:StdErrLog | Select-Object -Last 60) -join "`n" } else { "" }
            $stdout = if (Test-Path $script:StdOutLog) { (Get-Content $script:StdOutLog | Select-Object -Last 60) -join "`n" } else { "" }
            throw "job/cancel request failed; process_exited=$($script:AgentProcess.HasExited); raw=$($r.raw)`nSTDERR:`n$stderr`nSTDOUT:`n$stdout"
        }
        $r.ok | Should Be $true
        $r.status | Should Be 403
        [string]$r.json.code | Should Be "forbidden_role"
    }

    It "allows mutating endpoints without token when no-safe mode is enabled" {
        $tempCfgUnsafe = Join-Path $env:TEMP ("hc_agent_contract_unsafe_{0}.json" -f [guid]::NewGuid().ToString("N"))
        $stdoutUnsafe = Join-Path $env:TEMP ("hc_agent_contract_unsafe_out_{0}.log" -f [guid]::NewGuid().ToString("N"))
        $stderrUnsafe = Join-Path $env:TEMP ("hc_agent_contract_unsafe_err_{0}.log" -f [guid]::NewGuid().ToString("N"))
        $unsafePort = 18300 + (Get-Random -Minimum 0 -Maximum 700)
        $unsafeBase = "http://127.0.0.1:$unsafePort"
        $unsafeProc = $null
        try {
            $cfgUnsafe = Get-Content -Raw -Path $script:CfgTemplate | ConvertFrom-Json
            $cfgUnsafe.agent.port = $unsafePort
            $cfgUnsafe.agent.auth_mode = "unsafe"
            $cfgUnsafe.agent.auth_token = "admin-unsafe-contract-token"
            $cfgUnsafe.agent.tokens.viewer = "viewer-unsafe-contract-token"
            $cfgUnsafe.agent.tokens.operator = "operator-unsafe-contract-token"
            $cfgUnsafe.agent.tokens.admin = "admin-unsafe-contract-token"
            $cfgUnsafe | ConvertTo-Json -Depth 20 | Set-Content -Path $tempCfgUnsafe -Encoding UTF8

            $argsUnsafe = @(
                "-ExecutionPolicy", "Bypass",
                "-File", $script:AgentScript,
                "-Port", [string]$unsafePort,
                "-ConfigPath", $tempCfgUnsafe,
                "-NoLock"
            )
            $unsafeProc = Start-Process -FilePath "powershell" -ArgumentList $argsUnsafe -PassThru -WindowStyle Hidden -RedirectStandardOutput $stdoutUnsafe -RedirectStandardError $stderrUnsafe

            function Invoke-UnsafeAgentJson {
                param([string]$Method, [string]$Path, [hashtable]$Headers = @{}, [string]$Body = "")
                $uri = "$unsafeBase/$Path".TrimEnd("/")
                $args = @{ Method = $Method; Uri = $uri; Headers = $Headers; TimeoutSec = 8 }
                if ((Get-Command Invoke-WebRequest).Parameters.ContainsKey("SkipHttpErrorCheck")) { $args["SkipHttpErrorCheck"] = $true }
                if ((Get-Command Invoke-WebRequest).Parameters.ContainsKey("UseBasicParsing")) { $args["UseBasicParsing"] = $true }
                if ($Body) { $args["Body"] = $Body; $args["ContentType"] = "application/json" }
                try {
                    $resp = Invoke-WebRequest @args
                    $json = $null
                    if ($resp.Content) { try { $json = $resp.Content | ConvertFrom-Json } catch {} }
                    return [pscustomobject]@{ ok = $true; status = [int]$resp.StatusCode; json = $json; raw = $resp.Content }
                } catch {
                    $webResp = $null
                    try { $webResp = $_.Exception.Response } catch {}
                    if (-not $webResp) { return [pscustomobject]@{ ok = $false; status = 0; json = $null; raw = [string]$_.Exception.Message } }
                    $statusCode = 0
                    try { $statusCode = [int]$webResp.StatusCode } catch {}
                    $rawBody = ""
                    try {
                        $stream = $webResp.GetResponseStream()
                        if ($stream) {
                            $reader = New-Object System.IO.StreamReader($stream)
                            $rawBody = $reader.ReadToEnd()
                            $reader.Dispose()
                            $stream.Dispose()
                        }
                    } catch {}
                    $jsonErr = $null
                    if ($rawBody) { try { $jsonErr = $rawBody | ConvertFrom-Json } catch {} }
                    return [pscustomobject]@{ ok = $true; status = $statusCode; json = $jsonErr; raw = $rawBody }
                }
            }

            $healthyUnsafe = $false
            for ($i = 0; $i -lt 40; $i++) {
                $health = Invoke-UnsafeAgentJson -Method "GET" -Path "health"
                if ($health.ok -and $health.status -eq 200 -and $health.json -and $health.json.ok -eq $true) {
                    if ([string]$health.json.auth_mode -eq "unsafe" -and [bool]$health.json.unsafe_no_auth) {
                        $healthyUnsafe = $true
                        break
                    }
                }
                if ($unsafeProc -and $unsafeProc.HasExited) { break }
                Start-Sleep -Milliseconds 300
            }
            $healthyUnsafe | Should Be $true

            $runUnsafe = Invoke-UnsafeAgentJson -Method "POST" -Path "run_async" -Body '{"action":"doctor"}'
            $runUnsafe.ok | Should Be $true
            $runUnsafe.status | Should Be 202
            $runUnsafe.json.ok | Should Be $true
            [string]$runUnsafe.json.code | Should Be "ok"
        } finally {
            if ($unsafeProc -and -not $unsafeProc.HasExited) {
                try { Stop-Process -Id $unsafeProc.Id -Force -ErrorAction Stop } catch {}
            }
            foreach ($p in @($tempCfgUnsafe, $stdoutUnsafe, $stderrUnsafe)) {
                if ($p -and (Test-Path $p)) {
                    Remove-Item -Force -Path $p -ErrorAction SilentlyContinue
                }
            }
        }
    }
}
