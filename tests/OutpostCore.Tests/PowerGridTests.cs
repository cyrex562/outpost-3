namespace OutpostGame.Tests;
using OutpostGame.Core.Colony;

[TestFixture]
public class PowerGridTests
{
    [Test]
    public void NoBrownout_WhenCapacityMeetsConsumption()
    {
        var grid = new PowerGrid();
        grid.RegisterProducer(Guid.NewGuid(), 100f);
        var consumer = Guid.NewGuid();
        grid.RegisterConsumer(consumer, 80f);
        Assert.That(grid.BrownoutLevel, Is.EqualTo(0f));
        Assert.That(grid.IsPowered(consumer), Is.True);
    }

    [Test]
    public void Brownout_WhenDeficit()
    {
        var grid = new PowerGrid();
        grid.RegisterProducer(Guid.NewGuid(), 50f);
        var consumer = Guid.NewGuid();
        grid.RegisterConsumer(consumer, 100f);
        Assert.That(grid.BrownoutLevel, Is.GreaterThan(0f));
    }

    [Test]
    public void Essential_AlwaysPowered_DuringBrownout()
    {
        var grid = new PowerGrid();
        grid.RegisterProducer(Guid.NewGuid(), 30f);
        var essential = Guid.NewGuid();
        grid.RegisterConsumer(essential, 50f);
        grid.SetEssential(essential, true);
        Assert.That(grid.IsPowered(essential), Is.True);
    }

    [Test]
    public void ManualDisable_OverridesEverything()
    {
        var grid = new PowerGrid();
        grid.RegisterProducer(Guid.NewGuid(), 200f);
        var building = Guid.NewGuid();
        grid.RegisterConsumer(building, 10f);
        grid.TogglePower(building);
        Assert.That(grid.IsPowered(building), Is.False);
    }
}
