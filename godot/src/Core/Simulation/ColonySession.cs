using System.Text.Json;
using System.Text.Json.Serialization;
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

    private static readonly JsonSerializerOptions _jsonOpts = new()
    {
        WriteIndented = true,
        Converters = { new JsonStringEnumConverter() },
    };

    /// <summary>Forwarded from <see cref="TurnManager.TurnAdvanced"/>.</summary>
    public event Action<int>? TurnAdvanced;

    /// <summary>Fires whenever something that could affect the rendered grid or HUD changed
    /// (turn processed, construction queued, …). The UI subscribes to redraw.</summary>
    public event Action? StateChanged;

    /// <summary>Fires once when the colony reaches a terminal or milestone status.</summary>
    public event Action<ColonyStatus>? ColonyStatusChanged;

    public int CurrentSol => State.TurnManager.CurrentSol;
    public ColonyStatus Status => State.Status;

    private bool _suppressStateChanged;

    public ColonySession(SiteDefinition site)
    {
        Site = site;
        State = new ColonyState(site.Size.Width, site.Size.Height);
        _processor = new ColonyTurnProcessor(State, site.Seed, site.Difficulty);

        State.TurnManager.TurnAdvanced += sol =>
        {
            var prevStatus = State.Status;
            _processor.ProcessTurn(sol);
            TurnAdvanced?.Invoke(sol);
            if (!_suppressStateChanged)
                StateChanged?.Invoke();
            if (State.Status != prevStatus)
                ColonyStatusChanged?.Invoke(State.Status);
        };

        // Ordered so the player encounters simpler buildings first.
        BuildableBuildings = new[]
        {
            "solar_array_mk1", "nuclear_reactor",
            "iron_mine", "ice_drill", "nutrient_recycler",
            "smelter", "fabricator",
            "vehicle_factory", "prefab_factory",
            "water_purifier", "life_support_system", "hydroponics_bay",
            "basic_habitat", "warehouse", "large_warehouse",
        }
            .Select(BuildingRegistry.Get)
            .ToList();

        SeedDefaults();
    }

    /// <summary>The set of buildings the Phase-2 placer offers. Backed by
    /// <see cref="BuildingRegistry"/>; ordered to match the 1/2/3 keyboard hotkeys.</summary>
    public IReadOnlyList<BuildingDefinition> BuildableBuildings { get; }

    /// <summary>
    /// Advance <paramref name="sols"/> turns. When <paramref name="fast"/> is true,
    /// per-tick <see cref="StateChanged"/> notifications are suppressed and a single
    /// event fires after the bulk advance — letting the HUD/grid skip intermediate
    /// redraws during long skips.
    /// </summary>
    public void EndTurn(int sols = 1, bool fast = false)
    {
        if (sols < 1) return;
        if (!fast)
        {
            State.TurnManager.Advance(sols);
            return;
        }

        _suppressStateChanged = true;
        try
        {
            State.TurnManager.Advance(sols);
        }
        finally
        {
            _suppressStateChanged = false;
        }
        StateChanged?.Invoke();
    }

    /// <summary>
    /// Distribute all available workers across operational buildings, preferring to
    /// assign workers with the matching <see cref="BuildingDefinition.PrimarySkill"/>
    /// to buildings that need them.
    /// </summary>
    public void AutoAssignLabor()
    {
        var operational = State.Grid.AllSlots
            .Where(s => s.State == BuildingState.Operational)
            .ToList();

        // Clear existing assignments so we start fresh.
        foreach (var slot in operational)
            State.Labor.Deallocate(slot.Id);

        // Availability buckets: how many workers of each skill type remain unassigned.
        var available = new Dictionary<SkillType, int>(State.Population.Skills);
        foreach (SkillType sk in Enum.GetValues<SkillType>())
            if (!available.ContainsKey(sk)) available[sk] = 0;

        int totalIdle = State.Labor.TotalWorkers;

        foreach (var slot in operational)
        {
            if (totalIdle <= 0) break;
            var def = BuildingRegistry.Get(slot.BuildingDefinitionId);
            if (def.LaborRequired == 0) continue;

            int need = Math.Min(def.LaborRequired, totalIdle);

            // Prefer the building's primary skill, fall back to Laborer, then any.
            SkillType chosen = def.PrimarySkill;
            int fromPrimary  = Math.Min(need, available.GetValueOrDefault(def.PrimarySkill, 0));
            int fromLaborer  = 0;
            if (fromPrimary < need && def.PrimarySkill != SkillType.Laborer)
                fromLaborer = Math.Min(need - fromPrimary,
                                       available.GetValueOrDefault(SkillType.Laborer, 0));

            // Record the dominant skill type for the efficiency calculation.
            chosen = fromPrimary >= fromLaborer ? def.PrimarySkill : SkillType.Laborer;

            // Deduct from skill buckets.
            int deductPrimary = Math.Min(fromPrimary, need);
            available[def.PrimarySkill] =
                Math.Max(0, available.GetValueOrDefault(def.PrimarySkill, 0) - deductPrimary);
            int remaining = need - deductPrimary;
            if (remaining > 0 && def.PrimarySkill != SkillType.Laborer)
            {
                int deductLab = Math.Min(remaining,
                                         available.GetValueOrDefault(SkillType.Laborer, 0));
                available[SkillType.Laborer] =
                    Math.Max(0, available.GetValueOrDefault(SkillType.Laborer, 0) - deductLab);
                remaining -= deductLab;
            }
            // Remaining workers come from any other skill bucket.
            if (remaining > 0)
            {
                foreach (var sk in available.Keys.ToList())
                {
                    if (remaining <= 0) break;
                    int take = Math.Min(remaining, available[sk]);
                    available[sk] -= take;
                    remaining     -= take;
                }
            }

            State.Labor.AssignWithSkill(slot.Id, need, chosen);
            totalIdle -= need;
        }

        StateChanged?.Invoke();
    }

    /// <summary>Assign a specific number of workers to one building slot.
    /// Returns false if <paramref name="count"/> exceeds idle + currently-assigned workers.</summary>
    public bool AssignLaborToBuilding(Guid slotId, int count)
    {
        bool ok = State.Labor.Assign(slotId, count);
        if (ok) StateChanged?.Invoke();
        return ok;
    }

    /// <summary>Remove all workers from a building slot.</summary>
    public void DeallocateLabor(Guid slotId)
    {
        State.Labor.Deallocate(slotId);
        StateChanged?.Invoke();
    }

    /// <summary>
    /// Repairs a <see cref="BuildingState.Damaged"/> slot, consuming 30 % of its
    /// original construction cost.  Returns false if the slot is not damaged or
    /// resources are insufficient.
    /// </summary>
    public bool RepairBuilding(Guid slotId)
    {
        var slot = State.Grid.GetSlot(slotId);
        if (slot is null || slot.State != BuildingState.Damaged) return false;

        var def = BuildingRegistry.Get(slot.BuildingDefinitionId);
        var cost = def.ConstructionCost.ToDictionary(
            kv => kv.Key, kv => (float)kv.Value * 0.3f);

        if (!State.Resources.HasEnough(cost)) return false;
        State.Resources.TryConsume(cost);

        slot.State = BuildingState.Operational;
        State.Power.RegisterConsumer(slot.Id, def.PowerConsumption);
        if (def.PowerProduction > 0)
            State.Power.RegisterProducer(slot.Id, def.PowerProduction);
        if (def.IsEssential)
            State.Power.SetEssential(slot.Id, true);

        State.EventLog.Add(new ColonyEvent(
            ColonyEventSeverity.Info,
            $"{def.DisplayName} repaired and back online.",
            CurrentSol));

        StateChanged?.Invoke();
        return true;
    }

    /// <summary>Peek the next decision the player must resolve, if any.</summary>
    public PendingDecision? CurrentDecision => State.CurrentDecision;

    /// <summary>True when at least one decision is awaiting a player response.</summary>
    public bool HasPendingDecision => State.PendingDecisions.Count > 0;

    /// <summary>
    /// Resolve the current pending decision by selecting one of its choices.
    /// Returns false if there is no pending decision or the index is out of range.
    /// </summary>
    public bool AnswerDecision(int choiceIndex)
    {
        if (State.PendingDecisions.Count == 0) return false;
        var current = State.PendingDecisions.Peek();
        if (choiceIndex < 0 || choiceIndex >= current.Choices.Count) return false;

        State.PendingDecisions.Dequeue();
        var choice = current.Choices[choiceIndex];
        choice.Apply?.Invoke(State);

        State.EventLog.Add(new ColonyEvent(
            ColonyEventSeverity.Info,
            $"Decision — {current.Title}: {choice.Label}",
            CurrentSol));

        StateChanged?.Invoke();
        return true;
    }

    /// <summary>
    /// Begins an upgrade of an <see cref="BuildingState.Operational"/> slot to its
    /// <see cref="BuildingDefinition.UpgradesTo"/> target. The slot transitions back
    /// to <see cref="BuildingState.UnderConstruction"/> for the duration of the
    /// upgrade and is unregistered from the power grid until completion.
    /// Requires the target definition to share the same footprint as the source.
    /// </summary>
    public UpgradeResult UpgradeBuilding(Guid slotId)
    {
        var slot = State.Grid.GetSlot(slotId);
        if (slot is null) return UpgradeResult.Fail("Building not found");
        if (slot.State != BuildingState.Operational)
            return UpgradeResult.Fail("Only operational buildings can be upgraded");

        var currentDef = BuildingRegistry.Get(slot.BuildingDefinitionId);
        if (currentDef.UpgradesTo is null)
            return UpgradeResult.Fail($"{currentDef.DisplayName} has no upgrade path");

        if (!BuildingRegistry.TryGet(currentDef.UpgradesTo, out var nextDef) || nextDef is null)
            return UpgradeResult.Fail($"Unknown upgrade target: {currentDef.UpgradesTo}");

        if (nextDef.Size.Width  != currentDef.Size.Width
         || nextDef.Size.Height != currentDef.Size.Height)
            return UpgradeResult.Fail(
                $"Upgrade footprint mismatch: {currentDef.Size.Width}x{currentDef.Size.Height}"
                + $" → {nextDef.Size.Width}x{nextDef.Size.Height}");

        var cost = nextDef.ConstructionCost.ToDictionary(kv => kv.Key, kv => (float)kv.Value);
        if (!State.Resources.HasEnough(cost))
            return UpgradeResult.Fail($"Insufficient resources to upgrade to {nextDef.DisplayName}");

        State.Resources.TryConsume(cost);

        slot.BuildingDefinitionId       = nextDef.Id;
        slot.State                      = BuildingState.UnderConstruction;
        slot.ConstructionTurnsRemaining = nextDef.ConstructionTurns;
        slot.ProductionCycleProgress    = 0;
        // Reset allocation tracking so AdvanceConstruction reallocates from the
        // pool for the upgrade work.
        slot.AllocatedFleetSlots        = 0;
        slot.AllocatedOperators         = 0;
        slot.AllocatedAtSol             = -1;

        // Drop from the power grid for the duration; AdvanceConstruction re-registers
        // on completion using the new definition.
        State.Power.Unregister(slot.Id);

        State.EventLog.Add(new ColonyEvent(
            ColonyEventSeverity.Info,
            $"Upgrade started: {currentDef.DisplayName} → {nextDef.DisplayName}",
            CurrentSol));

        StateChanged?.Invoke();
        return UpgradeResult.Ok();
    }

    /// <summary>
    /// Cancels an <see cref="BuildingState.UnderConstruction"/> slot, removes it
    /// from the grid, and refunds <paramref name="refundPct"/> of its construction
    /// cost (default 50 %). Returns false if the slot does not exist or is no
    /// longer cancellable.
    /// </summary>
    public bool CancelConstruction(Guid slotId, float refundPct = 0.5f)
    {
        var slot = State.Grid.GetSlot(slotId);
        if (slot is null || slot.State != BuildingState.UnderConstruction) return false;

        var def = BuildingRegistry.Get(slot.BuildingDefinitionId);
        foreach (var kv in def.ConstructionCost)
            State.Resources.Add(kv.Key, kv.Value * refundPct);

        // Release any allocated fleet / operators back to their pools so the
        // next queued project can pick them up.
        if (slot.AllocatedFleetSlots > 0) State.Fleet.Release(slot.AllocatedFleetSlots);
        if (slot.AllocatedOperators  > 0) State.Operators.Release(slot.AllocatedOperators);

        State.Grid.Remove(slotId);
        State.Labor.Deallocate(slotId);

        State.EventLog.Add(new ColonyEvent(
            ColonyEventSeverity.Info,
            $"Construction cancelled: {def.DisplayName} ({(int)(refundPct * 100)}% refund).",
            CurrentSol));

        StateChanged?.Invoke();
        return true;
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

    // ── Save / Load ──────────────────────────────────────────────────────────

    /// <summary>Serialises the current colony state to a JSON string.</summary>
    public string CreateSave()
    {
        var s = State;
        var save = new ColonySaveData
        {
            ColonyName              = s.Name,
            CurrentSol              = s.TurnManager.CurrentSol,
            Status                  = s.Status,
            DustStormTurnsRemaining = s.DustStormTurnsRemaining,
            LowMoraleTurns          = s.LowMoraleTurns,
            ThrivingTurns           = s.ThrivingTurns,
            GrowthAccumulator       = _processor.GrowthAccumulator,
            Population = new PopulationSaveData
            {
                Count             = s.Population.Count,
                Health            = s.Population.Health,
                Morale            = s.Population.Morale,
                NeedsDeficitTurns = s.Population.NeedsDeficitTurns,
                Skills            = s.Population.Skills
                    .ToDictionary(kv => kv.Key.ToString(), kv => kv.Value),
            },
            Labor = new LaborSaveData
            {
                TotalWorkers = s.Labor.TotalWorkers,
                Allocations  = s.Labor.AllAllocations
                    .ToDictionary(kv => kv.Key.ToString(), kv => kv.Value),
                SkillSlots   = s.Labor.AllSkillSlots
                    .ToDictionary(kv => kv.Key.ToString(), kv => kv.Value.ToString()),
            },
            Resources = new ResourceSaveData
            {
                Amounts = new Dictionary<string, float>(s.Resources.Snapshot()),
                Caps    = new Dictionary<string, float>(s.Resources.CapSnapshot()),
            },
            Buildings = s.Grid.AllSlots.Select(slot => new BuildingSlotSaveData
            {
                Id                        = slot.Id.ToString(),
                DefinitionId              = slot.BuildingDefinitionId,
                OriginX                   = slot.Origin.X,
                OriginY                   = slot.Origin.Y,
                SizeWidth                 = slot.Size.Width,
                SizeHeight                = slot.Size.Height,
                State                     = slot.State,
                ConstructionTurnsRemaining= slot.ConstructionTurnsRemaining,
                AssignedWorkers           = slot.AssignedWorkers,
                ProductionCycleProgress   = slot.ProductionCycleProgress,
                AllocatedFleetSlots       = slot.AllocatedFleetSlots,
                AllocatedOperators        = slot.AllocatedOperators,
                AllocatedAtSol            = slot.AllocatedAtSol,
            }).ToList(),
            Events = s.EventLog.All.Select(e => new EventSaveData
            {
                Severity = e.Severity,
                Message  = e.Message,
                Sol      = e.Sol,
            }).ToList(),
            Fleet = new ConstructionFleetSaveData
            {
                BaseSlots    = s.Fleet.BaseSlots,
                FactorySlots = s.Fleet.FactorySlots,
                TechBonus    = s.Fleet.TechBonus,
                UsedSlots    = s.Fleet.UsedSlots,
            },
            Operators = new OperatorPoolSaveData
            {
                People    = s.Operators.People,
                Robots    = s.Operators.Robots,
                Allocated = s.Operators.Allocated,
            },
        };
        return JsonSerializer.Serialize(save, _jsonOpts);
    }

    /// <summary>Restores colony state in-place from a JSON string produced by
    /// <see cref="CreateSave"/>.  Existing events, buildings, and resources are replaced.</summary>
    public void ApplySave(string json)
    {
        var save = JsonSerializer.Deserialize<ColonySaveData>(json, _jsonOpts)
            ?? throw new InvalidOperationException("Failed to deserialise save data.");

        var s = State;

        // Sol counter (no events fired).
        s.TurnManager.RestoreSol(save.CurrentSol);

        // Status fields.
        s.Name                    = save.ColonyName;
        s.Status                  = save.Status;
        s.DustStormTurnsRemaining = save.DustStormTurnsRemaining;
        s.LowMoraleTurns          = save.LowMoraleTurns;
        s.ThrivingTurns           = save.ThrivingTurns;
        _processor.GrowthAccumulator = save.GrowthAccumulator;

        // Population.
        s.Population.Count             = save.Population.Count;
        s.Population.Health            = save.Population.Health;
        s.Population.Morale            = save.Population.Morale;
        s.Population.NeedsDeficitTurns = save.Population.NeedsDeficitTurns;
        s.Population.Skills.Clear();
        foreach (var kv in save.Population.Skills)
            if (Enum.TryParse<SkillType>(kv.Key, out var sk))
                s.Population.Skills[sk] = kv.Value;

        // Labor.
        s.Labor.TotalWorkers = save.Labor.TotalWorkers;
        // (Allocations restored after buildings, once slot IDs are in place.)

        // Resources.
        s.Resources.RestoreFromSnapshot(save.Resources.Amounts, save.Resources.Caps);

        // Buildings — clear grid and power grid, then replay slots.
        s.Grid.Clear();
        s.Power.UnregisterAll();

        foreach (var bd in save.Buildings)
        {
            var slot = new BuildingSlot
            {
                Id                         = Guid.Parse(bd.Id),
                BuildingDefinitionId       = bd.DefinitionId,
                Origin                     = new GridPosition(bd.OriginX, bd.OriginY),
                Size                       = new GridSize(bd.SizeWidth, bd.SizeHeight),
                State                      = bd.State,
                ConstructionTurnsRemaining = bd.ConstructionTurnsRemaining,
                AssignedWorkers            = bd.AssignedWorkers,
                ProductionCycleProgress    = bd.ProductionCycleProgress,
                AllocatedFleetSlots        = bd.AllocatedFleetSlots,
                AllocatedOperators         = bd.AllocatedOperators,
                AllocatedAtSol             = bd.AllocatedAtSol,
            };
            s.Grid.Place(slot);

            // Re-register with the power grid for operational buildings.
            if (slot.State == BuildingState.Operational)
            {
                var def = BuildingRegistry.Get(slot.BuildingDefinitionId);
                s.Power.RegisterConsumer(slot.Id, def.PowerConsumption);
                if (def.PowerProduction > 0)
                    s.Power.RegisterProducer(slot.Id, def.PowerProduction);
                if (def.IsEssential)
                    s.Power.SetEssential(slot.Id, true);
            }
        }

        // Restore labor allocations and skill slot assignments with now-valid slot IDs.
        s.Labor.RestoreAllocations(
            save.Labor.Allocations.ToDictionary(kv => Guid.Parse(kv.Key), kv => kv.Value));
        s.Labor.RestoreSkillSlots(
            save.Labor.SkillSlots
                .Where(kv => Enum.TryParse<SkillType>(kv.Value, out _))
                .ToDictionary(kv => Guid.Parse(kv.Key),
                              kv => Enum.Parse<SkillType>(kv.Value)));

        // Fleet + operator pools. Pre-3C saves don't have these — populate
        // from the SeedDefaults values so the colony still has working pools.
        if (save.Fleet is not null)
            s.Fleet.RestoreFromSave(save.Fleet.BaseSlots, save.Fleet.FactorySlots,
                                    save.Fleet.TechBonus, save.Fleet.UsedSlots);
        else
            s.Fleet.RestoreFromSave(10, 0, 0, 0);

        if (save.Operators is not null)
            s.Operators.RestoreFromSave(save.Operators.People, save.Operators.Robots, save.Operators.Allocated);
        else
            s.Operators.RestoreFromSave(10, 5, 0);

        // Event log.
        s.EventLog.Clear();
        foreach (var e in save.Events)
            s.EventLog.Add(new ColonyEvent(e.Severity, e.Message, e.Sol));

        StateChanged?.Invoke();
    }

    /// <summary>Bootstraps the colony with starting resources, population, labour,
    /// fleet/operator pools and the pre-placed starter buildings (Lander +
    /// Multi-Purpose Habitat) so the player has a working base from sol 0.</summary>
    private void SeedDefaults()
    {
        float resMult = DifficultySettings.ResourceMultiplier(Site.Difficulty);

        // Storage caps — generous enough that production fills them gradually.
        State.Resources.SetCap("steel",            1000f);
        State.Resources.SetCap("electronics",       500f);
        State.Resources.SetCap("components",        200f);
        State.Resources.SetCap("prefab_components", 500f);
        State.Resources.SetCap("nutrients",         200f);
        State.Resources.SetCap("water",             200f);
        State.Resources.SetCap("oxygen",            200f);
        State.Resources.SetCap("ice",               500f);
        State.Resources.SetCap("iron_ore",          500f);
        State.Resources.SetCap("food",              200f);
        State.Resources.SetCap("uranium_fuel",       50f);

        // Construction stockpile — scaled by difficulty.
        State.Resources.Add("steel",            800f * resMult);
        State.Resources.Add("electronics",      300f * resMult);
        State.Resources.Add("components",       150f * resMult);
        State.Resources.Add("prefab_components",100f * resMult);
        // Survival buffer — also difficulty-scaled.
        State.Resources.Add("nutrients", 100f * resMult);
        State.Resources.Add("water",     100f * resMult);
        State.Resources.Add("oxygen",    100f * resMult);
        State.Resources.Add("ice",       200f * resMult);

        State.Population.Count       = 20;
        State.Labor.TotalWorkers     = 15;

        // Initialise skill distribution from the difficulty-aware helper.
        var skills = DifficultySettings.StartingSkills(State.Labor.TotalWorkers);
        foreach (var kv in skills)
            State.Population.Skills[kv.Key] = kv.Value;

        // Construction-fleet + operator pools. Generous defaults so the early
        // game doesn't feel bottlenecked while the player learns the system.
        State.Fleet.BaseSlots   = 10;
        State.Operators.People  = 10;
        State.Operators.Robots  =  5;

        // Pre-place the starter buildings near the grid centre. They're not in
        // BuildableBuildings, so if destroyed they can't be rebuilt (fail-state
        // narrative — Phase 3C MVP, may add an "indestructible" flag later).
        PlaceStarterBuildings();

        ApplyBiomeStartingConditions(Site.Biome);
    }

    private void PlaceStarterBuildings()
    {
        int cx = State.Grid.Width / 2;
        int cy = State.Grid.Height / 2;

        // Lander: 3×3, command + power + small housing + storage. Anchor at
        // (cx - 4, cy - 1) so it sits left of centre with the habitat to the right.
        PlaceStarterAt("lander",        new GridPosition(cx - 4, cy - 1));
        PlaceStarterAt("multi_habitat", new GridPosition(cx + 1, cy - 1));
    }

    private void PlaceStarterAt(string buildingId, GridPosition origin)
    {
        if (!BuildingRegistry.TryGet(buildingId, out var maybeDef) || maybeDef is null)
            return; // content authoring problem — skip gracefully
        var def = maybeDef;

        var slot = new BuildingSlot
        {
            Origin = origin,
            Size   = def.Size,
            BuildingDefinitionId = buildingId,
            State  = BuildingState.Operational, // pre-built, no construction phase
        };
        var place = State.Grid.Place(slot);
        if (!place.Success) return; // off-grid for very small maps — skip

        // Register with the power grid like AdvanceConstruction does on completion.
        State.Power.RegisterConsumer(slot.Id, def.PowerConsumption);
        if (def.PowerProduction > 0)
            State.Power.RegisterProducer(slot.Id, def.PowerProduction);
        if (def.IsEssential)
            State.Power.SetEssential(slot.Id, true);
    }

    /// <summary>
    /// Adjusts starting resources and population to reflect the chosen biome's
    /// natural advantages and hardships.
    /// </summary>
    private void ApplyBiomeStartingConditions(BiomeType biome)
    {
        switch (biome)
        {
            case BiomeType.Rocky:
                // Rich in surface ore, but harsher living conditions.
                State.Resources.Add("steel", 200f);
                State.Resources.Add("iron_ore", 200f);
                State.Resources.TryConsume("nutrients", 30f);
                State.Resources.TryConsume("water", 30f);
                break;

            case BiomeType.Polar:
                // Abundant water ice, but nutrient scarcity and cramped quarters.
                State.Resources.SetCap("ice", 1000f);
                State.Resources.Add("ice", 300f);
                State.Resources.Add("water", 100f);
                State.Resources.TryConsume("nutrients", 40f);
                break;

            case BiomeType.Desert:
                // Good solar conditions (modelled via cheaper power; no direct mechanic yet).
                // Water is extremely scarce; nutrients are moderate.
                State.Resources.TryConsume("water", 60f);
                State.Resources.TryConsume("ice", 100f);
                State.Resources.Add("nutrients", 20f);
                break;

            case BiomeType.Volcanic:
                // Geothermal energy source; harsh and resource-poor on the surface.
                State.Resources.Add("uranium_fuel", 20f);
                State.Resources.TryConsume("steel", 200f);
                State.Resources.TryConsume("water", 50f);
                State.Resources.TryConsume("nutrients", 50f);
                break;

            case BiomeType.MarginalHabitable:
                // The most forgiving biome — extra food buffer, more starting pop.
                State.Resources.Add("nutrients", 60f);
                State.Resources.Add("food", 40f);
                State.Resources.Add("water", 40f);
                State.Population.Count = 25;
                State.Labor.TotalWorkers = 18;
                break;

            case BiomeType.Barren:
            default:
                // No modifier — the base defaults represent Barren conditions.
                break;
        }
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

public sealed class UpgradeResult
{
    public bool Success { get; init; }
    public string? FailureReason { get; init; }
    public static UpgradeResult Ok() => new() { Success = true };
    public static UpgradeResult Fail(string reason) => new() { Success = false, FailureReason = reason };
}
