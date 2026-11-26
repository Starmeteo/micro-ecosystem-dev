param(
    [string]$Installer,
    [string]$ReleaseExe,
    [string]$ExpectedVersion,
    [ValidateSet('Legacy', 'R5', 'Upgrade')]
    [string]$Scenario,
    [string]$PriorInstaller,
    [string]$PriorReleaseExe
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$config = Get-Content -LiteralPath (Join-Path $root 'apps\desktop\src-tauri\tauri.conf.json') -Raw -Encoding utf8 | ConvertFrom-Json
$product = $config.productName
$manufacturer = $config.identifier.Split('.')[1]
if (-not $ExpectedVersion) { $ExpectedVersion = $config.version }
if (-not $Scenario) { $Scenario = if ($ExpectedVersion -eq '0.1.0') { 'Legacy' } else { 'R5' } }
if (-not $ReleaseExe) { $ReleaseExe = Join-Path $root 'target\release\micro-ecosystem-desktop.exe' }
if (-not $Installer) {
    $Installer = Join-Path $root "target\release\bundle\nsis\${product}_${ExpectedVersion}_x64-setup.exe"
}
$Installer = (Resolve-Path -LiteralPath $Installer).Path
$ReleaseExe = (Resolve-Path -LiteralPath $ReleaseExe).Path
if ($Scenario -eq 'Upgrade') {
    if (-not $PriorInstaller -or -not $PriorReleaseExe) { throw 'Upgrade requires explicit archived prior installer and executable paths.' }
    $PriorInstaller = (Resolve-Path -LiteralPath $PriorInstaller).Path
    $PriorReleaseExe = (Resolve-Path -LiteralPath $PriorReleaseExe).Path
    if ($PriorInstaller -eq $Installer -or $PriorReleaseExe -eq $ReleaseExe) { throw 'Prior and current artifacts must be distinct.' }
    $priorVersion = (Get-Item -LiteralPath $PriorReleaseExe).VersionInfo.ProductVersion
    if ($priorVersion -eq '0.1.0' -and [version]$ExpectedVersion -gt [version]$priorVersion) {
        $priorProbe = 'upgrade-seed'
    } elseif ($priorVersion -eq '0.2.1' -and $ExpectedVersion -eq '0.2.2') {
        $priorProbe = 'upgrade-seed-r5'
    } elseif ($priorVersion -eq '0.2.2' -and $ExpectedVersion -eq '0.2.3') {
        $priorProbe = 'upgrade-seed-r5-terrain'
    } elseif ($priorVersion -eq '0.2.3' -and $ExpectedVersion -eq '0.2.4') {
        $priorProbe = 'upgrade-seed-r5-living'
    } elseif ($priorVersion -eq '0.2.4' -and $ExpectedVersion -eq '0.2.5') {
        $priorProbe = 'upgrade-seed-r5-gravel'
    } elseif ($priorVersion -eq '0.2.5' -and $ExpectedVersion -eq '0.2.6') {
        $priorProbe = 'upgrade-seed-r5-sand'
    } elseif ($priorVersion -eq '0.2.6' -and $ExpectedVersion -eq '0.2.7') {
        $priorProbe = 'upgrade-seed-r5-feedback'
    } elseif ($priorVersion -eq '0.2.7' -and $ExpectedVersion -eq '0.2.8') {
        $priorProbe = 'upgrade-seed-r5-topology'
    } elseif ($priorVersion -eq '0.2.8' -and $ExpectedVersion -eq '0.2.9') {
        $priorProbe = 'upgrade-seed-r5-sources'
    } elseif ($priorVersion -eq '0.2.9' -and $ExpectedVersion -eq '0.2.10') {
        $priorProbe = 'upgrade-seed-r5-stone-plant'
    } elseif ($priorVersion -eq '0.2.10' -and $ExpectedVersion -eq '0.2.11') {
        $priorProbe = 'upgrade-seed-r5-ribbon'
    } elseif ($priorVersion -eq '0.2.11' -and $ExpectedVersion -eq '0.2.12') {
        $priorProbe = 'upgrade-seed-r5-floating'
    } elseif ($priorVersion -eq '0.2.12' -and $ExpectedVersion -eq '0.2.13') {
        $priorProbe = 'upgrade-seed-r5-stone-biofilm'
    } elseif ($priorVersion -eq '0.2.13' -and $ExpectedVersion -eq '0.2.14') {
        $priorProbe = 'upgrade-seed-live-topology'
    } elseif ($priorVersion -eq '0.2.14' -and $ExpectedVersion -eq '0.2.15') {
        $priorProbe = 'upgrade-seed-static-sections'
    } elseif ($priorVersion -eq '0.2.15' -and $ExpectedVersion -eq '0.2.16') {
        $priorProbe = 'upgrade-seed-leaf-sections'
    } elseif ($priorVersion -eq '0.2.16' -and $ExpectedVersion -eq '0.2.17') {
        $priorProbe = 'upgrade-seed-plant-sections'
    } elseif ($priorVersion -eq '0.2.17' -and $ExpectedVersion -eq '0.2.18') {
        $priorProbe = 'upgrade-seed-living-sections'
    } elseif ($priorVersion -eq '0.2.18' -and $ExpectedVersion -eq '0.2.19') {
        $priorProbe = 'upgrade-seed-structures-litter'
    } elseif ($priorVersion -eq '0.2.19' -and $ExpectedVersion -eq '0.2.20') {
        $priorProbe = 'upgrade-seed-botany'
    } else {
        throw 'Upgrade supports archived 0.1.0 to a newer release, and consecutive R5 releases between 0.2.1 and 0.2.20.'
    }
}
if ((Get-Item -LiteralPath $ReleaseExe).VersionInfo.ProductVersion -ne $ExpectedVersion) {
    throw 'The release executable ProductVersion does not match ExpectedVersion.'
}
$installKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$product"
$vendorParent = "HKCU:\Software\$manufacturer"
$vendorKey = Join-Path $vendorParent $product
$identityKey = "HKCU:\Software\Classes\AppUserModelId\$($config.identifier)"
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$shortcuts = @('DesktopDirectory', 'Programs') | ForEach-Object {
    Join-Path ([Environment]::GetFolderPath($_)) "$product.lnk"
}

if (Get-Process -Name 'micro-ecosystem-desktop' -ErrorAction SilentlyContinue) {
    throw 'A desktop instance already exists. It was left running.'
}
foreach ($path in @($installKey, $vendorKey, $identityKey) + $shortcuts) {
    if (Test-Path -LiteralPath $path) { throw "An existing product entry was preserved: $path" }
}
if (Get-ItemProperty -LiteralPath $runKey -Name $product -ErrorAction SilentlyContinue) {
    throw 'An existing product startup entry was preserved.'
}
$machineUninstallRoots = @(
    'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
if (Get-ItemProperty -Path $machineUninstallRoots -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq $product }) {
    throw 'An existing per-machine product entry was preserved.'
}
$vendorParentExisted = Test-Path -LiteralPath $vendorParent
$evidenceBase = if ($Scenario -eq 'Legacy') { 'm4-installer-evidence' } else { 'r5-installer-evidence' }
$evidence = Join-Path $root ('.local\' + $evidenceBase + '\' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $evidence | Out-Null
$evidence = (Resolve-Path -LiteralPath $evidence).Path
$destination = Join-Path $evidence 'installed'
$installedExe = Join-Path $destination 'micro-ecosystem-desktop.exe'
$uninstaller = Join-Path $destination 'uninstall.exe'
$data = Join-Path $evidence 'data'
function Get-NsisPayloadHash([string]$Executable) {
    $bytes = [IO.File]::ReadAllBytes($Executable)
    $marker = '__TAURI_BUNDLE_TYPE_VAR_UNK'
    $offset = [Text.Encoding]::Latin1.GetString($bytes).IndexOf($marker, [StringComparison]::Ordinal)
    if ($offset -lt 0) { throw 'The unbundled executable has no Tauri bundle-type marker.' }
    # The NSIS bundler changes this marker in the packaged copy only.
    $nsisMarker = [Text.Encoding]::ASCII.GetBytes('__TAURI_BUNDLE_TYPE_VAR_NSS')
    [Array]::Copy($nsisMarker, 0, $bytes, $offset, $nsisMarker.Length)
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes))
}

