<# Builds the Wall NSIS installer with its public release filename. #>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$ConfigPath = Join-Path $ProjectRoot "src-tauri\tauri.conf.json"
$Config = Get-Content -LiteralPath $ConfigPath -Raw -Encoding UTF8 | ConvertFrom-Json
$Version = $Config.version
$TauriCli = Join-Path $ProjectRoot "node_modules\.bin\tauri.cmd"
$Icon = Join-Path $ProjectRoot "src-tauri\icons\icon.ico"
$MpvDirectory = Join-Path $ProjectRoot "src-tauri\binaries"
$ReleaseRoot = Join-Path $ProjectRoot "release"
$InternalInstaller = Join-Path $ProjectRoot `
    "src-tauri\target\release\bundle\nsis\Wall_${Version}_x64-setup.exe"
$PublishedInstaller = Join-Path $ReleaseRoot "Wall-$Version-x64-setup.exe"

foreach ($file in @($TauriCli, $Icon, (Join-Path $MpvDirectory "mpv.exe"), `
    (Join-Path $MpvDirectory "d3dcompiler_43.dll"))) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "Required installer input is missing: $file"
    }
}

New-Item -ItemType Directory -Force -Path $ReleaseRoot | Out-Null
foreach ($artifact in @($InternalInstaller, $PublishedInstaller)) {
    if (Test-Path -LiteralPath $artifact) {
        Remove-Item -LiteralPath $artifact -Force
    }
}

Push-Location $ProjectRoot
try {
    & $TauriCli build --bundles nsis
    if ($LASTEXITCODE -ne 0) {
        throw "The Tauri NSIS build failed."
    }
} finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $InternalInstaller -PathType Leaf)) {
    throw "The Tauri NSIS build did not create $InternalInstaller."
}
Copy-Item -LiteralPath $InternalInstaller -Destination $PublishedInstaller
Write-Host "Installer package created: $PublishedInstaller"
