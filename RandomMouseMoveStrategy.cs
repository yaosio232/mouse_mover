namespace MouseMoverApp;

public sealed class RandomMouseMoveStrategy : IMouseMoveStrategy
{
    public async Task ExecuteAsync(MouseMoveStrategyContext context, CancellationToken token)
    {
        for (int i = 0; i < 10; i++)
        {
            await context.MoveByDeltaAsync(context.Random.Next(-450,451),context.Random.Next(-450,451),50,4,token).ConfigureAwait(false);
            await Task.Delay(context.Random.Next(200,1801),token).ConfigureAwait(false);
        }
    }
}