function Get-DataManifest {
    $files = @{}
    if (Test-Path -LiteralPath $data -PathType Container) {
        Get-ChildItem -LiteralPath $data -Recurse -File | ForEach-Object {
            $files[[IO.Path]::GetRelativePath($data, $_.FullName)] = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
        }
    }
    $files
}

function Assert-RetainedFiles($Manifest, [switch]$Exact) {
    $actual = Get-DataManifest
    if ($Exact -and $actual.Count -ne $Manifest.Count) { throw 'The isolated data file count changed unexpectedly.' }
    foreach ($name in $Manifest.Keys) {
        if ($Manifest[$name] -ne $actual[$name]) { throw "The isolated data file was changed or removed: $name" }
    }
}

function Get-OwnedRegistration {
    $entry = Get-ItemProperty -LiteralPath $installKey -ErrorAction SilentlyContinue
    if ($null -ne $entry -and ([string]$entry.InstallLocation).Trim('"') -eq $destination) { $entry }
}

$hash = (Get-FileHash -LiteralPath $Installer -Algorithm SHA256).Hash
$expectedPayloadSha256 = Get-NsisPayloadHash $ReleaseExe
$registered = $false
$app = $null
$beforeUninstall = $null
$test = [ordered]@{
    evidence = $evidence
    scenario = $Scenario
    version = $ExpectedVersion
    installerSha256 = $hash
    installerBytes = (Get-Item -LiteralPath $Installer).Length
    signature = [string](Get-AuthenticodeSignature -LiteralPath $Installer).Status
    windowsBuild = [Environment]::OSVersion.Version.ToString()
    installMode = $config.bundle.windows.nsis.installMode
    noShortcuts = $false
    registrationVerified = $false
    expectedPayloadSha256 = $expectedPayloadSha256
    releaseExeSha256 = (Get-FileHash -LiteralPath $ReleaseExe -Algorithm SHA256).Hash
    payloadHashMatches = $false
    installedAppVerified = $false
    uninstalled = $false
}
Write-Output "INSTALL_EVIDENCE=$evidence"

