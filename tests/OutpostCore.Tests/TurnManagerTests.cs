namespace OutpostGame.Tests;
using OutpostGame.Core.Simulation;

[TestFixture]
public class TurnManagerTests
{
    [Test]
    public void Advance_IncrementsSol()
    {
        var tm = new TurnManager();
        tm.Advance(10);
        Assert.That(tm.CurrentSol, Is.EqualTo(10));
    }

    [Test]
    public void StrategicTurn_FiresEvery30Sols()
    {
        var tm = new TurnManager();
        int strategicFires = 0;
        tm.StrategicTurnAdvanced += _ => strategicFires++;
        tm.Advance(90);
        Assert.That(strategicFires, Is.EqualTo(3));
    }

    [Test]
    public void WaitDirective_DoesNotAdvancePastTarget()
    {
        var tm = new TurnManager();
        tm.BeginWait(new WaitDirective { WaitUntilSol = 50, InterruptOnEvent = false });
        // Simulate a wait loop
        while (tm.ShouldContinueWait()) tm.Advance(1);
        Assert.That(tm.CurrentSol, Is.EqualTo(50));
    }
}
