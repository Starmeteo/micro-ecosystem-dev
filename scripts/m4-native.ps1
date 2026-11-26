param(
    [Parameter(Mandatory)][ValidateSet('standby', 'performance', 'visual')][string]$Mode,
    [string]$Executable,
    [ValidateRange(5, 60)][int]$SampleSeconds = 10,
    [ValidateRange(15, 3600)][int]$SoakSeconds = 120
)

$ErrorActionPreference = 'Stop'
$ecoRoot = Split-Path -Parent $PSScriptRoot
if (-not $Executable) { $Executable = Join-Path $ecoRoot 'target/release/micro-ecosystem-desktop.exe' }
$ecoExe = (Resolve-Path -LiteralPath $Executable).Path
if (Get-Process -Name 'micro-ecosystem-desktop' -ErrorAction SilentlyContinue) {
    throw 'A micro-ecosystem instance already exists. It was left running.'
}
$ecoRun = Join-Path $ecoRoot ('.local/m4-native/' + $Mode + '-' + (Get-Date -Format 'yyyyMMdd-HHmmss-fff'))
New-Item -ItemType Directory -Path $ecoRun -Force | Out-Null
$ecoListener = [System.Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
$ecoListener.Start()
$ecoPort = $ecoListener.LocalEndpoint.Port
$ecoListener.Stop()
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-address=127.0.0.1 --remote-debugging-port=$ecoPort"
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $ecoRun 'profile'
$env:MICRO_ECOSYSTEM_DATA_DIR = Join-Path $ecoRun 'data'
$env:MICRO_ECOSYSTEM_TRACE_FILE = Join-Path $ecoRun 'native.jsonl'
Remove-Item Env:MICRO_ECOSYSTEM_AUTOSAVE_INTERVAL_MS -ErrorAction SilentlyContinue
Write-Output "M4_EVIDENCE=$ecoRun"
@{
    exe = $ecoExe
    sha256 = (Get-FileHash -LiteralPath $ecoExe -Algorithm SHA256).Hash
    logicalProcessors = [Environment]::ProcessorCount
    mode = $Mode
} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $ecoRun 'build.json') -Encoding utf8

$ecoApp = Start-Process -FilePath $ecoExe -WindowStyle Hidden -PassThru `
    -RedirectStandardOutput (Join-Path $ecoRun 'app-stdout.log') -RedirectStandardError (Join-Path $ecoRun 'app-stderr.log')
try {
    $ecoScript = Join-Path $PSScriptRoot 'm4-native.mjs'
    if ($Mode -ne 'performance') {
        & node $ecoScript $Mode $ecoPort $ecoRun $SampleSeconds $SoakSeconds
        if ($LASTEXITCODE -ne 0) { throw "Native $Mode assertions failed." }
    } else {
        . (Join-Path $PSScriptRoot 'm4-resources.ps1')
        Measure-EcoResources -App $ecoApp -Run $ecoRun -Script $ecoScript -Port $ecoPort `
            -SampleSeconds $SampleSeconds -SoakSeconds $SoakSeconds
    }
    if (-not $ecoApp.WaitForExit(12000)) { throw 'The test instance did not exit after saving.' }
    if ($ecoApp.ExitCode -ne 0) { throw "The test instance exited with $($ecoApp.ExitCode)." }
    $ecoCommit = Get-Content -LiteralPath $env:MICRO_ECOSYSTEM_TRACE_FILE -Encoding utf8 |
        ForEach-Object { ConvertFrom-Json -InputObject $_ } | Where-Object { $_.event -eq 'exit-request-committed' }
    if (-not $ecoCommit) { throw 'The isolated application did not log a committed exit.' }
    Write-Output "M4_NATIVE_PASS=$Mode"
} finally {
    # Terminate only the process object created for this run.
    if (-not $ecoApp.HasExited) { $ecoApp.Kill(); $ecoApp.WaitForExit(6000) | Out-Null }
}
