param(
    [Parameter(Mandatory)]
    [ValidateSet('check', 'headless', 'forest-replay', 'pond-replay', 'test', 'dev', 'r5-preview', 'r5-dev', 'r5-terrain', 'build', 'package')]
    [string]$Mode
)

$ErrorActionPreference = 'Stop'
$ecoRoot = Split-Path -Parent $PSScriptRoot
$ecoLocal = Join-Path $ecoRoot '.local'
$ecoRustHome = Join-Path $ecoLocal 'rustup'
$ecoRustBin = Join-Path $ecoRustHome 'toolchains\1.100.0-x86_64-pc-windows-msvc\bin'
if (Test-Path -LiteralPath (Join-Path $ecoRustBin 'cargo.exe')) {
    $env:RUSTUP_HOME = $ecoRustHome
    $env:CARGO_HOME = Join-Path $ecoLocal 'cargo'
    $env:Path = "$ecoRustBin;$env:CARGO_HOME\bin;$env:Path"
}
$env:npm_config_cache = Join-Path $ecoLocal 'npm-cache'

$ecoDesktop = Join-Path $ecoRoot 'apps\desktop'
if ($Mode -ne 'check') {
    $ecoVsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path -LiteralPath $ecoVsWhere) {
        $ecoVs = & $ecoVsWhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($ecoVs) {
            & (Join-Path $ecoVs 'Common7\Tools\Launch-VsDevShell.ps1') -Arch amd64 -HostArch amd64 -SkipAutomaticLocation
        }
    }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'Rust cargo is missing. Install the pinned Rust MSVC toolchain described in README.md.'
    }
}

Push-Location -LiteralPath $ecoRoot
try {
    switch ($Mode) {
        'headless' { cargo run --locked -p world-runtime --bin m0-headless }
        'forest-replay' { cargo run --locked -p world-runtime --bin m1-forest-replay }
        'pond-replay' { cargo run --locked -p world-runtime --bin m2-pond-replay }
        'test' { cargo test --locked --workspace }
        'check' { Set-Location -LiteralPath $ecoDesktop; npm run build:web }
        'dev' { Set-Location -LiteralPath $ecoDesktop; npm run desktop -- dev }
        { $_ -in @('r5-preview', 'r5-dev', 'r5-terrain') } {
            $ecoPreview = Join-Path $ecoLocal $Mode
            if ($Mode -eq 'r5-preview') { $env:VITE_R5_SAMPLE = '1' }
            else { Remove-Item Env:VITE_R5_SAMPLE -ErrorAction SilentlyContinue }
            $env:MICRO_ECOSYSTEM_DATA_DIR = Join-Path $ecoPreview 'data'
            $env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $ecoPreview 'webview'
            $ecoWindowConfig = 'src-tauri/tauri.r5-preview.conf.json'
            if ($Mode -eq 'r5-terrain') {
                $env:CARGO_TARGET_DIR = Join-Path $ecoPreview 'target'
                if (-not $env:CARGO_BUILD_BUILD_DIR) { $env:CARGO_BUILD_BUILD_DIR = Join-Path $ecoRoot 'target' }
                $ecoWindowConfig = 'src-tauri/tauri.r5-terrain.conf.json'
            }
            Set-Location -LiteralPath $ecoDesktop
            npm run desktop -- dev --config $ecoWindowConfig --no-watch
        }
        'build' { Set-Location -LiteralPath $ecoDesktop; npm run desktop -- build --debug --no-bundle }
        'package' { Set-Location -LiteralPath $ecoDesktop; npm run desktop -- build --ci --bundles nsis -- --locked }
    }
    $ecoExitCode = $LASTEXITCODE
} finally {
    Pop-Location
}
exit $ecoExitCode
