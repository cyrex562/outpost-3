namespace OutpostGame.Core.Simulation;

using OutpostGame.Core.Colony;

public sealed class ColonyTurnProcessor
{
    private readonly ColonyState      _state;
    private readonly RandomEventProcessor _events;
    private readonly DifficultyPreset _difficulty;

    public ColonyTurnProcessor(ColonyState state, int eventSeed = 0,
                               DifficultyPreset difficulty = DifficultyPreset.Normal)
    {
        _state      = state;
        _difficulty = difficulty;
        _events     = new RandomEventProcessor(state, eventSeed);
    }

    // Called each turn by TurnManager.TurnAdvanced event
    public void ProcessTurn(int sol)
    {
        var before = _state.Resources.Snapshot();

        // Random events fire at the START of a strategic turn so they only
        // target buildings that were already Operational before this turn's
        // AdvanceConstruction runs (prevents race with same-sol completion).
        int eventInterval = DifficultySettings.StrategicEventInterval(_difficulty);
        if (eventInterval > 0 && sol > 0 && sol % eventInterval == 0)
            _events.ProcessStrategicTurn(sol);

        AdvanceConstruction(sol);
        ApplyDustStormEffects(sol);
        ProcessProduction();
        ProcessFleetGrowth();
        ProcessConsumption();
        ProcessPopulationNeeds();
        ProcessDeaths();
        ProcessPopulationGrowth(sol);
        RecomputePowerGrid();
        CheckColonyStatus(sol);

        _state.ResourceDeltas = ComputeDeltas(before, _state.Resources.Snapshot());
    }

    private static Dictionary<string, float> ComputeDeltas(
        IReadOnlyDictionary<string, float> before,
        IReadOnlyDictionary<string, float> after)
    {
        var deltas = new Dictionary<string, float>();
        foreach (var kv in after)
            deltas[kv.Key] = kv.Value - before.GetValueOrDefault(kv.Key, 0f);
        foreach (var kv in before)
            if (!deltas.ContainsKey(kv.Key))
                deltas[kv.Key] = -kv.Value;
        return deltas;
    }

    /// <summary>
    /// Phase 3C: construction now gates on fleet + operator availability. A
    /// slot in <see cref="BuildingState.UnderConstruction"/> only ticks down
    /// while it holds the building's <see cref="BuildingDefinition.RequiredFleetSlots"/>
    /// fleet slots and <see cref="BuildingDefinition.RequiredOperators"/> operators.
    /// Allocation is FIFO by <c>AllocatedAtSol</c> then <c>Id</c> so order is
    /// deterministic across save/load. On completion the held pools are released.
    /// </summary>
    private void AdvanceConstruction(int currentSol)
    {
        // Build a deterministic order: slots that have been allocated longest
        // go first; ties (e.g., unallocated) fall back to Id ordering.
        var building = _state.Grid.AllSlots
            .Where(s => s.State == BuildingState.UnderConstruction)
            .OrderBy(s => s.AllocatedAtSol < 0 ? int.MaxValue : s.AllocatedAtSol)
            .ThenBy(s => s.Id)
            .ToList();

        foreach (var slot in building)
        {
            var def = BuildingRegistry.Get(slot.BuildingDefinitionId);
            bool allocated = slot.AllocatedFleetSlots > 0 || def.RequiredFleetSlots == 0;

            // Try to allocate fleet + operators for any slot that isn't yet running.
            if (!allocated)
            {
                int needSlots = def.RequiredFleetSlots;
                int needOps   = def.RequiredOperators;
                if (_state.Fleet.Available >= needSlots
                    && _state.Operators.Available >= needOps
                    && _state.Fleet.TryAllocate(needSlots))
                {
                    if (_state.Operators.TryAllocate(needOps))
                    {
                        slot.AllocatedFleetSlots = needSlots;
                        slot.AllocatedOperators  = needOps;
                        slot.AllocatedAtSol      = currentSol;
                        allocated = true;
                    }
                    else
                    {
                        // Roll back the fleet allocation since operators were missing.
                        _state.Fleet.Release(needSlots);
                    }
                }
            }

            // Hard stop: no progress while not allocated.
            if (!allocated) continue;

            slot.ConstructionTurnsRemaining--;
            if (slot.ConstructionTurnsRemaining <= 0)
            {
                // Release pools back.
                if (slot.AllocatedFleetSlots > 0) _state.Fleet.Release(slot.AllocatedFleetSlots);
                if (slot.AllocatedOperators  > 0) _state.Operators.Release(slot.AllocatedOperators);
                slot.AllocatedFleetSlots = 0;
                slot.AllocatedOperators  = 0;
                slot.AllocatedAtSol      = -1;

                slot.State = BuildingState.Operational;
                _state.Power.RegisterConsumer(slot.Id, def.PowerConsumption);
                if (def.PowerProduction > 0) _state.Power.RegisterProducer(slot.Id, def.PowerProduction);
                if (def.IsEssential) _state.Power.SetEssential(slot.Id, true);
            }
        }
    }

