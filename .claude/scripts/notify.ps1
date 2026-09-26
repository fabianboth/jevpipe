param(
    [string]$Text = "Claude Code"
)

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public class WindowHelper {
    [DllImport("user32.dll")]
    public static extern bool FlashWindowEx(ref FLASHWINFO pwfi);
    public delegate bool EnumWindowsProc(IntPtr hwnd, IntPtr lParam);
    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);
    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetWindowText(IntPtr hwnd, StringBuilder lpString, int nMaxCount);

    [StructLayout(LayoutKind.Sequential)]
    public struct FLASHWINFO {
        public uint cbSize; public IntPtr hwnd; public uint dwFlags;
        public uint uCount; public uint dwTimeout;
    }

    public static List<IntPtr> GetVisibleWindows(uint pid) {
        var result = new List<IntPtr>();
        EnumWindows((hwnd, _) => {
            uint p; GetWindowThreadProcessId(hwnd, out p);
            if (p == pid && IsWindowVisible(hwnd)) result.Add(hwnd);
            return true;
        }, IntPtr.Zero);
        return result;
    }

    public static string GetTitle(IntPtr hwnd) {
        var sb = new StringBuilder(512);
        GetWindowText(hwnd, sb, sb.Capacity);
        return sb.ToString();
    }
}
'@ -ErrorAction SilentlyContinue

function Flash-Window([IntPtr]$hwnd) {
    $fi = New-Object WindowHelper+FLASHWINFO
    $fi.cbSize   = [System.Runtime.InteropServices.Marshal]::SizeOf($fi)
    $fi.hwnd     = $hwnd
    $fi.dwFlags  = 15   # FLASHW_ALL | FLASHW_TIMERNOFG
    $fi.uCount   = 0
    $fi.dwTimeout = 0
    [WindowHelper]::FlashWindowEx([ref]$fi) | Out-Null
}

# Find the IDE process: prefer VSCODE_PID, otherwise walk up the process tree.
$targetPid = $null
if ($env:VSCODE_PID) {
    $targetPid = [uint32]$env:VSCODE_PID
} else {
    $id = $PID
    while ($id) {
        $wmi = Get-CimInstance Win32_Process -Filter "ProcessId=$id" -ErrorAction SilentlyContinue
        if (-not $wmi -or -not $wmi.ParentProcessId) { break }
        $parent = Get-Process -Id $wmi.ParentProcessId -ErrorAction SilentlyContinue
        if ($parent -and $parent.MainWindowHandle -ne 0) {
            $targetPid = [uint32]$parent.Id; break
        }
        $id = $wmi.ParentProcessId
    }
}

# Match the workspace folder name against window titles to flash the right one.
if ($targetPid) {
    $workspace = Split-Path -Leaf (Get-Location)
    $handles = [WindowHelper]::GetVisibleWindows($targetPid)
    $match = $handles | Where-Object { [WindowHelper]::GetTitle($_) -like "*$workspace*" }
    if ($match) { foreach ($h in $match) { Flash-Window $h } }
}

# Toast notification (works regardless of which window is active)
Import-Module BurntToast -ErrorAction SilentlyContinue
if (Get-Command New-BurntToastNotification -ErrorAction SilentlyContinue) {
    New-BurntToastNotification -Text $Text
}
