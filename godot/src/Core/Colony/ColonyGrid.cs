namespace OutpostGame.Core.Colony;

public sealed class PlacementResult
{
    public bool Success { get; init; }
    public string? FailureReason { get; init; }
    public static PlacementResult Ok() => new() { Success = true };
    public static PlacementResult Fail(string reason) => new() { Success = false, FailureReason = reason };
}

public sealed class BuildingSlot
{
    public Guid Id { get; init; } = Guid.NewGuid();
    public GridPosition Origin { get; init; }      // Top-left cell
    public GridSize Size { get; init; }
    public string BuildingDefinitionId { get; set; } = "";
    public BuildingState State { get; set; } = BuildingState.Planned;
    public int ConstructionTurnsRemaining { get; set; }
    public int AssignedWorkers { get; set; }
    public int ProductionCycleProgress { get; set; }

    // Phase 3C — fleet + operator allocation tracking. Held during
    // UnderConstruction, released on completion / cancel / damage. Progress
    // ticks only while both pools are allocated (hard stop on starvation).
    public int AllocatedFleetSlots { get; set; }
    public int AllocatedOperators { get; set; }

    /// <summary>Sol at which fleet+operators were first allocated to this slot.
    /// Used to order FIFO allocation deterministically across save/load. -1
    /// means never allocated.</summary>
    public int AllocatedAtSol { get; set; } = -1;
}

public readonly record struct GridPosition(int X, int Y);
public readonly record struct GridSize(int Width, int Height);

public sealed class ColonyGrid
{
    private readonly int _width;
    private readonly int _height;
    private readonly Dictionary<GridPosition, Guid> _occupancy = new();
    private readonly Dictionary<Guid, BuildingSlot> _slots = new();

    public int Width  => _width;
    public int Height => _height;

    public ColonyGrid(int width, int height)
    {
        _width = width;
        _height = height;
    }

    public PlacementResult Validate(GridPosition origin, GridSize size)
    {
        for (int x = origin.X; x < origin.X + size.Width; x++)
        for (int y = origin.Y; y < origin.Y + size.Height; y++)
        {
            var cell = new GridPosition(x, y);
            if (x < 0 || x >= _width || y < 0 || y >= _height)
                return PlacementResult.Fail("Out of grid bounds");
            if (_occupancy.ContainsKey(cell))
                return PlacementResult.Fail("Cell already occupied");
        }
        return PlacementResult.Ok();
    }

    public PlacementResult Place(BuildingSlot slot)
    {
        var result = Validate(slot.Origin, slot.Size);
        if (!result.Success) return result;

        _slots[slot.Id] = slot;
        for (int x = slot.Origin.X; x < slot.Origin.X + slot.Size.Width; x++)
        for (int y = slot.Origin.Y; y < slot.Origin.Y + slot.Size.Height; y++)
            _occupancy[new GridPosition(x, y)] = slot.Id;

        return PlacementResult.Ok();
    }

    public void Remove(Guid slotId)
    {
        if (!_slots.TryGetValue(slotId, out var slot)) return;
        for (int x = slot.Origin.X; x < slot.Origin.X + slot.Size.Width; x++)
        for (int y = slot.Origin.Y; y < slot.Origin.Y + slot.Size.Height; y++)
            _occupancy.Remove(new GridPosition(x, y));
        _slots.Remove(slotId);
    }

    public BuildingSlot? GetSlot(Guid id) => _slots.GetValueOrDefault(id);
    public IEnumerable<BuildingSlot> AllSlots => _slots.Values;
    public bool IsCellOccupied(GridPosition pos) => _occupancy.ContainsKey(pos);

    /// <summary>Returns the slot whose footprint contains <paramref name="pos"/>,
    /// or null when the cell is empty.</summary>
    public BuildingSlot? GetSlotAtCell(GridPosition pos) =>
        _occupancy.TryGetValue(pos, out var id) && _slots.TryGetValue(id, out var slot)
            ? slot : null;

    /// <summary>Removes all slots — used when restoring a save.</summary>
    public void Clear()
    {
        _slots.Clear();
        _occupancy.Clear();
    }
}
