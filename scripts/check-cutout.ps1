param([string]$Executable = 'target/release/azu-sanpo.exe')
$ErrorActionPreference = 'Stop'
. ./scripts/check-ui.ps1 -Executable $Executable -Screenshot artifacts/ui-clean-cutout.png
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class CutoutFixture {
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X, Y; }
    delegate IntPtr WindowProc(IntPtr hwnd,uint msg,IntPtr w,IntPtr l);
    static WindowProc proc = DefWindowProc;
    [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] struct WindowClass {
        public uint style; public IntPtr procedure; public int classExtra,windowExtra;
        public IntPtr instance,icon,cursor,background; public string menu,className;
    }
    [DllImport("user32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern ushort RegisterClass(ref WindowClass cls);
    [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr DefWindowProc(IntPtr hwnd,uint msg,IntPtr w,IntPtr l);
    [DllImport("kernel32.dll",CharSet=CharSet.Unicode)] static extern IntPtr GetModuleHandle(string name);
    [DllImport("gdi32.dll")] static extern IntPtr GetStockObject(int index);
    [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr CreateWindowEx(uint ex,string cls,string text,uint style,int x,int y,int w,int h,IntPtr parent,IntPtr menu,IntPtr instance,IntPtr param);
    [DllImport("user32.dll")] public static extern bool DestroyWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd,ref Point point);
    [DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr hwnd,IntPtr dc);
    [DllImport("user32.dll")] public static extern bool RedrawWindow(IntPtr hwnd,IntPtr rect,IntPtr region,uint flags);
    [DllImport("gdi32.dll")] public static extern bool BitBlt(IntPtr dest,int x,int y,int w,int h,IntPtr src,int sx,int sy,uint op);
    [DllImport("dwmapi.dll")] public static extern int DwmFlush();
    public static IntPtr CreateBackdrop(bool white,int x,int y,int width,int height) {
        var instance=GetModuleHandle(null);
        var name="CutoutFixture"+(white?"White":"Black")+System.Diagnostics.Process.GetCurrentProcess().Id;
        var cls=new WindowClass {procedure=Marshal.GetFunctionPointerForDelegate(proc),instance=instance,background=GetStockObject(white?0:4),className=name};
        if(RegisterClass(ref cls)==0 && Marshal.GetLastWin32Error()!=1410) throw new Exception("Backdrop class registration failed");
        return CreateWindowEx(0x08000088,name,"Cutout test backdrop",2147483648U,x,y,width,height,IntPtr.Zero,IntPtr.Zero,instance,IntPtr.Zero);
    }
}
'@
$preview = Start-Process -FilePath (Resolve-Path $Executable).Path -PassThru
$backdrop = [IntPtr]::Zero
try {
    Require ($preview.WaitForInputIdle(10000)) 'Preview did not initialize'
    $preview.Refresh()
    $window = $preview.MainWindowHandle
    $toggle = [NativeUiCheck]::GetDlgItem($window,117)
    $rect = [NativeUiCheck+Rect]::new()
    Require ([NativeUiCheck]::GetWindowRect($window,[ref]$rect)) 'No preview bounds'
    $origin = [CutoutFixture+Point]::new()
    Require ([CutoutFixture]::ClientToScreen($window,[ref]$origin)) 'No client origin'
    $scale = [NativeUiCheck]::GetDpiForWindow($window)/96.0
    Click $toggle
    foreach ($theme in @(@('black',4,0),@('white',6,255))) {
        $backdrop = [CutoutFixture]::CreateBackdrop(($theme[0] -eq 'white'),$rect.Left-20,$rect.Top-20,$rect.Right-$rect.Left+40,$rect.Bottom-$rect.Top+40)
        Require ($backdrop -ne 0) 'Could not create controlled backdrop'
        Require ([CutoutFixture]::SetWindowPos($backdrop,$window,$rect.Left-20,$rect.Top-20,$rect.Right-$rect.Left+40,$rect.Bottom-$rect.Top+40,0x50)) 'Could not place backdrop beneath app'
        $null = [CutoutFixture]::SetWindowPos($window,[IntPtr](-1),0,0,0,0,0x13)
        $null = [CutoutFixture]::RedrawWindow($backdrop,0,0,0x105)
        $null = [CutoutFixture]::RedrawWindow($window,0,0,0x185)
        $null = [CutoutFixture]::DwmFlush()
        Start-Sleep -Milliseconds 100
        $bitmap = [Drawing.Bitmap]::new($rect.Right-$rect.Left+40,$rect.Bottom-$rect.Top+40)
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        $dc = $graphics.GetHdc()
        $screen = [CutoutFixture]::GetDC(0)
        try { Require ([CutoutFixture]::BitBlt($dc,0,0,$bitmap.Width,$bitmap.Height,$screen,$rect.Left-20,$rect.Top-20,0x40CC0020)) 'Composited capture failed' }
        finally { $null=[CutoutFixture]::ReleaseDC(0,$screen); $graphics.ReleaseHdc($dc); $graphics.Dispose() }
        try {
            # All captures are over our own solid backdrop; no desktop content is saved.
            # Compare with the same captured backdrop, allowing display color management.
            $reference = $bitmap.GetPixel(5,25)
            foreach ($sample in @(@(4,60),@(50,59),@(200,59),@(290,59),@(100,97),@(198,99),@(306,14))) {
                $x = 20+$origin.X-$rect.Left+[int][Math]::Round($sample[0]*$scale)
                $y = 20+$origin.Y-$rect.Top+[int][Math]::Round($sample[1]*$scale)
                $pixel = $bitmap.GetPixel($x,$y)
                Require ($pixel.R -eq $reference.R -and $pixel.G -eq $reference.G -and $pixel.B -eq $reference.B) "Opaque remnant at $($sample[0]),$($sample[1]) over $($theme[0]): $pixel vs $reference"
            }
            $inputPixel = $bitmap.GetPixel(20+$origin.X-$rect.Left+[int](158*$scale),20+$origin.Y-$rect.Top+[int](100*$scale))
            Require ($inputPixel.R -gt 160 -and [Math]::Abs($inputPixel.R-$inputPixel.G) -le 2 -and [Math]::Abs($inputPixel.G-$inputPixel.B) -le 2) "Input field must remain opaque: $inputPixel"
            $bitmap.Save((Join-Path (Get-Location) "artifacts/cutout-on-$($theme[0]).png"),[Drawing.Imaging.ImageFormat]::Png)
            "PASS composited cutout over $($theme[0]) backdrop"
        } finally { $bitmap.Dispose() }
        $null = [CutoutFixture]::DestroyWindow($backdrop)
        $backdrop = [IntPtr]::Zero
    }
    Click $toggle
} finally {
    if ($backdrop -ne 0) { $null = [CutoutFixture]::DestroyWindow($backdrop) }
    if (!$preview.HasExited) { $null=$preview.CloseMainWindow(); Require ($preview.WaitForExit(5000)) 'Preview did not close normally' }
    $preview.Dispose()
}
