function Get-EcoProcessTree {
    param([int]$RootId)
    $processes = @(Get-CimInstance -Query "SELECT ProcessId, ParentProcessId, Name FROM Win32_Process WHERE Name='msedgewebview2.exe' OR ProcessId=$RootId")
    $ids = [Collections.Generic.HashSet[int]]::new()
    $null = $ids.Add($RootId)
    do {
        $changes = $false
        foreach ($process in $processes) {
            if ($ids.Contains([int]$process.ParentProcessId) -and $ids.Add([int]$process.ProcessId)) { $changes = $true }
        }
    } while ($changes)
    return ,$ids
}

function Get-EcoGpuSample {
    param([Collections.Generic.HashSet[int]]$Ids)
    try {
        $engines = @(Get-CimInstance -ClassName Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine -Property Name,UtilizationPercentage)
    } catch { return $null }
    if (-not $engines.Count) { return $null }
    $load = @{}
    foreach ($engine in $engines) {
        if ($engine.Name -match '^pid_(\d+)_(.+)$' -and $Ids.Contains([int]$Matches[1])) {
            $name = $Matches[2]
            $load[$name] = [double]$load[$name] + [double]$engine.UtilizationPercentage
        }
    }
    $busiest = ($load.Values | Measure-Object -Maximum).Maximum
    if ($null -eq $busiest) { return 0 }
    return [Math]::Min(100, $busiest)
}

function Measure-EcoResources {
    param([Diagnostics.Process]$App, [string]$Run, [string]$Script, [int]$Port, [int]$SampleSeconds, [int]$SoakSeconds)
    $machine = Get-CimInstance -ClassName Win32_ComputerSystem -Property TotalPhysicalMemory,NumberOfLogicalProcessors
    $system = Get-CimInstance -ClassName Win32_OperatingSystem -Property Caption,Version,BuildNumber
    $cpu = @(Get-CimInstance -ClassName Win32_Processor -Property Name | ForEach-Object { $_.Name })
    $video = @(Get-CimInstance -ClassName Win32_VideoController -Property Name,DriverVersion |
        Select-Object Name,DriverVersion)
    $logical = [int]$machine.NumberOfLogicalProcessors
    if ($logical -lt 1) { throw 'Windows reported no logical CPUs.' }
    @{
        os = $system | Select-Object Caption,Version,BuildNumber
        cpu = $cpu
        logicalProcessors = $logical
        ramMiB = [Math]::Round($machine.TotalPhysicalMemory / 1MB)
        gpu = $video
        powerPlan = (powercfg /getactivescheme | Out-String).Trim()
    } | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 -LiteralPath (Join-Path $Run 'hardware.json')

    $coordinator = Start-Process -FilePath (Get-Command node).Source -ArgumentList @(
        ('"' + $Script + '"'), 'performance', $Port, ('"' + $Run + '"'), $SampleSeconds, $SoakSeconds
    ) -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput (Join-Path $Run 'runner-stdout.log') -RedirectStandardError (Join-Path $Run 'runner-stderr.log')
    $samplePath = Join-Path $Run 'resource-samples.jsonl'
    $phasePath = Join-Path $Run 'phase.json'
    $lastWall = 0
    $lastCpu = @{}
    $phaseName = ''
    $observedVersions = [Collections.Generic.HashSet[string]]::new()
    try {
        while (-not $coordinator.HasExited) {
            $current = $null
            if (Test-Path -LiteralPath $phasePath) {
                $current = Get-Content -LiteralPath $phasePath -Raw | ConvertFrom-Json
            }
            $ids = Get-EcoProcessTree -RootId $App.Id
            $members = @(Get-Process -Id @($ids) -ErrorAction SilentlyContinue)
            $now = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
            $cpuTimes = @{}
            $increment = 0.0
            $private = 0.0
            $workingSet = 0.0
            foreach ($member in $members) {
                $id = $member.Id
                $cpuTimes[$id] = [double]$member.CPU
                if ($lastCpu.ContainsKey($id)) { $increment += [Math]::Max(0.0, $cpuTimes[$id] - $lastCpu[$id]) }
                $private += $member.PrivateMemorySize64
                $workingSet += $member.WorkingSet64
                if ($member.ProcessName -eq 'msedgewebview2' -and $member.Path) {
                    $null = $observedVersions.Add([Diagnostics.FileVersionInfo]::GetVersionInfo($member.Path).FileVersion)
                }
            }
            $cpuPercent = if ($lastWall -gt 0 -and $now -gt $lastWall) { 100 * $increment / (($now - $lastWall) / 1000 * $logical) } else { $null }
            $lastWall = $now
            $lastCpu = $cpuTimes
            if ($current) {
                if ($phaseName -ne $current.name) {
                    $phaseName = $current.name
                    Write-Output "M4_PHASE=$phaseName"
                }
                @{
                    phase = $phaseName
                    wallMillis = $now
                    webviewProcesses = $members.Count - 1
                    cpuMachinePercent = $cpuPercent
                    gpuBusiestEnginePercent = Get-EcoGpuSample -Ids $ids
                    privateMiB = $private / 1MB
                    workingSetMiB = $workingSet / 1MB
                } | ConvertTo-Json -Compress | Add-Content -LiteralPath $samplePath -Encoding utf8
            }
            Start-Sleep -Milliseconds 800
        }
        if ($coordinator.ExitCode -ne 0) {
            Get-Content -LiteralPath (Join-Path $Run 'runner-stderr.log') -Tail 40
            throw 'The native performance scenario failed.'
        }
        $observedVersions | Set-Content -Encoding utf8 -LiteralPath (Join-Path $Run 'webview-versions.txt')
        & node $Script 'report' $Port $Run
        if ($LASTEXITCODE -ne 0) { throw 'Could not summarize resource samples.' }
    } finally {
        if (-not $coordinator.HasExited) { $coordinator.Kill(); $coordinator.WaitForExit(6000) | Out-Null }
    }
}
