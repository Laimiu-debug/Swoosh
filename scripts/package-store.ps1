param(
    [string]$StoreIdentityPath,
    [switch]$SkipBuild
)
$ErrorActionPreference = 'Stop'
$workspacePath = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $workspacePath
$outputRoot = Join-Path $workspacePath 'target\msix'
$mode = 'local-preview'
$identity = [pscustomobject]@{
    packageName = 'Swoosh.LocalPreview'
    publisher = 'CN=Swoosh Local Preview'
    publisherDisplayName = 'Swoosh Local Preview'
    version = '1.0.0.0'
}
if ($StoreIdentityPath) {
    $identity = Get-Content -LiteralPath $StoreIdentityPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $mode = 'store-identity'
}
foreach ($field in @('packageName', 'publisher', 'publisherDisplayName', 'version')) {
    if ($identity.$field -isnot [string] -or [string]::IsNullOrWhiteSpace($identity.$field)) {
        throw "Missing $field. Copy the actual identity from Partner Center; the example file cannot be submitted."
    }
}
if ($identity.packageName -notmatch '^[A-Za-z0-9][A-Za-z0-9.-]{2,49}$') { throw 'Invalid MSIX package name.' }
if ($StoreIdentityPath -and $identity.packageName -eq 'Swoosh.LocalPreview') { throw 'Local preview identity cannot be submitted to the Store.' }
if ($identity.publisher -notmatch '^CN=') { throw 'Publisher must match the X.500 publisher value from Partner Center.' }
[void][System.Security.Cryptography.X509Certificates.X500DistinguishedName]::new($identity.publisher)
if ($identity.publisherDisplayName.Length -gt 256) { throw 'Publisher display name is too long.' }
if ($identity.version -notmatch '^([1-9][0-9]*)\.([0-9]+)\.([0-9]+)\.0$') {
    throw 'Store package version must have a nonzero major version and a zero fourth component, e.g. 1.0.0.0.'
}
foreach ($part in $identity.version.Split('.')) { if ([long]$part -gt 65535) { throw 'Version component exceeds 65535.' } }
$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
$makeAppx = Get-ChildItem -LiteralPath $sdkRoot -Directory |
    Where-Object { $_.Name -match '^10\.0\.[0-9]+\.0$' } |
    Sort-Object { [version]$_.Name } -Descending |
    ForEach-Object { Join-Path $_.FullName 'x64\makeappx.exe' } |
    Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $makeAppx) { throw 'Install the Windows 10/11 SDK with MakeAppx.exe before building an MSIX.' }
if (-not $SkipBuild) {
    $cargoDirectory = Join-Path $env:USERPROFILE '.cargo\bin'
    if (Test-Path -LiteralPath $cargoDirectory) { $env:Path = $cargoDirectory + ';' + $env:Path }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Rust MSVC toolchain is required.' }
    & pnpm tauri build --no-bundle
    if ($LASTEXITCODE -ne 0) { throw 'Tauri release build failed.' }
}
$executablePath = Join-Path $workspacePath 'target\release\swoosh.exe'
if (-not (Test-Path -LiteralPath $executablePath)) { throw 'Release executable is missing. Run without -SkipBuild first.' }
$assetSource = Join-Path $workspacePath 'packaging\windows\Assets'
foreach ($asset in @('StoreLogo.png', 'Square44x44Logo.png', 'Square150x150Logo.png', 'Wide310x150Logo.png')) {
    if (-not (Test-Path -LiteralPath (Join-Path $assetSource $asset))) { throw "Missing MSIX asset: $asset" }
}
$jobRoot = Join-Path $outputRoot ([guid]::NewGuid().ToString('N'))
$stage = Join-Path $jobRoot 'package'
$verification = Join-Path $jobRoot 'unpacked'
[void](New-Item -ItemType Directory -Path $stage -Force)
Copy-Item -LiteralPath $executablePath -Destination (Join-Path $stage 'swoosh.exe')
Copy-Item -LiteralPath $assetSource -Destination (Join-Path $stage 'Assets') -Recurse
$manifest = Get-Content -LiteralPath (Join-Path $workspacePath 'packaging\windows\AppxManifest.xml') -Raw -Encoding UTF8
foreach ($pair in @(@('PackageName', $identity.packageName), @('Publisher', $identity.publisher), @('PublisherDisplayName', $identity.publisherDisplayName), @('Version', $identity.version))) {
    $manifest = $manifest.Replace('{{' + $pair[0] + '}}', [System.Security.SecurityElement]::Escape($pair[1]))
}
if ($manifest -match '\{\{') { throw 'Unresolved manifest placeholder.' }
$manifestPath = Join-Path $stage 'AppxManifest.xml'
[System.IO.File]::WriteAllText($manifestPath, $manifest, [System.Text.UTF8Encoding]::new($false))
[void][xml]$manifest
$packagePath = Join-Path $outputRoot ("Swoosh_{0}_x64_{1}.msix" -f $identity.version, $mode)
& $makeAppx pack /d $stage /p $packagePath /o
if ($LASTEXITCODE -ne 0) { throw 'MakeAppx manifest validation/packing failed.' }
& $makeAppx unpack /p $packagePath /d $verification /o
if ($LASTEXITCODE -ne 0) { throw 'MSIX unpack verification failed.' }
$sourceHash = (Get-FileHash -LiteralPath $executablePath -Algorithm SHA256).Hash
$packedHash = (Get-FileHash -LiteralPath (Join-Path $verification 'swoosh.exe') -Algorithm SHA256).Hash
if ($sourceHash -ne $packedHash) { throw 'The packaged executable differs from the release executable.' }
$report = [ordered]@{
    mode = $mode
    package = $packagePath
    packageName = $identity.packageName
    publisher = $identity.publisher
    packageVersion = $identity.version
    appVersion = (Get-Content -LiteralPath 'src-tauri\tauri.conf.json' -Raw -Encoding UTF8 | ConvertFrom-Json).version
    sha256 = (Get-FileHash -LiteralPath $packagePath -Algorithm SHA256).Hash.ToLowerInvariant()
    executableSha256 = $sourceHash.ToLowerInvariant()
    signature = 'unsigned; Microsoft signs packages accepted for Store distribution'
    validation = 'MakeAppx schema validation and unpacked executable hash passed'
    submissionReady = $false
    remaining = @('Partner Center identity', 'Packaged app installation/upgrade/uninstall and clean-machine WebView2 test', 'Windows App Certification Kit', 'Real two-PC network acceptance')
}
$reportPath = $packagePath + '.json'
$report | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $reportPath -Encoding UTF8
Write-Host "MSIX prepared: $packagePath"
Write-Host "Report: $reportPath"
if ($mode -eq 'local-preview') { Write-Warning 'Local preview identity is not a Store product identity. Do not upload this package.' }
# Keep staging files for local registration/testing. This command neither registers an app nor trusts a certificate.
