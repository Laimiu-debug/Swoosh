param([switch]$Package)
$ErrorActionPreference = 'Stop'
$workspacePath = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $workspacePath
$cargoDirectory = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path -LiteralPath $cargoDirectory) {
    $env:Path = $cargoDirectory + ';' + $env:Path
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw '请先安装 Rust MSVC 工具链：https://v2.tauri.app/start/prerequisites/'
}
if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {
    throw '请安装 Node.js 和 pnpm，然后重新打开终端。'
}
if (-not (Test-Path -LiteralPath (Join-Path $workspacePath 'node_modules'))) {
    pnpm install --frozen-lockfile
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
if ($Package) { pnpm package } else { pnpm desktop }
exit $LASTEXITCODE
