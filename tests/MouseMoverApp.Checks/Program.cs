using MouseMoverApp;
using System.Drawing;

int failures = 0;
void Check(string name, Action action)
{
    try { action(); Console.WriteLine($"PASS {name}"); }
    catch (Exception error) { failures++; Console.WriteLine($"FAIL {name}: {error.Message}"); }
}
void Expect(bool condition, string message)
{
    if (!condition) throw new Exception(message);
}
var now = new DateTime(2026, 10, 1, 12, 0, 0);
Check("idle threshold and input return to monitoring", () => {
    var state = new ActivityState();
    state.Start(1000, now);
    Expect(!state.Tick(180999, now), "moved before three minutes");
    Expect(state.Tick(181000, now) && state.Moving, "did not move at three minutes");
    state.Input(181001);
    Expect(state.Monitoring && !state.Moving, "input should preserve monitoring");
    Expect(!state.Tick(361000, now), "input must restart idle threshold");
    Expect(state.Tick(361001, now), "did not resume after next idle interval");
});
Check("countdown expires before idle movement", () => {
    var state = new ActivityState();
    state.ConfigureCountdown(true, 180, now);
    state.Start(0, now);
    Expect(!state.Tick(180000, now.AddSeconds(180)), "expired countdown started movement");
    Expect(!state.Monitoring && !state.Moving && state.CountdownDone, "expiry must fully stop");
    state.Start(180000, now.AddSeconds(180));
    Expect(!state.CountdownDone && state.Deadline == now.AddSeconds(360), "restart must reset countdown");
});
Check("manual stop, countdown before start, zero and disabled", () => {
    var state = new ActivityState();
    state.ConfigureCountdown(true, 5, now);
    state.Tick(0, now.AddSeconds(5));
    Expect(state.CountdownDone, "countdown must run before Start");
    state.Start(0, now);
    state.Stop();
    Expect(state.Deadline == null && !state.CountdownDone && !state.Monitoring, "manual stop must clear deadline");
    state.ConfigureCountdown(true, 0, now);
    Expect(state.Deadline == null && !state.CountdownDone, "zero must not expire");
    state.ConfigureCountdown(false, 30, now);
    Expect(state.Deadline == null, "disabled must clear deadline");
});
Check("input preserves countdown deadline", () => {
    var state = new ActivityState();
    state.ConfigureCountdown(true, 600, now);
    state.Start(0, now);
    state.Tick(180000, now.AddSeconds(180));
    state.Input(180001);
    Expect(state.Deadline == now.AddSeconds(600), "input changed stop deadline");
});
Check("446 samples including joints, fixed endpoints and centering", () => {
    var path = AzkiMouseMoveStrategy.BuildPath();
    Expect(path.Count == 446, $"expected 446, got {path.Count}");
    Expect(path[0] == new Point(0, 360), "wrong first point");
    Expect(path[26] == new Point(78, 15) && path[27] == path[26], "Bezier endpoint/joint lost");
    Expect(path[^1] == new Point(960, 177), "wrong final point");
    var centered = AzkiMouseMoveStrategy.Center(path, new Rectangle(0, 0, 1920, 1080));
    Expect(centered[0] == new Point(480, 360 + (1080 - (path.Max(p => p.Y) - path.Min(p => p.Y))) / 2 - path.Min(p => p.Y)), "centering incorrect");
    var nonzero = AzkiMouseMoveStrategy.Center(path, new Rectangle(100, -100, 1920, 1080));
    Expect(centered.SequenceEqual(nonzero), "spec offset must ignore screen origin");
});
Check("relative movement rounds cumulatively, clips and cancels", () => {
    var context = new MouseMoveStrategyContext(new Rectangle(0, 0, 100, 100), new Random(1));
    Expect(MouseMoveStrategyContext.RoundStep(2.5, -2.5, Point.Empty) == new Point(2, -2), "midpoint must round to even");
    Expect(MouseMoveStrategyContext.RoundStep(5, -5, new Point(2,-2)) == new Point(3, -3), "cumulative remainder lost");
    Expect(context.Clamp(new Point(-10, 120)) == new Point(0, 99), "screen clipping failed");
    using var canceled = new CancellationTokenSource();
    canceled.Cancel();
    try { context.MoveByDeltaAsync(1, 1, 1, 0, canceled.Token).GetAwaiter().GetResult(); throw new Exception("canceled movement executed"); }
    catch (OperationCanceledException) { }
});
if (!args.Contains("--core")) Check("WinForms lifecycle, countdown, hooks and global F10", SmokeChecks.Run);
Console.WriteLine($"{failures} failed");
return failures == 0 ? 0 : 1;
