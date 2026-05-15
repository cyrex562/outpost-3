using OutpostGame.Core.Colony;
using OutpostGame.Core.World;

namespace OutpostGame.Core.Simulation;

/// <summary>
/// Phase-2 orchestrator that ties the headless core to the rendered scene.
/// Owns the <see cref="ColonyState"/> and a <see cref="ColonyTurnProcessor"/>,
/// subscribes the processor to the turn manager, and exposes the construction
/// and turn-control API the UI calls into.
/// </summary>
public sealed class ColonySession
{
    public ColonyState State { get; }
    public SiteDefinition Site { get; }
    private readonly ColonyTurnProcessor _processor;

    /// <summary>Forwarded from <see cref="TurnManager.TurnAdvanced"/>.</summary>
    public event Action<int>? TurnAdvanced;

    /// <summary>Fires whenever something that could affect the rendered grid or HUD changed
    /// (turn processed, construction queued, …). The UI subscribes to redraw.</summary>
    public event Action? StateChanged;

    public int CurrentSol => State.TurnManager.CurrentSol;

    public ColonySession(SiteDefinition site)
    {
        Site = site;
        State = new ColonyState(site.Size.Width, site.Size.Height);
        _processor = new ColonyTurnProcessor(State);

        State.TurnManager.TurnAdvanced += sol =>
        {
            _processor.ProcessTurn(sol);
            TurnAdvanced?.Invoke(sol);
            StateChanged?.Invoke();
        };

        BuildableBuildings = new[] { "solar_array_mk1", "basic_habitat", "iron_mine" }
            .Select(BuildingRegistry.Get)
            .ToList();

        SeedDefaults();
    }

    /// <summary>The set of buildings the Phase-2 placer offers. Backed by
    /// <see cref="BuildingRegistry"/>; ordered to match the 1/2/3 keyboard hotkeys.</summary>
    public IReadOnlyList<BuildingDefinition> BuildableBuildings { get; }

    public void EndTurn(int sols = 1)
    {
        if (sols < 1) return;
        State.TurnManager.Advance(sols);
    }

    public ConstructionResult QueueConstruction(string buildingId, GridPosition origin)
    {
        if (!BuildingRegistry.TryGet(buildingId, out var maybeDef) || maybeDef is null)
            return ConstructionResult.Fail($"Unknown building: {buildingId}");
        var def = maybeDef;

        var validation = State.Grid.Validate(origin, def.Size);
        if (!validation.Success)
            return ConstructionResult.Fail(validation.FailureReason ?? "Invalid placement");

        var cost = def.ConstructionCost.ToDictionary(kv => kv.Key, kv => (float)kv.Value);
        if (!State.Resources.HasEnough(cost))
            return ConstructionResult.Fail($"Insufficient resources for {def.DisplayName}");

        State.Resources.TryConsume(cost);

        var slot = new BuildingSlot
        {
            Origin = origin,
            Size = def.Size,
            BuildingDefinitionId = buildingId,
            State = BuildingState.UnderConstruction,
            ConstructionTurnsRemaining = def.ConstructionTurns,
        };
        var place = State.Grid.Place(slot);
        if (!place.Success)
        {
            // Refund — only happens if validation/placement diverge, but stay honest.
            foreach (var kv in cost) State.Resources.Add(kv.Key, kv.Value);
            return ConstructionResult.Fail(place.FailureReason ?? "Placement failed");
        }

        State.EventLog.Add(new ColonyEvent(
            ColonyEventSeverity.Info,
            $"Construction started: {def.DisplayName} at ({origin.X}, {origin.Y})",
            CurrentSol));

        StateChanged?.Invoke();
        return ConstructionResult.Ok(slot);
    }

    /// <summary>Bootstraps the colony with starting resources, population and labour
    /// so the player can place buildings and see things happen in Phase 2.</summary>
    private void SeedDefaults()
    {
        // Storage caps — generous enough that production fills them gradually.
        State.Resources.SetCap("steel", 1000f);
        State.Resources.SetCap("electronics", 500f);
        State.Resources.SetCap("components", 200f);
        State.Resources.SetCap("nutrients", 200f);
        State.Resources.SetCap("water", 200f);
        State.Resources.SetCap("oxygen", 200f);
        State.Resources.SetCap("ice", 500f);
        State.Resources.SetCap("iron_ore", 500f);
        State.Resources.SetCap("food", 200f);
        State.Resources.SetCap("uranium_fuel", 50f);

        // Construction stockpile.
        State.Resources.Add("steel", 800f);
        State.Resources.Add("electronics", 300f);
        // Survival buffer.
        State.Resources.Add("nutrients", 100f);
        State.Resources.Add("water", 100f);
        State.Resources.Add("oxygen", 100f);
        State.Resources.Add("ice", 200f);

        State.Population.Count = 20;
        State.Labor.TotalWorkers = 15;
    }
}

public sealed class ConstructionResult
{
    public bool Success { get; init; }
    public string? FailureReason { get; init; }
    public BuildingSlot? Slot { get; init; }
    public static ConstructionResult Ok(BuildingSlot slot) => new() { Success = true, Slot = slot };
    public static ConstructionResult Fail(string reason) => new() { Success = false, FailureReason = reason };
}
