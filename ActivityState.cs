namespace MouseMoverApp;

public sealed class ActivityState
{
    public bool Monitoring { get; private set; }
    public bool Moving { get; private set; }
    public bool CountdownDone { get; private set; }
    public DateTime? Deadline { get; private set; }
    public long LastInputTick { get; private set; }
    private bool countdownEnabled;
    private int countdownSeconds;
    public void Start(long lastInputTick, DateTime now)
    {
        Monitoring = true;
        Moving = false;
        LastInputTick = lastInputTick;
        ResetCountdown(now);
    }
    public void Stop(bool done = false)
    {
        Monitoring = Moving = false;
        Deadline = null;
        CountdownDone = done;
    }
    public void Input(long tick)
    {
        LastInputTick = tick;
        Moving = false;
    }
    public void ConfigureCountdown(bool enabled, int seconds, DateTime now)
    {
        countdownEnabled = enabled;
        countdownSeconds = Math.Max(0, seconds);
        ResetCountdown(now);
    }
    private void ResetCountdown(DateTime now)
    {
        CountdownDone = false;
        Deadline = countdownEnabled && countdownSeconds > 0 ? now.AddSeconds(countdownSeconds) : null;
    }
    public bool Tick(long tick, DateTime now)
    {
        if (Deadline.HasValue && now >= Deadline.Value)
        {
            Stop(done: true);
            return false;
        }
        if (Monitoring && !Moving && tick - LastInputTick >= 180000)
        {
            Moving = true;
            return true;
        }
        return false;
    }
}