function Install-IntoTestDirectory([string]$Artifact, [string]$Version, [string]$ExpectedHash) {
    if (Get-Process -Name 'micro-ecosystem-desktop' -ErrorAction SilentlyContinue) {
        throw 'A desktop instance appeared before setup. It was left running.'
    }
    if (Test-Path -LiteralPath $installKey) {
        if (-not (Get-OwnedRegistration)) { throw 'Setup refused a product installation outside the test directory.' }
    }
    $setup = Start-Process -FilePath $Artifact -ArgumentList "/S /NS /D=$destination" -WindowStyle Hidden -PassThru -Wait
    $entry = Get-OwnedRegistration
    $script:registered = $null -ne $entry
    if ($setup.ExitCode -ne 0 -or -not $registered -or $entry.DisplayVersion -ne $Version) {
        throw 'The installer did not commit the expected current-user registration.'
    }
    if ($shortcuts | Where-Object { Test-Path -LiteralPath $_ }) { throw 'Expected the isolated install to omit shortcuts.' }
    $actualHash = (Get-FileHash -LiteralPath $installedExe -Algorithm SHA256).Hash
    if ($actualHash -ne $ExpectedHash) { throw 'The installed executable differs from the packaged release build.' }
    $actualVersion = (Get-Item -LiteralPath $installedExe).VersionInfo.ProductVersion
    if ($actualVersion -ne $Version) { throw 'The installed executable ProductVersion did not match registration.' }
    [ordered]@{ exitCode = $setup.ExitCode; version = $actualVersion; registeredVersion = $entry.DisplayVersion; payloadSha256 = $actualHash }
}

