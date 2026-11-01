param(
    [string]$Executable = 'artifacts/release/MouseMoverApp.exe',
    [string]$Screenshot
)
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class NativeUiCheck {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd, IntPtr dc, uint flags);
    [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr hwnd, int id);
    [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hwnd, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll", EntryPoint="SendMessageW", CharSet=CharSet.Unicode)] public static extern IntPtr ReadText(IntPtr hwnd, uint msg, IntPtr size, StringBuilder text);
    [DllImport("user32.dll", EntryPoint="SendMessageW", CharSet=CharSet.Unicode)] public static extern IntPtr WriteText(IntPtr hwnd, uint msg, IntPtr w, string text);
    [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr hwnd, int index);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
}
'@
function Require([bool]$Condition, [string]$Message) { if (!$Condition) { throw $Message } }
function Text([IntPtr]$Handle) {
    $buffer = [Text.StringBuilder]::new(256)
    $null = [NativeUiCheck]::ReadText($Handle,0xD,$buffer.Capacity,$buffer)
    $buffer.ToString()
}
function Click([IntPtr]$Handle) { $null = [NativeUiCheck]::SendMessage($Handle,0xF5,0,0) }
function F10 {
    [NativeUiCheck]::keybd_event(0x79,0,0,[UIntPtr]::Zero)
    [NativeUiCheck]::keybd_event(0x79,0,2,[UIntPtr]::Zero)
    Start-Sleep -Milliseconds 150
}
$process = Start-Process -FilePath (Resolve-Path -LiteralPath $Executable).Path -PassThru
$failed = $true
try {
    Require ($process.WaitForInputIdle(10000)) 'App did not initialize'
    $process.Refresh()
    $window = $process.MainWindowHandle
    Require ($window -ne 0) 'No main window'
    Require ((Text $window) -eq 'AZu Sanpo') 'Window title differs'
    Require (([NativeUiCheck]::GetWindowLong($window,-20) -band 8) -ne 0) 'Window is not topmost'
    Require (([NativeUiCheck]::GetWindowLong($window,-16) -band 0x70000) -eq 0) 'Window allows resize/minimize/maximize'
    $button = [NativeUiCheck]::GetDlgItem($window,100)
    $check = [NativeUiCheck]::GetDlgItem($window,103)
    $minutes = [NativeUiCheck]::GetDlgItem($window,104)
    $seconds = [NativeUiCheck]::GetDlgItem($window,105)
    $hours = [NativeUiCheck]::GetDlgItem($window,111)
    $label = [NativeUiCheck]::GetDlgItem($window,110)
    Require ((Text $button) -eq 'Start (F10)') 'Startup button differs'
    Require ((Text $hours) -eq '3' -and (Text $minutes) -eq '0' -and (Text $seconds) -eq '0') 'Default duration differs'
    Require ((Text $label) -eq 'Stop countdown: Off') 'Countdown not initially off'
    if ($Screenshot) {
        Add-Type -AssemblyName System.Drawing
        $rect = [NativeUiCheck+Rect]::new()
        Require ([NativeUiCheck]::GetWindowRect($window,[ref]$rect)) 'Could not read render bounds'
        $bitmap = [Drawing.Bitmap]::new($rect.Right-$rect.Left,$rect.Bottom-$rect.Top)
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        $dc = $graphics.GetHdc()
        try { Require ([NativeUiCheck]::PrintWindow($window,$dc,2)) 'Native window render failed' }
        finally { $graphics.ReleaseHdc($dc); $graphics.Dispose() }
        try {
            $imagePath = [IO.Path]::GetFullPath($Screenshot)
            New-Item -ItemType Directory -Force (Split-Path $imagePath -Parent) | Out-Null
            $bitmap.Save($imagePath,[Drawing.Imaging.ImageFormat]::Png)
        } finally { $bitmap.Dispose() }
    }
    $minuteSpinner = [NativeUiCheck]::GetDlgItem($window,106)
    $secondSpinner = [NativeUiCheck]::GetDlgItem($window,107)
    $hourSpinner = [NativeUiCheck]::GetDlgItem($window,112)
    Require ([NativeUiCheck]::SendMessage($hourSpinner,0x46A,0,0) -eq $hours) 'Hour spinner has no edit buddy'
    Require ([NativeUiCheck]::SendMessage($minuteSpinner,0x46A,0,0) -eq $minutes) 'Minute spinner has no edit buddy'
    Require ([NativeUiCheck]::SendMessage($secondSpinner,0x46A,0,0) -eq $seconds) 'Second spinner has no edit buddy'
    Require (([NativeUiCheck]::SendMessage($hourSpinner,0x466,0,0).ToInt64() -band 0xffff) -eq 999) 'Hour range differs'
    Require (([NativeUiCheck]::SendMessage($minuteSpinner,0x466,0,0).ToInt64() -band 0xffff) -eq 59) 'Minute range differs'
    Require (([NativeUiCheck]::SendMessage($secondSpinner,0x466,0,0).ToInt64() -band 0xffff) -eq 59) 'Second range differs'
    Click $button
    Require ((Text $button) -eq 'Stop (F10)') 'Start did not start monitoring'
    Click $button
    Require ((Text $button) -eq 'Start (F10)') 'Stop did not stop'
    $null = [NativeUiCheck]::WriteText($hours,0xC,0,'1')
    Click $check
    Require ((Text $label) -in @('Stop countdown: 60:00','Stop countdown: 59:59')) 'Hour field did not set a one-hour countdown'
    Click $check
    $null = [NativeUiCheck]::WriteText($hours,0xC,0,'0')
    $null = [NativeUiCheck]::WriteText($minutes,0xC,0,'0')
    $null = [NativeUiCheck]::WriteText($seconds,0xC,0,'1')
    Click $check
    Start-Sleep -Milliseconds 2200
    Require ((Text $label) -eq 'Stop countdown: Done') "Countdown must expire even before Start; actual='$(Text $label)', minutes='$(Text $minutes)', seconds='$(Text $seconds)'"
    Click $button
    Require ((Text $button) -eq 'Stop (F10)' -and (Text $label) -ne 'Stop countdown: Done') 'Start did not reset countdown'
    Click $button
    Require ((Text $label) -eq 'Stop countdown: Ready') 'Manual stop must clear deadline'
    $null = [NativeUiCheck]::WriteText($seconds,0xC,0,'0')
    Require ((Text $label) -eq 'Stop countdown: set > 00:00') 'Zero duration differs'
    Click $check
    Require ((Text $label) -eq 'Stop countdown: Off') 'Disabling countdown differs'
    # Focus stays with the calling shell, so this also tests global registration.
    F10
    Require ((Text $button) -eq 'Stop (F10)') 'Global F10 did not start'
    F10
    Require ((Text $button) -eq 'Start (F10)') 'Global F10 did not stop'
    'PASS native window, controls, Start/Stop, countdown, zero duration, global F10'
    $failed = $false
} finally {
    if (!$process.HasExited) {
        $null = $process.CloseMainWindow()
        if (!$process.WaitForExit(5000)) {
            $process.Kill()
            if (!$failed) { throw 'Window did not shut down normally' }
        }
    }
    $process.Dispose()
}
