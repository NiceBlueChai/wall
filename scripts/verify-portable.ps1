<# Runs a no-mouse native media smoke test and restores the user's Wall data byte-for-byte. #>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$WallDirectory,

    [Alias("VideoPath")]
    [string]$MediaPath,

    [ValidateSet("video", "image")]
    [string]$MediaKind,

    [switch]$UseExistingData,

    [switch]$RecoverInterruptedRun
)

$ErrorActionPreference = "Stop"
$WallDirectory = [IO.Path]::GetFullPath($WallDirectory).TrimEnd('\')
$AppDataRoot = [IO.Path]::GetFullPath($env:APPDATA).TrimEnd('\')
$DataDirectory = [IO.Path]::GetFullPath((Join-Path $AppDataRoot "com.wall.app")).TrimEnd('\')
if (-not $DataDirectory.StartsWith("$AppDataRoot\", [StringComparison]::OrdinalIgnoreCase)) {
    throw "The resolved Wall data directory is outside APPDATA."
}
if ((Split-Path -Leaf $DataDirectory) -ne "com.wall.app") {
    throw "The resolved Wall data directory has an unexpected name."
}
$InterruptedBackups = @(Get-ChildItem -LiteralPath $AppDataRoot -Force -Directory | Where-Object {
    $_.Name -like "com.wall.app.verify-*"
})

if ($RecoverInterruptedRun) {
    if ($InterruptedBackups.Count -ne 1) {
        throw "Expected exactly one interrupted verification backup, found $($InterruptedBackups.Count)."
    }
    @(Get-Process Wall,mpv -ErrorAction SilentlyContinue | Where-Object {
        $_.Path -and (Split-Path -Parent $_.Path).TrimEnd('\') -eq $WallDirectory
    }) | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 400
    if (Get-Process Wall -ErrorAction SilentlyContinue) {
        throw "Another Wall process is still running; close it before recovering application data."
    }
    if (Test-Path -LiteralPath $DataDirectory) {
        Remove-Item -LiteralPath $DataDirectory -Recurse -Force
    }
    Move-Item -LiteralPath $InterruptedBackups[0].FullName -Destination $DataDirectory
    Write-Host "INTERRUPTED_RUN_RECOVERED: $DataDirectory"
    exit 0
}
if ($InterruptedBackups.Count -gt 0) {
    throw "An interrupted verification backup exists; run this script with -RecoverInterruptedRun first."
}
if (-not $MediaPath) {
    throw "MediaPath is required unless -RecoverInterruptedRun is used."
}
$MediaPath = (Resolve-Path -LiteralPath $MediaPath).Path
$Extension = [IO.Path]::GetExtension($MediaPath).TrimStart('.').ToLowerInvariant()
if (@("jpg", "jpeg", "png", "webp", "bmp", "gif") -contains $Extension) {
    $DetectedKind = "image"
} elseif (@("mp4", "mkv", "webm", "mov", "avi") -contains $Extension) {
    $DetectedKind = "video"
} else {
    throw "Unable to infer the media kind from .$Extension."
}
if (-not $MediaKind) {
    $MediaKind = $DetectedKind
} elseif ($MediaKind -ne $DetectedKind) {
    throw "MediaKind '$MediaKind' does not match .$Extension media."
}
$MediaId = "portable-verification-$MediaKind"
$WallExecutable = Join-Path $WallDirectory "Wall.exe"
if (-not (Test-Path -LiteralPath $WallExecutable)) {
    throw "Wall.exe is missing from $WallDirectory."
}
if (Get-Process Wall -ErrorAction SilentlyContinue) {
    throw "A Wall process is already running; close it before the automated smoke test."
}

$BackupDirectory = "$DataDirectory.verify-$([guid]::NewGuid().ToString('N'))"
$HadData = Test-Path -LiteralPath $DataDirectory

function Get-TreeHashes {
    param([Parameter(Mandatory = $true)][string]$Root)

    if (-not (Test-Path -LiteralPath $Root)) {
        return @()
    }
    return @(Get-ChildItem -LiteralPath $Root -Recurse -Force -File | Sort-Object FullName | ForEach-Object {
        $Relative = $_.FullName.Substring($Root.Length).TrimStart('\')
        "$Relative|$((Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash)"
    })
}

$OriginalHashes = Get-TreeHashes -Root $DataDirectory
if ($HadData) {
    Move-Item -LiteralPath $DataDirectory -Destination $BackupDirectory
}
$WallProcess = $null
$NativeProcess = $null
$NativeOutput = Join-Path ([IO.Path]::GetTempPath()) "wall-native-$([guid]::NewGuid().ToString('N')).out"
$NativeError = "$NativeOutput.err"
try {
    if ($UseExistingData) {
        if (-not $HadData) {
            throw "Wall has no existing application data to verify."
        }
        Copy-Item -LiteralPath $BackupDirectory -Destination $DataDirectory -Recurse -Force
    } else {
        New-Item -ItemType Directory -Path $DataDirectory -Force | Out-Null
        $Library = @(
            [ordered]@{
                id = $MediaId
                name = [IO.Path]::GetFileNameWithoutExtension($MediaPath)
                path = $MediaPath
                kind = $MediaKind
                format = $Extension.ToUpperInvariant()
                width = $null
                height = $null
                durationSeconds = $null
                thumbnailPath = $null
                missing = $false
            }
        )
        $Session = [ordered]@{
            activeId = $MediaId
            status = "playing"
            muted = $true
            volume = 0
            pauseReasons = @()
            lastError = $null
        }
        ConvertTo-Json -InputObject $Library -Depth 5 | Set-Content -LiteralPath `
            (Join-Path $DataDirectory "library.json") -Encoding utf8
        ConvertTo-Json -InputObject $Session -Depth 5 | Set-Content -LiteralPath `
            (Join-Path $DataDirectory "session.json") -Encoding utf8
    }

    $WallProcess = Start-Process -FilePath $WallExecutable -WorkingDirectory $WallDirectory `
        -WindowStyle Hidden -PassThru
    $Deadline = [DateTime]::UtcNow.AddSeconds(25)
    $ResponsiveDeadline = [DateTime]::UtcNow.AddSeconds(6)
    do {
        Start-Sleep -Milliseconds 250
        if ([DateTime]::UtcNow -ge $ResponsiveDeadline) {
            $WallProcess.Refresh()
            if (-not $WallProcess.Responding) {
                throw "Wall stopped processing Windows messages while waiting for mpv."
            }
        }
        $MpvProcess = @(Get-Process mpv -ErrorAction SilentlyContinue | Where-Object {
            $_.Path -and (Split-Path -Parent $_.Path).TrimEnd('\') -eq $WallDirectory
        }) | Select-Object -First 1
    } while (-not $MpvProcess -and [DateTime]::UtcNow -lt $Deadline)
    if (-not $MpvProcess) {
        throw "Wall did not start its bundled mpv within 25 seconds."
    }

    Start-Sleep -Seconds 6
    $WallProcess.Refresh()
    if (-not $WallProcess.Responding) {
        throw "Wall stopped processing Windows messages after startup."
    }

    $MpvProcess = @(Get-Process mpv -ErrorAction SilentlyContinue | Where-Object {
        $_.Path -and (Split-Path -Parent $_.Path).TrimEnd('\') -eq $WallDirectory
    }) | Select-Object -First 1
    if (-not $MpvProcess) {
        throw "Wall lost its bundled mpv process after startup."
    }

    $CommandLine = (Get-CimInstance Win32_Process -Filter "ProcessId = $($MpvProcess.Id)").CommandLine
    if ($CommandLine -notmatch [regex]::Escape($MediaPath)) {
        throw "mpv did not open the requested test media."
    }
    if ($MediaKind -eq "image" -and $CommandLine -notmatch "--image-display-duration=inf") {
        throw "mpv did not keep the requested image visible indefinitely."
    }
    $NativeScript = Join-Path $PSScriptRoot "verify-wallpaper-host.ps1"
    $NativeArguments = @(
        "-NoProfile",
        "-NonInteractive",
        "-File",
        "`"$NativeScript`"",
        "-WallDirectory",
        "`"$WallDirectory`""
    )
    $NativeProcess = Start-Process -FilePath (Get-Command pwsh).Source -ArgumentList $NativeArguments `
        -WindowStyle Hidden -RedirectStandardOutput $NativeOutput -RedirectStandardError $NativeError `
        -PassThru
    if (-not $NativeProcess.WaitForExit(20000)) {
        $NativeProcess.Kill($true)
        throw "Native wallpaper verification did not finish within 20 seconds."
    }
    if ($NativeProcess.ExitCode -ne 0) {
        throw "Native wallpaper verification failed: $([IO.File]::ReadAllText($NativeError))"
    }
    $NativeResult = [IO.File]::ReadAllText($NativeOutput)
    $ExecutableBytes = [IO.File]::ReadAllBytes($WallExecutable)
    $PeOffset = [BitConverter]::ToInt32($ExecutableBytes, 0x3c)
    $Subsystem = [BitConverter]::ToUInt16($ExecutableBytes, $PeOffset + 24 + 68)
    if ($Subsystem -ne 2) {
        throw "Wall.exe is not a Windows GUI executable. Subsystem=$Subsystem"
    }
    [ordered]@{
        media = $MediaPath
        mediaKind = $MediaKind
        mpvCommandLine = $CommandLine
        peSubsystem = $Subsystem
        nativeVerification = ($NativeResult | ConvertFrom-Json)
    } | ConvertTo-Json -Depth 8
} finally {
    if ($NativeProcess -and -not $NativeProcess.HasExited) {
        $NativeProcess.Kill($true)
    }
    Remove-Item -LiteralPath $NativeOutput,$NativeError -Force -ErrorAction SilentlyContinue
    @(Get-Process mpv -ErrorAction SilentlyContinue | Where-Object {
        $_.Path -and (Split-Path -Parent $_.Path).TrimEnd('\') -eq $WallDirectory
    }) | Stop-Process -Force -ErrorAction SilentlyContinue
    if ($WallProcess -and -not $WallProcess.HasExited) {
        Stop-Process -Id $WallProcess.Id -Force -ErrorAction SilentlyContinue
    }
    Start-Sleep -Milliseconds 400
    if (Test-Path -LiteralPath $DataDirectory) {
        Remove-Item -LiteralPath $DataDirectory -Recurse -Force
    }
    if ($HadData) {
        Move-Item -LiteralPath $BackupDirectory -Destination $DataDirectory
    }
}

$RestoredHashes = Get-TreeHashes -Root $DataDirectory
if (Compare-Object -ReferenceObject $OriginalHashes -DifferenceObject $RestoredHashes) {
    throw "Wall application data was not restored byte-for-byte after the smoke test."
}
Write-Host "APP_DATA_RESTORED: SHA-256 tree matches"
