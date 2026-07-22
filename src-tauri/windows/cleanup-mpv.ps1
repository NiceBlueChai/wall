<# Stops only mpv processes whose executable path matches Wall's installed mpv. #>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$TargetPath
)

$ErrorActionPreference = "Stop"
$ResolvedTarget = [IO.Path]::GetFullPath($TargetPath)
$Processes = @(Get-CimInstance Win32_Process -Filter "Name = 'mpv.exe'" | Where-Object {
    $_.ExecutablePath -and [IO.Path]::GetFullPath($_.ExecutablePath).Equals(
        $ResolvedTarget,
        [StringComparison]::OrdinalIgnoreCase
    )
})

foreach ($process in $Processes) {
    Stop-Process -Id $process.ProcessId -Force -ErrorAction Stop
}

$Deadline = [DateTime]::UtcNow.AddSeconds(5)
foreach ($process in $Processes) {
    while (Get-Process -Id $process.ProcessId -ErrorAction SilentlyContinue) {
        if ([DateTime]::UtcNow -ge $Deadline) {
            throw "Timed out waiting for mpv process $($process.ProcessId) to stop."
        }
        Start-Sleep -Milliseconds 50
    }
}
