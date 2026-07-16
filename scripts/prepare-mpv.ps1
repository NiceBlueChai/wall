<# Downloads, verifies, and extracts the pinned Windows x64 mpv runtime. #>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$Version = "20260610-git-304426c"
$ExpectedHash = "FACAC536BAA73C7B925771AF5E39A3C9CB16B8D75B59A6E9800DE89799DFFCA7"
$Url = "https://github.com/shinchiro/mpv-winbuild-cmake/releases/download/20260610/" +
    "mpv-x86_64-$Version.7z"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$Destination = Join-Path $ProjectRoot "src-tauri\binaries"
$Archive = Join-Path $env:TEMP "wall-mpv-$Version.7z"
$Extracted = Join-Path $env:TEMP "wall-mpv-$Version"

if (-not (Get-Command 7z -ErrorAction SilentlyContinue)) {
    throw "7z was not found. Install 7-Zip and run this script again."
}

if (-not (Test-Path -LiteralPath $Archive)) {
    curl.exe -L --fail --retry 3 -o $Archive $Url
    if ($LASTEXITCODE -ne 0) {
        throw "The mpv download failed."
    }
}

$ActualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $Archive).Hash
if ($ActualHash -ne $ExpectedHash) {
    Remove-Item -LiteralPath $Archive -Force
    throw "The mpv archive hash did not match; the archive was deleted."
}

if (Test-Path -LiteralPath $Extracted) {
    Remove-Item -LiteralPath $Extracted -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $Extracted, $Destination | Out-Null
7z x -y "-o$Extracted" $Archive mpv.exe d3dcompiler_43.dll | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "The mpv archive could not be extracted."
}
Copy-Item -LiteralPath (Join-Path $Extracted "mpv.exe") -Destination $Destination -Force
Copy-Item -LiteralPath (Join-Path $Extracted "d3dcompiler_43.dll") -Destination $Destination -Force
Remove-Item -LiteralPath $Extracted -Recurse -Force
Write-Host "mpv $Version is ready in $Destination"
