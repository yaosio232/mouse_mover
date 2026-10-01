using System.ComponentModel;
using System.Runtime.InteropServices;

namespace MouseMoverApp;

internal sealed class PhysicalInputMonitor : IDisposable
{
    private readonly Action activity;
    private readonly NativeMethods.HookProc mouseCallback;
    private readonly NativeMethods.HookProc keyboardCallback;
    private nint mouseHook, keyboardHook;
    internal PhysicalInputMonitor(Action activity)
    {
        this.activity = activity;
        mouseCallback = MouseEvent;
        keyboardCallback = KeyboardEvent;
    }
    internal void Start()
    {
        if (mouseHook != 0) return;
        var module = NativeMethods.GetModuleHandle(null);
        mouseHook = NativeMethods.SetWindowsHookEx(14,mouseCallback,module,0);
        if (mouseHook == 0) throw new Win32Exception(Marshal.GetLastWin32Error());
        keyboardHook = NativeMethods.SetWindowsHookEx(13,keyboardCallback,module,0);
        if (keyboardHook == 0)
        {
            var error = new Win32Exception(Marshal.GetLastWin32Error());
            Dispose();
            throw error;
        }
    }
    private nint MouseEvent(int code, nint wParam, nint lParam)
    {
        if (code >= 0 && (Marshal.PtrToStructure<NativeMethods.MouseHookData>(lParam).Flags & 1) == 0) activity();
        return NativeMethods.CallNextHookEx(mouseHook,code,wParam,lParam);
    }
    private nint KeyboardEvent(int code, nint wParam, nint lParam)
    {
        // LLKHF_INJECTED is bit 4; keyboard bit 0 means an extended physical key.
        if (code >= 0 && (Marshal.PtrToStructure<NativeMethods.KeyboardHookData>(lParam).Flags & 0x10) == 0) activity();
        return NativeMethods.CallNextHookEx(keyboardHook,code,wParam,lParam);
    }
    public void Dispose()
    {
        if (mouseHook != 0) NativeMethods.UnhookWindowsHookEx(mouseHook);
        if (keyboardHook != 0) NativeMethods.UnhookWindowsHookEx(keyboardHook);
        mouseHook = keyboardHook = 0;
    }
}
