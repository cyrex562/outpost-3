using System;
using System.Collections.Generic;
using HarshRealm.Data;

namespace HarshRealm.Services;

public sealed class GameStateService
{
    private readonly DateTime _startDate = new(2070, 1, 1, 0, 0, 0, DateTimeKind.Utc);

    public SimulationService Simulation { get; } = new();
    public SolarSystemService SolarSystem { get; }

    public GameStateService()
    {
        SolarSystem = new SolarSystemService(new DateTime(2070, 1, 1, 0, 0, 0, DateTimeKind.Utc));
    }

    public bool TryLoadSolarSystem(string resourcePath)
    {
        return SolarSystem.LoadFromCsv(resourcePath);
    }

    public IReadOnlyList<OrbitalChange> ProcessTurn()
    {
        var changes = SolarSystem.UpdateAllPositions(30.0);
        Simulation.AdvanceTurn();
        return changes;
    }

    public string FormattedDate => SolarSystem.FormattedDate;
}


