namespace OutpostGame.Tests;
using OutpostGame.Core.Colony;

[TestFixture]
public class ColonyGridTests
{
    [Test]
    public void Place_1x1_Succeeds_OnEmptyGrid()
    {
        var grid = new ColonyGrid(10, 10);
        var slot = new BuildingSlot { Origin = new(0,0), Size = new(1,1), BuildingDefinitionId = "solar_array" };
        Assert.That(grid.Place(slot).Success, Is.True);
    }

    [Test]
    public void Place_2x3_Occupies_AllCells()
    {
        var grid = new ColonyGrid(10, 10);
        var slot = new BuildingSlot { Origin = new(1,1), Size = new(2,3), BuildingDefinitionId = "smelter" };
        grid.Place(slot);
        for (int x = 1; x <= 2; x++)
        for (int y = 1; y <= 3; y++)
            Assert.That(grid.IsCellOccupied(new(x,y)), Is.True);
    }

    [Test]
    public void Place_Overlapping_Fails()
    {
        var grid = new ColonyGrid(10, 10);
        var s1 = new BuildingSlot { Origin = new(0,0), Size = new(2,2), BuildingDefinitionId = "solar_array" };
        var s2 = new BuildingSlot { Origin = new(1,1), Size = new(2,2), BuildingDefinitionId = "solar_array" };
        grid.Place(s1);
        Assert.That(grid.Place(s2).Success, Is.False);
    }

    [Test]
    public void Place_OutOfBounds_Fails()
    {
        var grid = new ColonyGrid(5, 5);
        var slot = new BuildingSlot { Origin = new(4,4), Size = new(2,2), BuildingDefinitionId = "solar_array" };
        Assert.That(grid.Place(slot).Success, Is.False);
    }

    [Test]
    public void Remove_FreesOccupancy()
    {
        var grid = new ColonyGrid(10, 10);
        var slot = new BuildingSlot { Origin = new(0,0), Size = new(2,2), BuildingDefinitionId = "solar_array" };
        grid.Place(slot);
        grid.Remove(slot.Id);
        Assert.That(grid.IsCellOccupied(new(0,0)), Is.False);
    }
}
