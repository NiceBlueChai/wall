<# Creates redistributable video and image samples for the portable Wall package. #>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Destination
)

$ErrorActionPreference = "Stop"
$Ffmpeg = Get-Command ffmpeg -CommandType Application -ErrorAction SilentlyContinue
if (-not $Ffmpeg) {
    throw "ffmpeg is required to generate the sample wallpapers."
}

$DestinationPath = [IO.Path]::GetFullPath($Destination)
New-Item -ItemType Directory -Force -Path $DestinationPath | Out-Null

$Samples = @(
    @{ Name = "01-landscape-1280x720.mp4"; Size = "1280x720"; Hue = 0 },
    @{ Name = "02-portrait-720x1280.mp4"; Size = "720x1280"; Hue = 100 },
    @{ Name = "03-square-720x720.mp4"; Size = "720x720"; Hue = 220 }
)

foreach ($Sample in $Samples) {
    $Output = Join-Path $DestinationPath $Sample.Name
    $Source = "testsrc2=size=$($Sample.Size):rate=30:duration=5,hue=h=$($Sample.Hue)"
    & $Ffmpeg.Source -hide_banner -loglevel error -y -f lavfi -i $Source -an -c:v libx264 -preset veryfast `
        -crf 28 -pix_fmt yuv420p -movflags +faststart $Output
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to generate sample wallpaper: $($Sample.Name)"
    }
}

$ImageSource = Join-Path (Split-Path -Parent $PSScriptRoot) "samples\images"
foreach ($Image in Get-ChildItem -LiteralPath $ImageSource -Filter "*.png" -File) {
    Copy-Item -LiteralPath $Image.FullName -Destination $DestinationPath
}

Write-Host "Sample wallpapers created in $DestinationPath"