function Invoke-InstalledProbe([string]$ProbeMode, [string]$Prefix) {
    if (Get-Process -Name 'micro-ecosystem-desktop' -ErrorAction SilentlyContinue) {
        throw 'A desktop instance appeared before the installed-app check. It was left running.'
    }
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $port = $listener.LocalEndpoint.Port
    $listener.Stop()
    $env:MICRO_ECOSYSTEM_DATA_DIR = $data
    $env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $evidence ($Prefix + '-webview')
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-address=127.0.0.1 --remote-debugging-port=$port"
    $env:MICRO_ECOSYSTEM_TRACE_FILE = Join-Path $evidence ($Prefix + '-app.jsonl')
    $script:app = Start-Process -FilePath $installedExe -WindowStyle Hidden -PassThru
    node (Join-Path $PSScriptRoot 'installer-smoke.mjs') $port $evidence $ProbeMode $app.Id $env:MICRO_ECOSYSTEM_TRACE_FILE |
        Tee-Object -FilePath (Join-Path $evidence ($Prefix + '-webview-check.jsonl')) | Write-Host
    if ($LASTEXITCODE -ne 0 -or -not $app.WaitForExit(10000) -or $app.ExitCode -ne 0) {
        throw 'The installed application did not complete its save-and-exit probe.'
    }
    $trace = Get-Content -LiteralPath $env:MICRO_ECOSYSTEM_TRACE_FILE -Encoding utf8 | ForEach-Object { $_ | ConvertFrom-Json }
    $checkpoint = @(Get-ChildItem -Path (Join-Path $data 'world-*\checkpoint-*.json') -File)
    if (-not ($trace | Where-Object { $_.event -eq 'exit-request-committed' -and $_.processId -eq $app.Id }) -or -not $checkpoint.Count) {
        throw 'No durable exit checkpoint from the installed application was found.'
    }
    [ordered]@{ processId = $app.Id; exitCode = $app.ExitCode; checkpointCount = $checkpoint.Count }
}

