param([string]$Executable = 'target/release/azu-sanpo.exe')
$ErrorActionPreference = 'Stop'
# Reuse the existing native control/state checks and helpers.
. ./scripts/check-ui.ps1 -Executable $Executable
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PointerCheck {
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X,Y; }
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(Point point);
    [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr hwnd,uint flags);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out Point point);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags,uint x,uint y,uint data,UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool RedrawWindow(IntPtr hwnd,IntPtr rect,IntPtr region,uint flags);
    [DllImport("dwmapi.dll")] public static extern int DwmFlush();
}
'@
function Bounds([IntPtr]$Handle) {
    $bounds = [NativeUiCheck+Rect]::new()
    Require ([NativeUiCheck]::GetWindowRect($Handle,[ref]$bounds)) 'Missing pointer target bounds'
    $bounds
}
function Require-Hit([IntPtr]$Handle,[int]$X,[int]$Y) {
    $point = [PointerCheck+Point]::new(); $point.X=$X; $point.Y=$Y
    $hit = [PointerCheck]::WindowFromPoint($point)
    Require ($hit -eq $Handle) "Pointer misses control $id ($Handle) in $mode mode at $X,$Y; actual target=$hit"
}
function Pointer-Click([IntPtr]$Handle,[int]$X,[int]$Y) {
    # Never click an unrelated underlying window if a hit-test fails.
    Require-Hit $Handle $X $Y
    Require ([PointerCheck]::SetCursorPos($X,$Y)) 'Could not position pointer'
    [PointerCheck]::mouse_event(2,0,0,0,[UIntPtr]::Zero)
    [PointerCheck]::mouse_event(4,0,0,0,[UIntPtr]::Zero)
    Start-Sleep -Milliseconds 100
}
function Flush-Ui([IntPtr]$Window) {
    $null=[PointerCheck]::RedrawWindow($Window,0,0,0x185)
    $null=[PointerCheck]::DwmFlush()
    Start-Sleep -Milliseconds 100
}
function Require-Eventually([scriptblock]$Condition,[string]$Message) {
    # Wait for the one injected click to finish; never retry a failed click.
    $deadline=[DateTime]::UtcNow.AddSeconds(2)
    while(!(& $Condition)) {
        if([DateTime]::UtcNow -ge $deadline) { throw $Message }
        Start-Sleep -Milliseconds 10
    }
}
$originalPointer=[PointerCheck+Point]::new()
$null=[PointerCheck]::GetCursorPos([ref]$originalPointer)
$preview=Start-Process -FilePath (Resolve-Path $Executable).Path -PassThru
try {
    Require ($preview.WaitForInputIdle(10000)) 'Pointer preview did not initialize'
    $preview.Refresh(); $window=$preview.MainWindowHandle
    $button=[NativeUiCheck]::GetDlgItem($window,100)
    $check=[NativeUiCheck]::GetDlgItem($window,103)
    $icon=[NativeUiCheck]::GetDlgItem($window,117)
    foreach ($mode in @('opaque','transparent')) {
        if($mode -eq 'transparent') { Click $icon }
        Flush-Ui $window
        foreach ($id in @(100,103,117,111,104,105,112,106,107)) {
            $handle=[NativeUiCheck]::GetDlgItem($window,$id)
            $rect=Bounds $handle
            # Check every target pixel, including transparent caption gaps,
            # icon padding, rounded corners, edit text and both spinner arrows.
            # Native up/down buddy borders overlap the edit by two pixels.
            for($y=$rect.Top;$y -lt $rect.Bottom;$y++) {
                for($x=$rect.Left+$(if($id -in @(112,106,107)){3}else{0});$x -lt $rect.Right;$x++) {
                    Require-Hit $handle $x $y
                }
            }
        }
        if($mode -eq 'transparent') {
            $buttonRect=Bounds $button
            $point=[PointerCheck+Point]::new()
            $scale=[NativeUiCheck]::GetDpiForWindow($window)/96.0
            $point.X=$buttonRect.Left-[int](4*$scale)
            $point.Y=$buttonRect.Top+[int](47*$scale)
            $hit=[PointerCheck]::WindowFromPoint($point)
            Require ([PointerCheck]::GetAncestor($hit,2) -ne $window) 'Empty cutout background must retain click-through'
        }
        $rect=Bounds $button
        Pointer-Click $button ($rect.Left+2) ($rect.Top+2)
        Require ((Text $button) -eq 'Stop (F10)') "Pointer Start failed in $mode mode"
        Pointer-Click $button ($rect.Right-3) ($rect.Bottom-3)
        Require ((Text $button) -eq 'Start (F10)') "Pointer Stop failed in $mode mode"
        foreach($caption in @('Stop (F10)','Start (F10)','Stop (F10)','Start (F10)')) {
            Pointer-Click $button ([int](($rect.Left+$rect.Right)/2)) ([int](($rect.Top+$rect.Bottom)/2))
            Require-Eventually { (Text $button) -eq $caption } "Rapid repeated primary clicks failed in $mode mode; expected=$caption"
        }
        $rect=Bounds $check
        # Use a blank portion of the caption, not the painted checkbox glyph.
        Pointer-Click $check ($rect.Right-3) ($rect.Top+2)
        Require ([NativeUiCheck]::SendMessage($check,0xF0,0,0).ToInt64() -eq 1) "Pointer checkbox enable failed in $mode mode"
        Pointer-Click $check ($rect.Right-3) ($rect.Bottom-3)
        Require ([NativeUiCheck]::SendMessage($check,0xF0,0,0).ToInt64() -eq 0) "Pointer checkbox disable failed in $mode mode"
        foreach($expected in @(1,0)) {
            [NativeUiCheck]::keybd_event(0x20,0,0,[UIntPtr]::Zero)
            [NativeUiCheck]::keybd_event(0x20,0,2,[UIntPtr]::Zero)
            Start-Sleep -Milliseconds 100
            Require ([NativeUiCheck]::SendMessage($check,0xF0,0,0).ToInt64() -eq $expected) "Checkbox Space key failed in $mode mode"
        }
        foreach($expected in @(1,0,1,0)) {
            Pointer-Click $check ($rect.Right-3) ($rect.Top+2)
            Require-Eventually { [NativeUiCheck]::SendMessage($check,0xF0,0,0).ToInt64() -eq $expected } "Rapid repeated checkbox clicks failed in $mode mode; expected=$expected"
        }
        foreach($pair in @(@(112,111),@(106,104),@(107,105))) {
            $spinner=[NativeUiCheck]::GetDlgItem($window,$pair[0])
            $edit=[NativeUiCheck]::GetDlgItem($window,$pair[1])
            $before=[int](Text $edit); $rect=Bounds $spinner
            $x=[int](($rect.Left+$rect.Right)/2)
            Pointer-Click $spinner $x ($rect.Top+3)
            Require ([int](Text $edit) -eq ($before+1)) "Spinner $($pair[0]) increment failed in $mode mode; expected=$($before+1), actual=$(Text $edit)"
            Pointer-Click $spinner $x ($rect.Bottom-4)
            Require ([int](Text $edit) -eq $before) "Spinner $($pair[0]) decrement failed in $mode mode; expected=$before, actual=$(Text $edit)"
        }
        "PASS full pointer hit rectangles and actual mouse actions in $mode mode"
    }
    $rect=Bounds $icon
    Pointer-Click $icon ($rect.Left+1) ($rect.Top+1)
    Require ((Text $icon) -eq 'Make background transparent') 'Pointer icon padding failed to restore opaque mode'
    foreach($caption in @('Restore opaque background','Make background transparent','Restore opaque background','Make background transparent')) {
        Pointer-Click $icon ($rect.Left+1) ($rect.Top+1)
        Require-Eventually { (Text $icon) -eq $caption } 'Rapid repeated transparency icon clicks failed'
    }
} finally {
    $null=[PointerCheck]::SetCursorPos($originalPointer.X,$originalPointer.Y)
    if(!$preview.HasExited) { $null=$preview.CloseMainWindow(); Require ($preview.WaitForExit(5000)) 'Pointer preview did not close normally' }
    $preview.Dispose()
}
