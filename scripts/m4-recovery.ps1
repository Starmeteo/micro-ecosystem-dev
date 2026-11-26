param(
    [string]$Executable,
    [switch]$IncludeAutosave
)

$ErrorActionPreference = 'Stop'
$ecoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
if (-not $Executable) { $Executable = Join-Path $ecoRoot 'target\debug\micro-ecosystem-desktop.exe' }
$ecoExe = (Resolve-Path -LiteralPath $Executable).Path
$ecoTest = Join-Path $PSScriptRoot 'm4-recovery.mjs'
$ecoRun = Join-Path $ecoRoot ('.local\m4-recovery\run-' + (Get-Date -Format 'yyyyMMdd-HHmmss-fff'))
$ecoPhases = @('seed', 'pending', 'newest-corrupt', 'write-failure', 'all-corrupt', 'future-format', 'missing-definition', 'locked')
if ($IncludeAutosave) { $ecoPhases += @('auto', 'auto-reopen') }
New-Item -ItemType Directory -Path $ecoRun | Out-Null
@{
    executable = $ecoExe
    sha256 = (Get-FileHash -LiteralPath $ecoExe -Algorithm SHA256).Hash
    windowsBuild = [System.Environment]::OSVersion.Version.ToString()
    runner = $ecoTest
} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $ecoRun 'build.json') -Encoding utf8
Write-Output "M4_RECOVERY=$ecoRun"

foreach ($ecoPhase in $ecoPhases) {
    if (Get-Process -Name 'micro-ecosystem-desktop' -ErrorAction SilentlyContinue) {
        throw 'The desktop app is already open. The existing instance was left running.'
    }
    $ecoCaseRoot = Join-Path $ecoRun $ecoPhase
    if ($ecoPhase -in @('seed', 'auto', 'auto-reopen')) { New-Item -ItemType Directory -Path $ecoCaseRoot | Out-Null }
    $ecoListener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $ecoListener.Start()
    $ecoPort = $ecoListener.LocalEndpoint.Port
    $ecoListener.Stop()
    $ecoDataRoot = if ($ecoPhase -eq 'auto-reopen') { Join-Path $ecoRun 'auto' } else { $ecoCaseRoot }
    $env:MICRO_ECOSYSTEM_DATA_DIR = Join-Path $ecoDataRoot 'data'
    $env:MICRO_ECOSYSTEM_AUTOSAVE_INTERVAL_MS = if ($ecoPhase -eq 'auto') { '1000' } else { '120000' }
    $env:MICRO_ECOSYSTEM_TRACE_FILE = Join-Path $ecoCaseRoot 'trace.jsonl'
    $env:MICRO_ECOSYSTEM_LIFECYCLE_PROBE = '0'
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-address=127.0.0.1 --remote-debugging-port=' + $ecoPort
    $env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $ecoCaseRoot 'profile'
    $ecoHeldFile = $null
    $ecoProc = $null
    try {
        if ($ecoPhase -eq 'locked') {
            $ecoSeed = Get-Content -LiteralPath (Join-Path $ecoRun 'seed.json') -Raw | ConvertFrom-Json
            if ($ecoSeed.worldId -notmatch '^pond-[0-9a-f]+$') { throw 'Unexpected isolated fixture ID.' }
            $ecoDirectory = Join-Path $env:MICRO_ECOSYSTEM_DATA_DIR ('world-' + $ecoSeed.worldId)
            $ecoCheckpoint = Join-Path $ecoDirectory 'checkpoint-00000000000000000002.json'
            $ecoHeldFile = [System.IO.File]::Open($ecoCheckpoint, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::None)
        }
        $ecoProc = Start-Process -FilePath $ecoExe -WindowStyle Hidden -PassThru `
            -RedirectStandardOutput (Join-Path $ecoCaseRoot 'stdout.log') `
            -RedirectStandardError (Join-Path $ecoCaseRoot 'stderr.log')
        node $ecoTest $ecoPhase $ecoPort $ecoRun | Tee-Object -FilePath (Join-Path $ecoCaseRoot 'checks.jsonl')
        if ($LASTEXITCODE -ne 0) { throw "Native assertions failed in $ecoPhase." }
        if ($ecoPhase -in @('seed', 'auto')) {
            # Abruptly stop only the process recorded from this isolated launch.
            if ($ecoProc.HasExited) { throw 'The test process exited before the crash simulation.' }
            $ecoProc.Kill()
            if (-not $ecoProc.WaitForExit(8000)) { throw 'The isolated seed process did not stop.' }
            if (Select-String -LiteralPath $env:MICRO_ECOSYSTEM_TRACE_FILE -Pattern 'exit-request-committed' -Quiet) {
                throw 'The test process committed an orderly exit before the crash simulation.'
            }
        } else {
            if (-not $ecoProc.WaitForExit(8000)) { throw "The $ecoPhase app did not close." }
            if ($ecoProc.ExitCode -ne 0) { throw "The $ecoPhase app exited with $($ecoProc.ExitCode)." }
        }
    } finally {
        if ($ecoHeldFile) { $ecoHeldFile.Dispose() }
        if ($ecoProc -and -not $ecoProc.HasExited) {
            $ecoProc.Kill()
            $ecoProc.WaitForExit(6000) | Out-Null
        }
    }
    if ($ecoPhase -eq 'seed') {
        node $ecoTest 'prepare' 0 $ecoRun | Tee-Object -FilePath (Join-Path $ecoRun 'fixtures.jsonl')
        if ($LASTEXITCODE -ne 0) { throw 'Failed to create the isolated fault scenarios.' }
    }
}

node $ecoTest 'seal' 0 $ecoRun | Tee-Object -FilePath (Join-Path $ecoRun 'seal.jsonl')
if ($LASTEXITCODE -ne 0) { throw 'On-disk verification failed.' }
if ($IncludeAutosave) {
    node $ecoTest 'seal-auto' 0 $ecoRun | Tee-Object -FilePath (Join-Path $ecoRun 'seal-auto.jsonl')
    if ($LASTEXITCODE -ne 0) { throw 'Automatic checkpoint verification failed.' }
}
