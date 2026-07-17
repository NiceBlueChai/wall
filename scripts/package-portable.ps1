<# Builds the portable Wall package for a standard Windows 10/11 x64 system. #>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$Version = "1.0.0"
$MpvDirectory = Join-Path $ProjectRoot "src-tauri\binaries"
$ReleaseRoot = Join-Path $ProjectRoot "release"
$Stage = Join-Path $ReleaseRoot "Wall-v$Version-windows-x64-portable"
$Archive = "$Stage.zip"

foreach ($file in @("mpv.exe", "d3dcompiler_43.dll")) {
    if (-not (Test-Path -LiteralPath (Join-Path $MpvDirectory $file))) {
        throw "$file is missing. Run scripts\prepare-mpv.ps1 first."
    }
}
if (-not (Get-Command ffmpeg -CommandType Application -ErrorAction SilentlyContinue)) {
    throw "ffmpeg is missing. Install it before building the portable package."
}

Push-Location $ProjectRoot
try {
    npm run test
    if ($LASTEXITCODE -ne 0) { throw "Frontend tests failed." }
    cargo test --manifest-path src-tauri\Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw "Rust tests failed." }
    & (Join-Path $ProjectRoot "node_modules\.bin\tauri.cmd") build --no-bundle
    if ($LASTEXITCODE -ne 0) { throw "The Tauri build failed." }
} finally {
    Pop-Location
}

if (Test-Path -LiteralPath $Stage) {
    Remove-Item -LiteralPath $Stage -Recurse -Force
}
if (Test-Path -LiteralPath $Archive) {
    Remove-Item -LiteralPath $Archive -Force
}
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item -LiteralPath (Join-Path $ProjectRoot "src-tauri\target\release\wall.exe") `
    -Destination (Join-Path $Stage "Wall.exe")
Copy-Item -LiteralPath (Join-Path $MpvDirectory "mpv.exe") -Destination $Stage
Copy-Item -LiteralPath (Join-Path $MpvDirectory "d3dcompiler_43.dll") -Destination $Stage
Copy-Item -LiteralPath (Join-Path $ProjectRoot "LICENSE") -Destination $Stage
Copy-Item -LiteralPath (Join-Path $ProjectRoot "THIRD_PARTY_NOTICES.md") -Destination $Stage
Copy-Item -LiteralPath (Join-Path $ProjectRoot "README.md") -Destination $Stage
Copy-Item -LiteralPath (Join-Path $ProjectRoot "README.zh-CN.md") -Destination $Stage
Copy-Item -LiteralPath (Join-Path $ProjectRoot "CONTRIBUTING.md") -Destination $Stage
Copy-Item -LiteralPath (Join-Path $ProjectRoot "SECURITY.md") -Destination $Stage
Copy-Item -LiteralPath (Join-Path $ProjectRoot "CHANGELOG.md") -Destination $Stage
$PublicAssets = Join-Path $Stage "public"
New-Item -ItemType Directory -Force -Path $PublicAssets | Out-Null
Copy-Item -LiteralPath (Join-Path $ProjectRoot "public\wall-app-icon.png") -Destination $PublicAssets
$DocumentationImages = Join-Path $Stage "docs\images"
New-Item -ItemType Directory -Force -Path $DocumentationImages | Out-Null
Copy-Item -LiteralPath (Join-Path $ProjectRoot "docs\images\wall-library.png") -Destination $DocumentationImages
$DesignDocs = Join-Path $Stage "docs\design"
New-Item -ItemType Directory -Force -Path $DesignDocs | Out-Null
Copy-Item -LiteralPath (Join-Path $ProjectRoot "docs\design\wall-v1-design-spec.md") -Destination $DesignDocs
$ImplementationDocs = Join-Path $Stage "docs\implementation"
New-Item -ItemType Directory -Force -Path $ImplementationDocs | Out-Null
Copy-Item -LiteralPath (Join-Path $ProjectRoot "docs\implementation\wall-v1-contract-report.md") `
    -Destination $ImplementationDocs
$SampleStage = Join-Path $Stage "Sample Wallpapers"
& (Join-Path $PSScriptRoot "create-sample-wallpapers.ps1") -Destination $SampleStage
Copy-Item -LiteralPath (Join-Path $ProjectRoot "samples\README.md") -Destination $SampleStage
Compress-Archive -LiteralPath $Stage -DestinationPath $Archive -CompressionLevel Optimal
Write-Host "Portable package created: $Archive"
