using System.Drawing;

namespace MouseMoverApp;

public sealed class AzkiMouseMoveStrategy : IMouseMoveStrategy
{
    public async Task ExecuteAsync(MouseMoveStrategyContext context, CancellationToken token)
    {
        foreach (var point in Center(BuildPath(),context.Bounds))
        {
            token.ThrowIfCancellationRequested();
            var current = Cursor.Position;
            await context.MoveByDeltaAsync(point.X-current.X,point.Y-current.Y,1,0,token).ConfigureAwait(false);
            await Task.Delay(context.Random.Next(5,10),token).ConfigureAwait(false);
        }
    }
    public static List<Point> BuildPath()
    {
        const int s = 3, topY = 0, midY = 195, baseY = 390;
        int x = 0;
        var points = new List<Point> { new(0, 360) };
        void B(Point c1, Point c2, Point end, int n)
        {
            var start = points[^1];
            for (int i = 0; i <= n; i++)
            {
                double t = (double)i / n, u = 1 - t;
                points.Add(new((int)(u*u*u*start.X + 3*u*u*t*c1.X + 3*u*t*t*c2.X + t*t*t*end.X),
                    (int)(u*u*u*start.Y + 3*u*u*t*c1.Y + 3*u*t*t*c2.Y + t*t*t*end.Y)));
            }
        }
        void L(Point end, int n)
        {
            var start = points[^1];
            for (int i = 0; i <= n; i++)
                points.Add(new(start.X + (end.X-start.X)*i/n, start.Y + (end.Y-start.Y)*i/n));
        }
        B(new(x+10*s,baseY-40*s),new(x+18*s,topY+10*s),new(x+26*s,topY+5*s),25);
        B(new(x+34*s,topY+8*s),new(x+42*s,midY),new(x+50*s,baseY-8*s),25);
        B(new(x+44*s,midY+5*s),new(x+28*s,midY-5*s),new(x+52*s,midY),20);
        x += 288;
        B(new(x-10*s,midY-10*s),new(x+5*s,topY+5*s),new(x+35*s,topY+8*s),20);
        B(new(x+50*s,topY+5*s),new(x+40*s,midY+10*s),new(x+8*s,baseY-5*s),25);
        B(new(x+20*s,baseY),new(x+45*s,baseY-2*s),new(x+58*s,baseY-8*s),20);
        x += 288;
        B(new(x-5*s,baseY-20*s),new(x+2*s,midY),new(x+5*s,topY+8*s),20);
        var kTop = new Point(x+8*s,topY+8*s);
        var kMid = new Point(x+8*s,midY+5*s);
        var kBottom = new Point(x+8*s,baseY-5*s);
        var kUpper = new Point(x+48*s,topY+15*s);
        var kLower = new Point(x+50*s,baseY-8*s);
        L(kTop,12); L(kBottom,28); L(kMid,14); L(kUpper,20); L(kMid,20); L(kLower,22);
        x += 72*s;
        B(new(x-12*s,baseY-8*s),new(x+2*s,midY+8*s),new(x+8*s,midY-6*s),22);
        B(new(x+10*s,midY+12*s),new(x+10*s,baseY-15*s),new(x+14*s,baseY-6*s),22);
        int size = 14*s;
        var center = new Point(x+14*s,topY+22*s);
        B(new(x+20*s,baseY-32*s),new(center.X+6*s,topY+42*s),new(center.X,center.Y+size),24);
        for (int i = 0; i <= 64; i++)
        {
            double t = Math.PI - 2*Math.PI*i/64;
            double hx = 16*Math.Pow(Math.Sin(t),3);
            double hy = 13*Math.Cos(t)-5*Math.Cos(2*t)-2*Math.Cos(3*t)-Math.Cos(4*t);
            points.Add(new(center.X+(int)(hx*size/18),center.Y-(int)(hy*size/18)));
        }
        B(new(x+26*s,topY+48*s),new(x+36*s,midY-2*s),new(x+56*s,midY-6*s),24);
        return points;
    }
    public static List<Point> Center(List<Point> path, Rectangle bounds)
    {
        int minX = path.Min(p => p.X), minY = path.Min(p => p.Y);
        int offsetX = (bounds.Width - (path.Max(p => p.X)-minX))/2-minX;
        int offsetY = (bounds.Height - (path.Max(p => p.Y)-minY))/2-minY;
        return path.Select(p => new Point(p.X+offsetX,p.Y+offsetY)).ToList();
    }
}
