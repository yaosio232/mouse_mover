namespace MouseMoverApp;

public interface IMouseMoveStrategy
{
    Task ExecuteAsync(MouseMoveStrategyContext context, CancellationToken token);
}
