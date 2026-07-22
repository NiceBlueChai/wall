<# Verifies the Wall installer language and exact-path mpv cleanup contract. #>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$ConfigPath = Join-Path $ProjectRoot "src-tauri\tauri.conf.json"
$CleanupScript = Join-Path $ProjectRoot "src-tauri\windows\cleanup-mpv.ps1"
$HookScript = Join-Path $ProjectRoot "src-tauri\windows\installer-hooks.nsh"
$Config = Get-Content -LiteralPath $ConfigPath -Raw -Encoding UTF8 | ConvertFrom-Json
$Languages = @($Config.bundle.windows.nsis.languages)

if ($Languages.Count -ne 1 -or $Languages[0] -ne "SimpChinese") {
    throw "NSIS installer languages must contain only SimpChinese."
}
if ($Config.bundle.windows.nsis.displayLanguageSelector -ne $false) {
    throw "NSIS installer language selector must be disabled."
}
if ($Config.bundle.windows.nsis.installerHooks -ne "windows/installer-hooks.nsh") {
    throw "NSIS installer hook path is not configured."
}
foreach ($file in @($CleanupScript, $HookScript)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "Required installer lifecycle file is missing: $file"
    }
}
[void][scriptblock]::Create((Get-Content -LiteralPath $CleanupScript -Raw -Encoding UTF8))

$TempName = "wall-installer-lifecycle-" + [guid]::NewGuid()
$TempRoot = Join-Path ([IO.Path]::GetTempPath()) $TempName
$TargetDirectory = Join-Path $TempRoot "target"
$OtherDirectory = Join-Path $TempRoot "other"
$TargetMpv = Join-Path $TargetDirectory "mpv.exe"
$OtherMpv = Join-Path $OtherDirectory "mpv.exe"
$TargetProcess = $null
$OtherProcess = $null

try {
    New-Item -ItemType Directory -Force -Path $TargetDirectory, $OtherDirectory | Out-Null
    Copy-Item -LiteralPath $env:ComSpec -Destination $TargetMpv
    Copy-Item -LiteralPath $env:ComSpec -Destination $OtherMpv
    $Arguments = @("/d", "/c", "ping 127.0.0.1 -n 30 > nul")
    $StartArguments = @{
        ArgumentList = $Arguments
        PassThru = $true
        WindowStyle = "Hidden"
    }
    $TargetProcess = Start-Process -FilePath $TargetMpv @StartArguments
    $OtherProcess = Start-Process -FilePath $OtherMpv @StartArguments
    Start-Sleep -Milliseconds 250

    & $CleanupScript -TargetPath $TargetMpv
    if (-not $TargetProcess.WaitForExit(5000)) {
        throw "The target mpv process was not stopped."
    }
    $OtherProcess.Refresh()
    if ($OtherProcess.HasExited) {
        throw "An mpv process outside the target path was stopped."
    }
} finally {
    foreach ($process in @($TargetProcess, $OtherProcess)) {
        if ($null -ne $process) {
            $process.Refresh()
            if (-not $process.HasExited) {
                Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            }
            $process.Dispose()
        }
    }
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}

Write-Host "Installer lifecycle contract verified."
