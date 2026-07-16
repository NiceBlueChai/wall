<# Verifies that Wall embeds mpv below the desktop icon layer without using UI input. #>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$WallDirectory
)

$ErrorActionPreference = "Stop"

Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

/// <summary>Provides Win32 helpers for validating the native wallpaper window.</summary>
public static class WallNativeVerification
{
    /// <summary>Receives a window handle while enumerating desktop windows.</summary>
    public delegate bool EnumWindowProc(IntPtr window, IntPtr data);

    /// <summary>Stores the bounds of a native window.</summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct Rect
    {
        /// <summary>Left edge in screen coordinates.</summary>
        public int Left;
        /// <summary>Top edge in screen coordinates.</summary>
        public int Top;
        /// <summary>Right edge in screen coordinates.</summary>
        public int Right;
        /// <summary>Bottom edge in screen coordinates.</summary>
        public int Bottom;
    }

    /// <summary>Finds a top-level window by class or caption.</summary>
    [DllImport("user32.dll", EntryPoint = "FindWindowW", CharSet = CharSet.Unicode, ExactSpelling = true)]
    public static extern IntPtr FindWindow(string className, string windowName);

    /// <summary>Finds a child window by class or caption.</summary>
    [DllImport("user32.dll", EntryPoint = "FindWindowExW", CharSet = CharSet.Unicode, ExactSpelling = true)]
    public static extern IntPtr FindWindowEx(
        IntPtr parent,
        IntPtr childAfter,
        string className,
        string windowName
    );

    /// <summary>Enumerates top-level windows.</summary>
    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowProc callback, IntPtr data);

    /// <summary>Enumerates descendant windows.</summary>
    [DllImport("user32.dll")]
    public static extern bool EnumChildWindows(IntPtr parent, EnumWindowProc callback, IntPtr data);

    /// <summary>Reads the process that owns a window.</summary>
    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

    /// <summary>Reads a window style value.</summary>
    [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")]
    public static extern IntPtr GetWindowLongPtr(IntPtr window, int index);

    /// <summary>Reads the parent of a window.</summary>
    [DllImport("user32.dll")]
    public static extern IntPtr GetParent(IntPtr window);

    /// <summary>Reads a related window in z-order.</summary>
    [DllImport("user32.dll")]
    public static extern IntPtr GetWindow(IntPtr window, uint command);

    /// <summary>Reads the screen bounds of a window.</summary>
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr window, out Rect rectangle);

    /// <summary>Reads a system display metric.</summary>
    [DllImport("user32.dll")]
    public static extern int GetSystemMetrics(int index);

    /// <summary>Reads the caption of a window.</summary>
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetWindowText(IntPtr window, StringBuilder text, int maximumLength);

    /// <summary>Changes the visibility state of a window.</summary>
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr window, int command);

    /// <summary>Finds all top-level and descendant windows owned by one process.</summary>
    public static IntPtr[] FindProcessWindows(uint processId)
    {
        HashSet<IntPtr> matches = new HashSet<IntPtr>();
        EnumWindowProc inspect = delegate(IntPtr window, IntPtr data)
        {
            uint owner;
            GetWindowThreadProcessId(window, out owner);
            if (owner == processId)
            {
                matches.Add(window);
            }
            return true;
        };
        EnumWindows(delegate(IntPtr root, IntPtr data)
        {
            inspect(root, data);
            EnumChildWindows(root, inspect, data);
            return true;
        }, IntPtr.Zero);
        IntPtr[] result = new IntPtr[matches.Count];
        matches.CopyTo(result);
        return result;
    }
}
"@

