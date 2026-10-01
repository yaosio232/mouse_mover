using MouseMoverApp;
using System.Drawing;
using System.Reflection;
using System.Runtime.InteropServices;

internal static class SmokeChecks
{
    [DllImport("user32.dll")]
    private static extern void keybd_event(byte key, byte scan, uint flags, nuint extra);
    private static T Field<T>(object instance, string name) => (T)instance.GetType().GetField(name,BindingFlags.Instance|BindingFlags.NonPublic)!.GetValue(instance)!;
    private static void Invoke(object instance, string name, params object[] args) => instance.GetType().GetMethod(name,BindingFlags.Instance|BindingFlags.NonPublic)!.Invoke(instance,args);
    private static void Require(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Pump(int milliseconds = 100)
    {
        var until = Environment.TickCount64+milliseconds;
        while (Environment.TickCount64 < until) { Application.DoEvents(); Thread.Sleep(5); }
    }
    internal static void Run()
    {
        Exception? failure = null;
        var thread = new Thread(() => {
            try { OnUiThread(); }
            catch (Exception error) { failure = error; }
        });
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        thread.Join();
        if (failure != null) throw failure;
    }
    private static void OnUiThread()
    {
        Application.SetHighDpiMode(HighDpiMode.SystemAware);
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        using var form = new Form1();
        form.Show();
        Pump();
        var state = Field<ActivityState>(form,"state");
        var button = Field<Button>(form,"btnToggle");
        var countdown = Field<CheckBox>(form,"chkStopCountdown");
        var minutes = Field<NumericUpDown>(form,"numStopMinutes");
        var seconds = Field<NumericUpDown>(form,"numStopSeconds");
        var countdownLabel = Field<Label>(form,"lblStopCountdown");
        var status = Field<Label>(form,"lblIdleInfo");
        Require(form.Text == "AZu Sanpo" && form.TopMost && !form.MaximizeBox && !form.MinimizeBox,"window attributes differ");
        Require(button.Text == "Start (F10)" && !state.Monitoring && minutes.Value == 180 && seconds.Value == 0 && !countdown.Checked,"startup differs");
        Directory.CreateDirectory("artifacts");
        using (var bitmap = new Bitmap(form.Width,form.Height))
        {
            form.DrawToBitmap(bitmap,new Rectangle(0,0,bitmap.Width,bitmap.Height));
            bitmap.Save(Path.GetFullPath("artifacts/ui-rebuilt.png"));
        }
        button.PerformClick();
        Require(state.Monitoring && !state.Moving && button.Text == "Stop (F10)","Start must monitor");
        var monitor = Field<PhysicalInputMonitor>(form,"input");
        Require(Field<nint>(monitor,"mouseHook") != 0 && Field<nint>(monitor,"keyboardHook") != 0,"both real hooks must install");

        var originalCursor = Cursor.Position;
        var observed = new List<(nint Message,uint Flags)>();
        NativeMethods.HookProc probe = (code,w,l) => {
            if (code >= 0) {
                var packet = Marshal.PtrToStructure<NativeMethods.MouseHookData>(l);
                observed.Add((w,packet.Flags));
            }
            return NativeMethods.CallNextHookEx(0,code,w,l);
        };
        var probeHandle = NativeMethods.SetWindowsHookEx(14,probe,NativeMethods.GetModuleHandle(null),0);
        bool physicalKeyboard = false;
        NativeMethods.HookProc keyboardProbe = (code,w,l) => {
            if (code >= 0 && (Marshal.PtrToStructure<NativeMethods.KeyboardHookData>(l).Flags & 0x10) == 0) physicalKeyboard = true;
            return NativeMethods.CallNextHookEx(0,code,w,l);
        };
        var keyboardProbeHandle = NativeMethods.SetWindowsHookEx(13,keyboardProbe,NativeMethods.GetModuleHandle(null),0);
        try
        {
            // Click our own inert status label, so no other application receives the test click.
            Cursor.Position = status.PointToScreen(new Point(5,5));
            Pump();
            long baseline = state.LastInputTick;
            observed.Clear();
            physicalKeyboard = false;
            NativeMethods.mouse_event(0x02|0x04,0,0,0,0);
            Pump();
            Require(observed.Any(e => e.Message == 0x0201 && (e.Flags & 1) != 0) && observed.Any(e => e.Message == 0x0202 && (e.Flags & 1) != 0),"real injected click flags missing");
            if (!physicalKeyboard && observed.All(e => (e.Flags & 1) != 0))
                Require(state.LastInputTick == baseline,"injected click reset idle");
            else Console.WriteLine("NOTE concurrent non-injected mouse activity; idle isolation checked by exact callback metadata below");
            observed.Clear();
            physicalKeyboard = false;
            baseline = state.LastInputTick;
            Cursor.Position = status.PointToScreen(new Point(10,5));
            Pump();
            if (!physicalKeyboard && observed.All(e => (e.Flags & 1) != 0))
                Require(state.LastInputTick == baseline,$"program cursor change reset idle: {baseline} -> {state.LastInputTick}, observed={observed.Count}");
            else Console.WriteLine("NOTE cursor-position idle isolation requires a quiet desktop; concurrent non-injected movement detected");

            // Exercise real hook callbacks with physical/injected metadata, without generating a desktop action.
            var data = new NativeMethods.MouseHookData { Flags=1 };
            nint memory = Marshal.AllocHGlobal(Marshal.SizeOf<NativeMethods.MouseHookData>());
            try
            {
                state.Start(Environment.TickCount64-180000,DateTime.Now);
                Require(state.Tick(Environment.TickCount64,DateTime.Now),"idle threshold not reached");
                long idleBaseline = state.LastInputTick;
                Marshal.StructureToPtr(data,memory,false);
                Invoke(monitor,"MouseEvent",0,(nint)0x0200,memory);
                Require(state.Moving,"injected metadata canceled moving");
                Require(state.LastInputTick == idleBaseline,"injected metadata reset idle clock");
                data.Flags = 0;
                Marshal.StructureToPtr(data,memory,false);
                Invoke(monitor,"MouseEvent",0,(nint)0x0200,memory);
                Require(state.Monitoring && !state.Moving,"physical metadata must restore monitoring");
            }
            finally { Marshal.FreeHGlobal(memory); }
            nint keyboardMemory = Marshal.AllocHGlobal(Marshal.SizeOf<NativeMethods.KeyboardHookData>());
            try
            {
                state.Start(Environment.TickCount64-180000,DateTime.Now);
                state.Tick(Environment.TickCount64,DateTime.Now);
                Marshal.StructureToPtr(new NativeMethods.KeyboardHookData { Flags=0x10 },keyboardMemory,false);
                Invoke(monitor,"KeyboardEvent",0,(nint)0x0100,keyboardMemory);
                Require(state.Moving,"injected keyboard event canceled moving");
                Marshal.StructureToPtr(new NativeMethods.KeyboardHookData { Flags=1 },keyboardMemory,false);
                Invoke(monitor,"KeyboardEvent",0,(nint)0x0100,keyboardMemory);
                Require(state.Monitoring && !state.Moving,"physical extended keyboard key must restore monitoring");
            }
            finally { Marshal.FreeHGlobal(keyboardMemory); }
        }
        finally { NativeMethods.UnhookWindowsHookEx(probeHandle); NativeMethods.UnhookWindowsHookEx(keyboardProbeHandle); GC.KeepAlive(probe); GC.KeepAlive(keyboardProbe); Cursor.Position = originalCursor; }

        minutes.Value = 0;
        seconds.Value = 1;
        countdown.Checked = true;
        Pump(2100);
        Require(state.CountdownDone && !state.Monitoring && countdownLabel.Text == "Stop countdown: Done","countdown must fully stop");
        Require(Field<nint>(monitor,"mouseHook") == 0 && Field<nint>(monitor,"keyboardHook") == 0,"stop must remove hooks");
        button.PerformClick();
        Require(state.Monitoring && !state.CountdownDone && state.Deadline.HasValue,"Start must restart countdown");
        button.PerformClick();
        Require(countdownLabel.Text == "Stop countdown: Ready","manual stop must show Ready");
        seconds.Value = 0;
        Require(countdownLabel.Text == "Stop countdown: set > 00:00","zero countdown differs");
        countdown.Checked = false;
        Require(countdownLabel.Text == "Stop countdown: Off","disabled countdown differs");

        // F10 must work while another form has focus.
        using var other = new Form { Text="Hotkey check",Size=new Size(150,80) };
        other.Show();
        other.Activate();
        Pump();
        keybd_event(0x79,0,0,0); keybd_event(0x79,0,2,0);
        Pump();
        Require(state.Monitoring,"global F10 did not start while unfocused");
        keybd_event(0x79,0,0,0); keybd_event(0x79,0,2,0);
        Pump();
        Require(!state.Monitoring && button.Text == "Start (F10)","second F10 did not stop");
        form.Close();
        Pump();
    }
}