    /// <summary>
    /// Each operational + powered <c>vehicle_factory</c> ticks its production
    /// cycle and, on completion, permanently raises the colony's construction
    /// fleet capacity by 1. Modelled as its own pass rather than via the normal
    /// recipe loop because the "output" is a state mutation, not a resource.
    /// </summary>
    private void ProcessFleetGrowth()
    {
        const string FactoryId = "vehicle_factory";
        foreach (var slot in _state.Grid.AllSlots
                                 .Where(s => s.State == BuildingState.Operational
                                          && s.BuildingDefinitionId == FactoryId))
        {
            if (!_state.Power.IsPowered(slot.Id)) continue;

            var def = BuildingRegistry.Get(FactoryId);
            int cycle = def.Recipe?.TurnsPerCycle ?? 60;

            // Use the same accumulator the normal recipe processor would.
            slot.ProductionCycleProgress++;
            if (slot.ProductionCycleProgress < cycle) continue;
            slot.ProductionCycleProgress = 0;

            // Also charge the cycle's input resources, matching the normal
            // recipe semantics so the factory isn't free output.
            if (def.Recipe is { } recipe && recipe.Inputs.Count > 0)
            {
                var inputs = recipe.Inputs.ToDictionary(kv => kv.Key, kv => kv.Value);
                if (!_state.Resources.HasEnough(inputs))
                {
                    // Roll the accumulator back so the player can retry next sol.
                    slot.ProductionCycleProgress = cycle - 1;
                    continue;
                }
                _state.Resources.TryConsume(inputs);
            }

            _state.Fleet.FactorySlots++;
            _state.EventLog.Add(new ColonyEvent(
                ColonyEventSeverity.Info,
                $"Vehicle Factory rolled out a new construction vehicle (fleet capacity now {_state.Fleet.TotalSlots}).",
                _state.TurnManager.CurrentSol));
        }
    }

    /// <summary>
    /// Decrements the storm counter and adjusts solar producer efficiency each turn.
    /// Efficiency is set to 20 % while a storm is active and restored when it clears.
    /// </summary>
    private void ApplyDustStormEffects(int sol)
    {
        if (_state.DustStormTurnsRemaining <= 0) return;

        _state.DustStormTurnsRemaining--;

        if (_state.DustStormTurnsRemaining == 0)
        {
            // Storm over — restore solar producers.
            foreach (var slot in _state.Grid.AllSlots
                .Where(s => BuildingRegistry.Get(s.BuildingDefinitionId).IsSolar))
                _state.Power.ResetProducerEfficiency(slot.Id);
            _state.EventLog.Add(new ColonyEvent(
                ColonyEventSeverity.Info, "Dust storm has passed.", sol));
        }
        else
        {
            // Storm active — throttle solar producers.
            foreach (var slot in _state.Grid.AllSlots
                .Where(s => s.State == BuildingState.Operational
                         && BuildingRegistry.Get(s.BuildingDefinitionId).IsSolar))
                _state.Power.SetProducerEfficiency(slot.Id, 0.2f);
        }
    }