$resolvedDirectory = (Resolve-Path -LiteralPath $WallDirectory).Path.TrimEnd('\')
$mpv = Get-Process mpv -ErrorAction Stop | Where-Object {
    $_.Path -and (Split-Path -Parent $_.Path).TrimEnd('\') -eq $resolvedDirectory
} | Select-Object -First 1
if (-not $mpv) {
    throw "The portable Wall instance has no mpv child process."
}

$wall = Get-Process Wall -ErrorAction Stop | Where-Object {
    $_.Path -and (Split-Path -Parent $_.Path).TrimEnd('\') -eq $resolvedDirectory
} | Select-Object -First 1
if ($wall -and $wall.MainWindowHandle -ne 0) {
    [WallNativeVerification]::ShowWindow($wall.MainWindowHandle, 0) | Out-Null
}

$progman = [WallNativeVerification]::FindWindow("Progman", $null)
$windows = [WallNativeVerification]::FindProcessWindows([uint32]$mpv.Id)
$playerWindows = @($windows | Where-Object {
    $candidateTitle = New-Object System.Text.StringBuilder 256
    [WallNativeVerification]::GetWindowText($_, $candidateTitle, $candidateTitle.Capacity) | Out-Null
    $candidateTitle.ToString().StartsWith("wall-wallpaper-")
})
if ($playerWindows.Count -ne 1) {
    throw "Expected one titled embedded mpv window, found $($playerWindows.Count)."
}

$player = $playerWindows[0]
$style = [WallNativeVerification]::GetWindowLongPtr($player, -16).ToInt64()
$extendedStyle = [WallNativeVerification]::GetWindowLongPtr($player, -20).ToInt64()
$parent = [WallNativeVerification]::GetParent($player)
if ($progman -eq [IntPtr]::Zero) {
    $progman = $parent
}
$shellView = [WallNativeVerification]::FindWindowEx($progman, [IntPtr]::Zero, "SHELLDLL_DefView", $null)
$progmanExtendedStyle = [WallNativeVerification]::GetWindowLongPtr($progman, -20).ToInt64()
$raisedDesktop = ($progmanExtendedStyle -band 0x00200000) -ne 0
$previousWindow = [WallNativeVerification]::GetWindow($player, 3)
$rectangle = New-Object WallNativeVerification+Rect
if (-not [WallNativeVerification]::GetWindowRect($player, [ref]$rectangle)) {
    throw "Unable to read the embedded mpv window rectangle."
}
$screenWidth = [WallNativeVerification]::GetSystemMetrics(0)
$screenHeight = [WallNativeVerification]::GetSystemMetrics(1)
$title = New-Object System.Text.StringBuilder 256
[WallNativeVerification]::GetWindowText($player, $title, $title.Capacity) | Out-Null

$checks = [ordered]@{
    playerFound = $player -ne [IntPtr]::Zero
    titleIsUnique = $title.ToString().StartsWith("wall-wallpaper-")
    parentIsDesktopHost = $parent -ne [IntPtr]::Zero
    childStyle = ($style -band 0x40000000) -ne 0
    popupStyleRemoved = ($style -band 0x80000000L) -eq 0
    fillsPrimaryScreen = (
        $rectangle.Left -eq 0 -and $rectangle.Top -eq 0 -and
        ($rectangle.Right - $rectangle.Left) -eq $screenWidth -and
        ($rectangle.Bottom - $rectangle.Top) -eq $screenHeight
    )
    raisedDesktopLayered = (-not $raisedDesktop) -or (($extendedStyle -band 0x00080000) -ne 0)
    raisedDesktopParent = (-not $raisedDesktop) -or ($parent -eq $progman)
    belowDesktopIcons = (-not $raisedDesktop) -or ($previousWindow -eq $shellView)
}

$result = [ordered]@{
    mpvProcessId = $mpv.Id
    playerWindow = $player.ToInt64()
    title = $title.ToString()
    parent = $parent.ToInt64()
    progman = $progman.ToInt64()
    shellView = $shellView.ToInt64()
    raisedDesktop = $raisedDesktop
    style = "0x{0:X}" -f $style
    extendedStyle = "0x{0:X}" -f $extendedStyle
    rectangle = "$($rectangle.Left),$($rectangle.Top),$($rectangle.Right),$($rectangle.Bottom)"
    screen = "${screenWidth}x${screenHeight}"
    checks = $checks
}

$result | ConvertTo-Json -Depth 4
$failedChecks = $checks.GetEnumerator() | Where-Object { -not $_.Value }
if ($failedChecks) {
    throw "Native wallpaper verification failed: $($failedChecks.Name -join ', ')"
}
