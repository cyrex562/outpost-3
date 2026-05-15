namespace OutpostGame.Core.Simulation;

using OutpostGame.Core.Colony;

public sealed class ColonyTurnProcessor
{
    private readonly ColonyState _state;

    public ColonyTurnProcessor(ColonyState state) => _state = state;

    // Called each turn by TurnManager.TurnAdvanced event
    public void ProcessTurn(int sol)
    {
        AdvanceConstruction();
        ProcessProduction();
        ProcessConsumption();
        ProcessPopulationNeeds();
        ProcessDeaths();
        RecomputePowerGrid();
    }

    private void AdvanceConstruction()
    {
        foreach (var slot in _state.Grid.AllSlots.Where(s => s.State == BuildingState.UnderConstruction))
        {
            slot.ConstructionTurnsRemaining--;
            if (slot.ConstructionTurnsRemaining <= 0)
            {
                slot.State = BuildingState.Operational;
                var def = BuildingRegistry.Get(slot.BuildingDefinitionId);
                _state.Power.RegisterConsumer(slot.Id, def.PowerConsumption);
                if (def.PowerProduction > 0) _state.Power.RegisterProducer(slot.Id, def.PowerProduction);
                if (def.IsEssential) _state.Power.SetEssential(slot.Id, true);
            }
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
                _state.Population.MoraleModifier);

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
        foreach (var slot in _state.Grid.AllSlots.Where(s => s.State == BuildingState.Operational))
        {
            var def = BuildingRegistry.Get(slot.BuildingDefinitionId);
            if (def.MaintenanceCost == null) continue;
            foreach (var cost in def.MaintenanceCost)
                _state.Resources.TryConsume(cost.Key, cost.Value);
        }
    }

    private void ProcessPopulationNeeds()
    {
        var pop = _state.Population;
        float totalFood = _state.Resources.Get("nutrients");
        float totalWater = _state.Resources.Get("water");
        float totalOxygen = _state.Resources.Get("oxygen");

        float demanded = pop.Count * pop.Needs.FoodPerSol;
        float foodMet = demanded > 0 ? Math.Min(totalFood / demanded, 1f) : 1f;
        _state.Resources.TryConsume("nutrients", Math.Min(totalFood, demanded));

        demanded = pop.Count * pop.Needs.WaterPerSol;
        float waterMet = demanded > 0 ? Math.Min(totalWater / demanded, 1f) : 1f;
        _state.Resources.TryConsume("water", Math.Min(totalWater, demanded));

        demanded = pop.Count * pop.Needs.OxygenPerSol;
        float oxygenMet = demanded > 0 ? Math.Min(totalOxygen / demanded, 1f) : 1f;
        _state.Resources.TryConsume("oxygen", Math.Min(totalOxygen, demanded));

        // Housing: count habitat capacity
        float housingCapacity = _state.Grid.AllSlots
            .Where(s => s.BuildingDefinitionId == "habitat" && s.State == BuildingState.Operational)
            .Sum(_ => 20f);  // 20 colonists per habitat
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

    private void RecomputePowerGrid()
    {
        // Recalculate storage caps from warehouses
        float warehouseCapacity = _state.Grid.AllSlots
            .Where(s => s.BuildingDefinitionId == "warehouse" && s.State == BuildingState.Operational)
            .Sum(_ => 500f);
        // Apply cap to all non-virtual resources
        foreach (var res in ResourceRegistry.All.Values.Where(r => r.Tier != ResourceTier.Virtual))
            _state.Resources.SetCap(res.Id, 100f + warehouseCapacity);
    }
}