    private void ProcessProduction()
    {
        foreach (var slot in _state.Grid.AllSlots.Where(s => s.State == BuildingState.Operational))
        {
            if (!_state.Power.IsPowered(slot.Id)) continue;
            var def = BuildingRegistry.Get(slot.BuildingDefinitionId);
            if (def.Recipe == null) continue;

            float efficiency = _state.Labor.Efficiency(slot.Id, def.LaborRequired,
                _state.Population.MoraleModifier, def.PrimarySkill);

            slot.ProductionCycleProgress++;
            if (slot.ProductionCycleProgress < def.Recipe.TurnsPerCycle) continue;
            slot.ProductionCycleProgress = 0;

            if (!_state.Resources.HasEnough(
                def.Recipe.Inputs.ToDictionary(kv => kv.Key, kv => kv.Value))) continue;

            _state.Resources.TryConsume(
                def.Recipe.Inputs.ToDictionary(kv => kv.Key, kv => kv.Value));

            foreach (var output in def.Recipe.Outputs)
                _state.Resources.Add(output.Key, output.Value * efficiency);
        }
    }

    private void ProcessConsumption()
    {
        float mult = DifficultySettings.ConsumptionMultiplier(_difficulty);
        foreach (var slot in _state.Grid.AllSlots.Where(s => s.State == BuildingState.Operational))
        {
            var def = BuildingRegistry.Get(slot.BuildingDefinitionId);
            if (def.MaintenanceCost == null) continue;
            foreach (var cost in def.MaintenanceCost)
                _state.Resources.TryConsume(cost.Key, cost.Value * mult);
        }
    }

    private void ProcessPopulationNeeds()
    {
        var pop = _state.Population;
        float mult = DifficultySettings.ConsumptionMultiplier(_difficulty);
        float totalFood = _state.Resources.Get("nutrients");
        float totalWater = _state.Resources.Get("water");
        float totalOxygen = _state.Resources.Get("oxygen");

        float demanded = pop.Count * pop.Needs.FoodPerSol * mult;
        float foodMet = demanded > 0 ? Math.Min(totalFood / demanded, 1f) : 1f;
        _state.Resources.TryConsume("nutrients", Math.Min(totalFood, demanded));

        demanded = pop.Count * pop.Needs.WaterPerSol * mult;
        float waterMet = demanded > 0 ? Math.Min(totalWater / demanded, 1f) : 1f;
        _state.Resources.TryConsume("water", Math.Min(totalWater, demanded));

        demanded = pop.Count * pop.Needs.OxygenPerSol * mult;
        float oxygenMet = demanded > 0 ? Math.Min(totalOxygen / demanded, 1f) : 1f;
        _state.Resources.TryConsume("oxygen", Math.Min(totalOxygen, demanded));

        float housingCapacity = TotalHousingCapacity();
        float housingMet = pop.Count > 0 ? Math.Min(housingCapacity / pop.Count, 1f) : 1f;

        pop.ApplyNeedsSatisfaction(foodMet, waterMet, oxygenMet, housingMet);
    }

    private void ProcessDeaths()
    {
        int deaths = _state.Population.ComputeDeaths();
        if (deaths > 0)
        {
            _state.Population.Count = Math.Max(0, _state.Population.Count - deaths);
            _state.EventLog.Add(new ColonyEvent(
                ColonyEventSeverity.Critical,
                $"{deaths} colonist(s) died due to unmet survival needs.",
                _state.TurnManager.CurrentSol));
        }
    }

    /// <summary>
    /// Population grows slowly when health and morale are high, capped by habitat capacity.
    /// Growth rate: ~0.2 % per sol when fully satisfied (doubles ~every 500 sols).
    /// </summary>
    private void ProcessPopulationGrowth(int sol)
    {
        var pop = _state.Population;
        if (pop.Count <= 0) return;

        // Only grow when health and morale are good.
        if (pop.Health < 60f || pop.Morale < 50f) return;

        float habitatCapacity = TotalHousingCapacity();

        if (pop.Count >= (int)habitatCapacity) return;

        // 0.2 % growth per sol → accumulate fractional colonists, add whole ones.
        float growth = pop.Count * 0.002f;
        GrowthAccumulator += growth;
        if (GrowthAccumulator >= 1f)
        {
            int newColonists = (int)GrowthAccumulator;
            newColonists = Math.Min(newColonists, (int)habitatCapacity - pop.Count);
            pop.Count += newColonists;
            GrowthAccumulator -= newColonists;

            _state.EventLog.Add(new ColonyEvent(
                ColonyEventSeverity.Info,
                $"{newColonists} new colonist(s) arrived (pop now {pop.Count}).",
                sol));
        }
    }

