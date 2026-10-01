using System.Drawing;

namespace MouseMoverApp;

public sealed class MouseMoveStrategyContext(Rectangle bounds, Random random)
{
    public Rectangle Bounds { get; } = bounds;
    public Random Random { get; } = random;
    public static Point RoundStep(double x, double y, Point applied) => new((int)Math.Round(x)-applied.X, (int)Math.Round(y)-applied.Y);
    public Point Clamp(Point point) => new(Math.Clamp(point.X,Bounds.Left,Bounds.Right-1),Math.Clamp(point.Y,Bounds.Top,Bounds.Bottom-1));
    public async Task MoveByDeltaAsync(int dx, int dy, int steps, int delayMs, CancellationToken token)
    {
        steps = Math.Max(1, steps);
        double x = 0, y = 0;
        var applied = Point.Empty;
        for (int step = 0; step < steps; step++)
        {
            token.ThrowIfCancellationRequested();
            x += (double)dx/steps;
            y += (double)dy/steps;
            var delta = RoundStep(x, y, applied);
            applied.Offset(delta);
            var current = Cursor.Position;
            current.Offset(delta);
            Cursor.Position = Clamp(current);
            if (delayMs > 0) await Task.Delay(delayMs, token).ConfigureAwait(false);
        }
    }
}
