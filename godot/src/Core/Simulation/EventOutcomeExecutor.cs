namespace OutpostGame.Core.Simulation;

using OutpostGame.Core.Colony;

/// <summary>
/// Applies an <see cref="EventOutcome"/> from the JSON event registry against
/// a <see cref="ColonyState"/>. Each <c>Kind</c> maps to a concrete simulation
/// side effect. Unknown kinds are no-ops (validation happens at load time, so
/// reaching here with a bad kind would indicate a programming error rather
/// than a content error).
/// </summary>
public static class EventOutcomeExecutor
{
    public static void Apply(ColonyState state, EventOutcome outcome, Random rng, int sol)
    {
        switch (outcome.Kind)
        {
            case "resource_change":
                ApplyResourceChange(state, outcome);
                break;

            case "log":
                if (!string.IsNullOrEmpty(outcome.Message))
                    state.EventLog.Add(new ColonyEvent(
                        ColonyEventSeverity.Info, outcome.Message!, sol));
                break;

            case "damage_random":
                DamageRandomOperationalBuilding(state, rng, sol);
                break;

            case "dust_storm":
                if (state.DustStormTurnsRemaining <= 0)
                    state.DustStormTurnsRemaining = outcome.Duration > 0 ? outcome.Duration : 10;
                break;

            case "population_add":
                state.Population.Count = Math.Max(0, state.Population.Count + (int)outcome.Amount);
                break;

            case "spawn_decision":
                // Reserved for nested decisions; spawn_decision is currently expressed
                // by attaching Choices to the EventDefinition itself.
                break;
        }
    }

    private static void ApplyResourceChange(ColonyState state, EventOutcome outcome)
    {
        if (string.IsNullOrEmpty(outcome.Resource)) return;
        if (outcome.Amount >= 0f)
            state.Resources.Add(outcome.Resource!, outcome.Amount);
        else
            state.Resources.TryConsume(outcome.Resource!, -outcome.Amount);
    }

    private static void DamageRandomOperationalBuilding(ColonyState state, Random rng, int sol)
    {
        // Damage only targets operational slots. Pre-placed essential starters
        // (Lander, multi_habitat) are exempt — losing them is a fail state and
        // RNG bricking the colony in sol 30 isn't great UX.
        var targets = state.Grid.AllSlots
            .Where(s => s.State == BuildingState.Operational)
            .Where(s => !IsStarterBuilding(s.BuildingDefinitionId))
            .ToList();
        if (targets.Count == 0) return;

        var slot = targets[rng.Next(targets.Count)];
        var def  = BuildingRegistry.Get(slot.BuildingDefinitionId);
        slot.State = BuildingState.Damaged;
        state.Power.Unregister(slot.Id);

        state.EventLog.Add(new ColonyEvent(
            ColonyEventSeverity.Warning,
            $"Equipment failure: {def.DisplayName} is damaged.",
            sol));
    }

    private static bool IsStarterBuilding(string id) =>
        id == "lander" || id == "multi_habitat";
}