    private void CheckColonyStatus(int sol)
    {
        var s = _state;
        // Terminal states are final.
        if (s.Status is ColonyStatus.Destroyed or ColonyStatus.Abandoned) return;

        // ── Destruction ────────────────────────────────────────────────────
        if (s.Population.Count <= 0)
        {
            s.Status = ColonyStatus.Destroyed;
            s.EventLog.Add(new ColonyEvent(ColonyEventSeverity.Critical,
                "All colonists have perished. Colony lost.", sol));
            return;
        }

        // ── Morale collapse tracking ───────────────────────────────────────
        const float CriticalMorale = 20f;
        const int CriticalAfterTurns  = 5;
        const int AbandonedAfterTurns = 15;

        if (s.Population.Morale < CriticalMorale)
        {
            s.LowMoraleTurns++;
            if (s.LowMoraleTurns >= AbandonedAfterTurns)
            {
                s.Status = ColonyStatus.Abandoned;
                s.EventLog.Add(new ColonyEvent(ColonyEventSeverity.Critical,
                    "Colonists have abandoned the settlement — morale completely collapsed.", sol));
                return;
            }
            if (s.LowMoraleTurns >= CriticalAfterTurns && s.Status == ColonyStatus.Active)
            {
                s.Status = ColonyStatus.Critical;
                s.EventLog.Add(new ColonyEvent(ColonyEventSeverity.Critical,
                    "Colony morale in freefall — colonists are considering leaving.", sol));
            }
        }
        else
        {
            // Morale recovered — reset counter and downgrade Critical → Active.
            s.LowMoraleTurns = 0;
            if (s.Status == ColonyStatus.Critical)
            {
                s.Status = ColonyStatus.Active;
                s.EventLog.Add(new ColonyEvent(ColonyEventSeverity.Info,
                    "Colony morale has stabilised.", sol));
            }
        }

        // ── Thriving ───────────────────────────────────────────────────────
        const int ThrivingAfterTurns = 30;
        bool conditionsMet = s.Population.Count >= 50
                          && s.Population.Morale  >= 80f
                          && s.Population.Health  >= 80f;

        if (conditionsMet)
        {
            s.ThrivingTurns++;
            if (s.ThrivingTurns >= ThrivingAfterTurns && s.Status == ColonyStatus.Active)
            {
                s.Status = ColonyStatus.Thriving;
                s.EventLog.Add(new ColonyEvent(ColonyEventSeverity.Info,
                    "Colony is thriving! A true home among the stars.", sol));
            }
        }
        else
        {
            if (s.ThrivingTurns > 0 && s.Status == ColonyStatus.Thriving)
                s.Status = ColonyStatus.Active; // Conditions no longer met
            s.ThrivingTurns = 0;
        }
    }

    public float GrowthAccumulator { get; set; }

    private void RecomputePowerGrid()
    {
        float storageCapacity = _state.Grid.AllSlots
            .Where(s => s.State == BuildingState.Operational)
            .Sum(s => BuildingRegistry.Get(s.BuildingDefinitionId).StorageCapacity);
        foreach (var res in ResourceRegistry.All.Values.Where(r => r.Tier != ResourceTier.Virtual))
            _state.Resources.SetCap(res.Id, 100f + storageCapacity);
    }

    private float TotalHousingCapacity()
    {
        return _state.Grid.AllSlots
            .Where(s => s.State == BuildingState.Operational)
            .Sum(s => (float)BuildingRegistry.Get(s.BuildingDefinitionId).HousingCapacity);
    }
}
