namespace MouseMoverApp;

public partial class Form1 : Form
{
    private readonly ActivityState state = new();
    private readonly System.Windows.Forms.Timer idleTimer = new() { Interval=1000 };
    private readonly PhysicalInputMonitor input;
    private readonly IMouseMoveStrategy strategy = new AzkiMouseMoveStrategy();
    private readonly Random random = new();
    private CancellationTokenSource? movement;
    public Form1()
    {
        InitializeComponent();
        input = new PhysicalInputMonitor(OnPhysicalInput);
        idleTimer.Tick += (_,_) => TickState();
        Load += (_,_) => {
            NativeMethods.SetWindowPos(Handle,-1,0,0,0,0,0x0010|0x0040|0x0001|0x0002);
            UpdateDisplay(TimeSpan.Zero);
        };
    }
    protected override void OnHandleCreated(EventArgs e)
    {
        base.OnHandleCreated(e);
        NativeMethods.RegisterHotKey(Handle,1,0,0x79);
    }
    protected override void OnHandleDestroyed(EventArgs e)
    {
        NativeMethods.UnregisterHotKey(Handle,1);
        base.OnHandleDestroyed(e);
    }
    protected override void WndProc(ref Message message)
    {
        if (message.Msg == 0x0312 && message.WParam == 1) Toggle();
        base.WndProc(ref message);
    }
    private void Toggle()
    {
        if (state.Monitoring || state.Moving) { Stop(); return; }
        try { input.Start(); }
        catch (Exception error)
        {
            MessageBox.Show(this,"Unable to monitor input:\r\n"+error.Message,"MouseMoverApp",MessageBoxButtons.OK,MessageBoxIcon.Error);
            return;
        }
        state.Start(Environment.TickCount64-NativeMethods.SystemIdleMilliseconds(),DateTime.Now);
        TickState();
    }
    private void Stop(bool done = false)
    {
        CancelMovement();
        input.Dispose();
        state.Stop(done);
        UpdateDisplay();
    }
    private void CancelMovement()
    {
        movement?.Cancel();
        movement?.Dispose();
        movement = null;
    }
    private void OnPhysicalInput()
    {
        bool wasMoving = state.Moving;
        state.Input(Environment.TickCount64);
        if (wasMoving)
        {
            CancelMovement();
            UpdateDisplay();
        }
    }
    private void ConfigureCountdown()
    {
        state.ConfigureCountdown(chkStopCountdown.Checked,(int)numStopMinutes.Value*60+(int)numStopSeconds.Value,DateTime.Now);
        UpdateDisplay();
    }
    private void TickState()
    {
        bool wasActive = state.Monitoring || state.Moving;
        bool hadDeadline = state.Deadline.HasValue;
        bool startMovement = state.Tick(Environment.TickCount64,DateTime.Now);
        if (hadDeadline && state.CountdownDone)
        {
            Stop(done:true);
            return;
        }
        if (wasActive && !state.Monitoring) { Stop(done:state.CountdownDone); return; }
        if (startMovement)
        {
            lblIdleInfo.Text = "Auto Start";
            pnlIdleLamp.BackColor = Color.OrangeRed;
            WindowState = FormWindowState.Normal;
            Show();
            Activate();
            NativeMethods.SetForegroundWindow(Handle);
            StartMovement();
        }
        UpdateDisplay();
    }
    private void StartMovement()
    {
        CancelMovement();
        movement = new CancellationTokenSource();
        var token = movement.Token;
        var bounds = (Screen.PrimaryScreen ?? Screen.FromControl(this)).Bounds;
        _ = Task.Run(async () => {
            try
            {
                while (true)
                {
                    token.ThrowIfCancellationRequested();
                    // Query the current primary screen for each round.
                    var context = new MouseMoveStrategyContext((Screen.PrimaryScreen?.Bounds ?? bounds),random);
                    await strategy.ExecuteAsync(context,token).ConfigureAwait(false);
                    token.ThrowIfCancellationRequested();
                    NativeMethods.mouse_event(0x02|0x04,0,0,0,0);
                    await Task.Delay(60000,token).ConfigureAwait(false);
                }
            }
            catch (OperationCanceledException) when (token.IsCancellationRequested) { }
            catch (Exception error)
            {
                if (!token.IsCancellationRequested && !IsDisposed && IsHandleCreated)
                {
                    try { BeginInvoke((Action)(() => {
                        if (token.IsCancellationRequested || IsDisposed) return;
                        Stop();
                        MessageBox.Show(this,"Unexpected error:\r\n"+error.Message,"MouseMoverApp",MessageBoxButtons.OK,MessageBoxIcon.Error);
                    })); }
                    catch (InvalidOperationException) { /* The form closed before dispatch. */ }
                }
            }
        });
    }
    private static string Clock(TimeSpan time)
    {
        long seconds = Math.Max(0,(long)time.TotalSeconds);
        return $"{seconds/60:00}:{seconds%60:00}";
    }
    private void UpdateDisplay(TimeSpan? initialIdle = null)
    {
        var idle = initialIdle ?? TimeSpan.FromMilliseconds(state.Monitoring ? Math.Max(0,Environment.TickCount64-state.LastInputTick) : NativeMethods.SystemIdleMilliseconds());
        string status = state.Moving ? "Moving (Auto)" : state.Monitoring ? "Monitoring" : "Not Started";
        pnlIdleLamp.BackColor = state.Moving ? Color.Red : state.Monitoring ? Color.LimeGreen : Color.Gray;
        btnToggle.Text = state.Monitoring || state.Moving ? "Stop (F10)" : "Start (F10)";
        lblIdleInfo.Text = status+"    Idle: "+Clock(idle)+"  In: "+Clock(TimeSpan.FromMinutes(3)-idle);
        string countdown = !chkStopCountdown.Checked ? "Off" : state.CountdownDone ? "Done" : state.Deadline.HasValue ? Clock(state.Deadline.Value-DateTime.Now) : numStopMinutes.Value+numStopSeconds.Value == 0 ? "set > 00:00" : "Ready";
        lblStopCountdown.Text = "Stop countdown: "+countdown;
        idleTimer.Enabled = state.Monitoring || state.Moving || state.Deadline.HasValue;
    }
    protected override void Dispose(bool disposing)
    {
        if (disposing)
        {
            idleTimer.Dispose();
            input.Dispose();
            CancelMovement();
        }
        base.Dispose(disposing);
    }
}