$environmentPaths = @('MICRO_ECOSYSTEM_DATA_DIR', 'WEBVIEW2_USER_DATA_FOLDER', 'WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS', 'MICRO_ECOSYSTEM_TRACE_FILE')
$oldEnvironment = @{}
foreach ($name in $environmentPaths) { $oldEnvironment[$name] = [Environment]::GetEnvironmentVariable($name) }
try {
    if ($Scenario -eq 'Upgrade') {
        $test.priorVersion = $priorVersion
        $test.priorInstallerSha256 = (Get-FileHash -LiteralPath $PriorInstaller -Algorithm SHA256).Hash
        $test.priorReleaseExeSha256 = (Get-FileHash -LiteralPath $PriorReleaseExe -Algorithm SHA256).Hash
        $test.priorInstall = Install-IntoTestDirectory $PriorInstaller $priorVersion (Get-NsisPayloadHash $PriorReleaseExe)
        $test.priorRun = Invoke-InstalledProbe $priorProbe 'prior'
        $priorFiles = Get-DataManifest
        $priorFiles | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $evidence 'prior-data-sha256.json') -Encoding utf8NoBOM
    }

    $test.currentInstall = Install-IntoTestDirectory $Installer $ExpectedVersion $expectedPayloadSha256
    $test.installExitCode = $test.currentInstall.exitCode
    $test.registrationVerified = $true
    $test.noShortcuts = $true
    $test.installedExeSha256 = $test.currentInstall.payloadSha256
    $test.payloadHashMatches = $true
    if ($Scenario -eq 'Upgrade') {
        Assert-RetainedFiles $priorFiles -Exact
        $test.upgradeKeptDataByteIdentical = $true
    }

    $test.currentRun = Invoke-InstalledProbe ($Scenario.ToLowerInvariant()) 'installed'
    $test.appExitCode = $test.currentRun.exitCode
    $test.checkpointCount = $test.currentRun.checkpointCount
    $test.installedAppVerified = $true
    if ($Scenario -eq 'R5') {
        $test.reopen = Invoke-InstalledProbe 'r5-reopen' 'reopened'
        if ($test.reopen.processId -eq $test.currentRun.processId) { throw 'Reopen must use a new installed-app process.' }
    }
    $beforeUninstall = Get-DataManifest
    $beforeUninstall | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $evidence 'before-uninstall-data-sha256.json') -Encoding utf8NoBOM
} finally {
    if ($app -and -not $app.HasExited) {
        $app.Kill()
        $app.WaitForExit(5000) | Out-Null
    }
    foreach ($name in $environmentPaths) { [Environment]::SetEnvironmentVariable($name, $oldEnvironment[$name]) }
    $otherInstance = Get-Process -Name 'micro-ecosystem-desktop' -ErrorAction SilentlyContinue
    if ($otherInstance) { $test.uninstallSkipped = 'A product instance appeared during the test and was left running.' }
    if (-not $otherInstance -and $registered -and (Get-OwnedRegistration) -and (Test-Path -LiteralPath $uninstaller -PathType Leaf)) {
        $actual = (Resolve-Path -LiteralPath $destination).Path
        if ($actual -ne $destination -or -not $actual.StartsWith($evidence + '\', [StringComparison]::OrdinalIgnoreCase)) {
            throw 'The uninstaller path is outside the isolated installation directory.'
        }
        $remove = Start-Process -FilePath $uninstaller -ArgumentList '/S' -WindowStyle Hidden -PassThru -Wait
        $test.uninstallExitCode = $remove.ExitCode
        $test.uninstalled = $remove.ExitCode -eq 0 -and -not (Test-Path -LiteralPath $installedExe) -and -not (Test-Path -LiteralPath $installKey)
        # NSIS keeps the suggested install location when data removal is unchecked.
        if ($test.uninstalled -and (Test-Path -LiteralPath $vendorKey)) {
            $key = Get-Item -LiteralPath $vendorKey
            if ($key.GetValue('') -eq $destination -and $key.SubKeyCount -eq 0) {
                Remove-Item -LiteralPath $vendorKey
            }
        }
        if ($test.uninstalled -and -not $vendorParentExisted -and (Test-Path -LiteralPath $vendorParent)) {
            $key = Get-Item -LiteralPath $vendorParent
            if ($key.SubKeyCount -eq 0 -and $key.ValueCount -eq 0) { Remove-Item -LiteralPath $vendorParent }
        }
    }
    $test.installDirectoryRemoved = -not (Test-Path -LiteralPath $destination)
    $test.vendorMetadataRemoved = -not (Test-Path -LiteralPath $vendorKey)
    $test.sampleDataPreserved = Test-Path -LiteralPath $data
    if ($beforeUninstall) {
        try {
            Assert-RetainedFiles $beforeUninstall -Exact
            $test.uninstallKeptDataByteIdentical = $true
        } catch {
            $test.uninstallKeptDataByteIdentical = $false
            $test.uninstallDataError = $_.Exception.Message
        }
    }
    $test | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $evidence 'installation.json') -Encoding utf8NoBOM
}
if (-not $test.uninstalled -or -not $test.vendorMetadataRemoved -or -not $test.sampleDataPreserved -or -not $test.uninstallKeptDataByteIdentical) {
    throw 'The isolated uninstall did not satisfy the cleanup and data-preservation checks.'
}
$test | ConvertTo-Json -Compress
