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
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool GetLayeredWindowAttributes(IntPtr hwnd, out uint key, out byte alpha, out uint flags);
    [DllImport("user32.dll")] public static extern int MapWindowPoints(IntPtr from, IntPtr to, ref Rect rect, uint points);
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
    $backgroundToggle = [NativeUiCheck]::GetDlgItem($window,117)
    Require ([NativeUiCheck]::GetDlgItem($window,115) -eq 0) 'Status prefix must be removed'
    # Countdown feedback shares the native checkbox caption, keeping three rows.
    $label = $check
    $client = [NativeUiCheck+Rect]::new()
    Require ([NativeUiCheck]::GetClientRect($window,[ref]$client)) 'No client bounds'
    $scale = [NativeUiCheck]::GetDpiForWindow($window)/96.0
    Require (($client.Right/$scale) -le 350 -and ($client.Bottom/$scale) -le 140) 'Three-row window must stay compact (max 350 x 140 logical pixels)'
    $rows = @()
    $buttonBounds = $null
    foreach ($id in @(100,102,103)) {
        $handle = [NativeUiCheck]::GetDlgItem($window,$id)
        $bounds = [NativeUiCheck+Rect]::new()
        Require ($handle -ne 0 -and [NativeUiCheck]::GetWindowRect($handle,[ref]$bounds)) "Missing row control $id"
        $null = [NativeUiCheck]::MapWindowPoints(0,$window,[ref]$bounds,2)
        Require ($bounds.Left -ge 0 -and $bounds.Top -ge 0 -and $bounds.Right -le $client.Right -and $bounds.Bottom -le $client.Bottom) "Row $id is clipped"
        if ($id -eq 100) {
            Require (($bounds.Bottom-$bounds.Top)/$scale -ge 39) 'Primary button must be at least 40 logical pixels high'
            $buttonBounds = $bounds
        }
        $rows += $bounds.Top
    }
    Require (@($rows | Sort-Object -Unique).Count -eq 3) 'Expected exactly three compact rows'
    $iconBounds = [NativeUiCheck+Rect]::new()
    Require ($backgroundToggle -ne 0 -and [NativeUiCheck]::GetWindowRect($backgroundToggle,[ref]$iconBounds)) 'Missing background icon'
    $null = [NativeUiCheck]::MapWindowPoints(0,$window,[ref]$iconBounds,2)
    Require ($iconBounds.Left -ge $buttonBounds.Right -and $iconBounds.Right -le $client.Right) 'Background icon must fit beside the primary button'
    Require ([Math]::Abs(($iconBounds.Top+$iconBounds.Bottom-$buttonBounds.Top-$buttonBounds.Bottom)/2) -le 3*$scale) 'Background icon must align with the primary button'
    Require (([NativeUiCheck]::GetWindowLong($backgroundToggle,-16) -band 0x10000) -ne 0) 'Background icon must support keyboard focus'
    $detailBounds = [NativeUiCheck+Rect]::new()
    Require ([NativeUiCheck]::GetWindowRect([NativeUiCheck]::GetDlgItem($window,114),[ref]$detailBounds)) 'Missing idle information'
    $null = [NativeUiCheck]::MapWindowPoints(0,$window,[ref]$detailBounds,2)
    Require ([Math]::Abs($detailBounds.Top-$rows[1]) -le 3*$scale) 'Status and time must share a row'
    Require ($detailBounds.Right -le $client.Right -and $detailBounds.Bottom -le $client.Bottom) 'Idle information is clipped'
    $inBounds = [NativeUiCheck+Rect]::new()
    $inLabel = [NativeUiCheck]::GetDlgItem($window,116)
    Require ($inLabel -ne 0 -and [NativeUiCheck]::GetWindowRect($inLabel,[ref]$inBounds)) 'Missing In countdown'
    $null = [NativeUiCheck]::MapWindowPoints(0,$window,[ref]$inBounds,2)
    Require ([Math]::Abs($inBounds.Top-$rows[1]) -le 3*$scale -and $inBounds.Right -le $client.Right) 'In countdown must fit the status row'
    Require ((Text ([NativeUiCheck]::GetDlgItem($window,114))) -eq 'IDLE 00:00:00') 'Initial IDLE must show hours, minutes and seconds'
    Require ((Text $inLabel) -eq 'In 03:00') 'Initial In countdown must show three minutes'
    foreach ($handle in @($hours,$minutes,$seconds)) {
        $bounds = [NativeUiCheck+Rect]::new()
        Require ([NativeUiCheck]::GetWindowRect($handle,[ref]$bounds)) 'Missing duration field bounds'
        $null = [NativeUiCheck]::MapWindowPoints(0,$window,[ref]$bounds,2)
        Require ([Math]::Abs($bounds.Top-$rows[2]) -le 3*$scale) 'Duration fields must share the checkbox row'
        Require ($bounds.Right -le $client.Right -and $bounds.Bottom -le $client.Bottom) 'Duration field is clipped'
    }
    Require ((Text $button) -eq 'Start (F10)') 'Startup button differs'
    Require ((Text $hours) -eq '3' -and (Text $minutes) -eq '0' -and (Text $seconds) -eq '0') 'Default duration differs'
    Require ((Text $label) -eq 'Stop after') 'Countdown not initially off'
    Require ((Text $backgroundToggle) -eq 'Make background transparent') 'Background icon must have an accessible action name'
    Click $backgroundToggle
    Require (([NativeUiCheck]::GetWindowLong($window,-20) -band 0x80000) -ne 0) 'Transparent background mode was not enabled'
    [uint32]$key = 0; [byte]$alpha = 0; [uint32]$flags = 0
    Require (![NativeUiCheck]::GetLayeredWindowAttributes($window,[ref]$key,[ref]$alpha,[ref]$flags)) 'Transparent mode must use a per-pixel surface, not color-key click-through holes'
    Require (([NativeUiCheck]::GetWindowLong($window,-20) -band 0x20) -eq 0) 'Controls must remain clickable in transparent mode'
    Require ((Text $backgroundToggle) -eq 'Restore opaque background') 'Transparent icon action name did not update'
    Click $backgroundToggle
    Require (([NativeUiCheck]::GetWindowLong($window,-20) -band 0x80000) -eq 0) 'Opaque background mode was not restored'
    Require ((Text $button) -eq 'Start (F10)' -and (Text $hours) -eq '3' -and (Text $label) -eq 'Stop after') 'Background toggle changed monitoring or duration state'
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
    Require ((Text $label) -in @('Stop 01:00:00','Stop 00:59:59')) 'Hour field did not set a one-hour countdown'
    Click $backgroundToggle
    Click $backgroundToggle
    Require ([NativeUiCheck]::SendMessage($check,0xF0,0,0).ToInt64() -eq 1) 'Background toggle changed checkbox state'
    Require ((Text $label) -in @('Stop 01:00:00','Stop 00:59:59')) 'Background toggle changed active countdown'
    Click $check
    $null = [NativeUiCheck]::WriteText($hours,0xC,0,'0')
    $null = [NativeUiCheck]::WriteText($minutes,0xC,0,'0')
    $null = [NativeUiCheck]::WriteText($seconds,0xC,0,'1')
    Click $check
    Start-Sleep -Milliseconds 2200
    Require ((Text $label) -eq 'Stop done') "Countdown must expire even before Start; actual='$(Text $label)', minutes='$(Text $minutes)', seconds='$(Text $seconds)'"
    Click $button
    Require ((Text $button) -eq 'Stop (F10)' -and (Text $label) -ne 'Stop done') 'Start did not reset countdown'
    Click $button
    Require ((Text $label) -eq 'Stop ready') 'Manual stop must clear deadline'
    $null = [NativeUiCheck]::WriteText($seconds,0xC,0,'0')
    Require ((Text $label) -eq 'Set > 00:00') 'Zero duration differs'
    Click $check
    Require ((Text $label) -eq 'Stop after') 'Disabling countdown differs'
    # Focus stays with the calling shell, so this also tests global registration.
    F10
    Require ((Text $button) -eq 'Stop (F10)') 'Global F10 did not start'
    F10
    Require ((Text $button) -eq 'Start (F10)') 'Global F10 did not stop'
    'PASS three-row window, controls, background toggle, Start/Stop, countdown, zero duration, global F10'
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
