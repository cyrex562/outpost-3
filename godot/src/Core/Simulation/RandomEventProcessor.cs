namespace OutpostGame.Core.Simulation;

using OutpostGame.Core.Colony;

/// <summary>
/// Drives random events once per strategic turn. As of Phase 3.2 the simple
/// trigger-and-effect events live in <see cref="EventRegistry"/> (loaded from
/// <c>content/events.json</c>) and are dispatched through
/// <see cref="EventOutcomeExecutor"/>. A few events with bespoke gating that
/// the current schema can't express (e.g. morale-gated colonist arrivals) are
/// still hardcoded — they're the next migration target.
/// </summary>
public sealed class RandomEventProcessor
{
    private readonly ColonyState _state;
    private readonly Random      _rng;

    // Hardcoded probability for the arrival event (not yet in JSON because the
    // morale threshold isn't expressible in the current trigger schema).
    private const double ArrivalChance = 0.10;

    public RandomEventProcessor(ColonyState state, int seed)
    {
        _state = state;
        _rng   = new Random(seed);
    }

    public void ProcessStrategicTurn(int sol)
    {
        ProcessRegistryEvents(sol);
        TryTriggerArrival(sol);
    }

    private void ProcessRegistryEvents(int sol)
    {
        foreach (var evt in EventRegistry.All.Values)
        {
            if (evt.Trigger.Type != EventTriggerType.Strategic) continue;
            if (sol < evt.Trigger.MinSol) continue;
            if (!ResourcesAvailable(evt.Trigger.RequiredResources)) continue;

            // Don't pile up decision events while one is awaiting an answer.
            if (evt.Choices.Count > 0 && _state.PendingDecisions.Count > 0) continue;

            // Skip a dust-storm trigger while a storm is already active.
            if (evt.Outcomes.Any(o => o.Kind == "dust_storm")
                && _state.DustStormTurnsRemaining > 0) continue;

            if (_rng.NextDouble() > evt.Trigger.Probability) continue;

            FireEvent(evt, sol);
        }
    }

    private bool ResourcesAvailable(IReadOnlyDictionary<string, float> required)
    {
        if (required.Count == 0) return true;
        return _state.Resources.HasEnough(required);
    }

    private void FireEvent(EventDefinition evt, int sol)
    {
        if (evt.Choices.Count > 0)
        {
            // Player-decision event: enqueue choices and log a stub entry.
            var choices = evt.Choices.Select(c =>
            {
                var capturedOutcomes = c.Outcomes;
                var capturedSol = sol;
                return new EventChoice(
                    c.Label,
                    c.ConsequenceSummary,
                    state =>
                    {
                        foreach (var outcome in capturedOutcomes)
                            EventOutcomeExecutor.Apply(state, outcome, _rng, capturedSol);
                    });
            }).ToList();

            _state.PendingDecisions.Enqueue(new PendingDecision
            {
                Title       = evt.Title,
                Description = evt.Description,
                Sol         = sol,
                Choices     = choices,
            });

            _state.EventLog.Add(new ColonyEvent(
                evt.Severity, $"{evt.Title} — awaiting decision.", sol));
            return;
        }

        foreach (var outcome in evt.Outcomes)
            EventOutcomeExecutor.Apply(_state, outcome, _rng, sol);
    }

    private void TryTriggerArrival(int sol)
    {
        if (_state.Population.Morale < 60f) return;
        if (_rng.NextDouble() > ArrivalChance) return;

        float capacity = _state.Grid.AllSlots
            .Where(s => s.State == BuildingState.Operational)
            .Sum(s => (float)BuildingRegistry.Get(s.BuildingDefinitionId).HousingCapacity);

        int space = (int)capacity - _state.Population.Count;
        if (space <= 0) return;

        int arrivals = Math.Min(_rng.Next(3, 9), space);
        _state.Population.Count += arrivals;
        _state.EventLog.Add(new ColonyEvent(
            ColonyEventSeverity.Info,
            $"{arrivals} colonist(s) arrived — population now {_state.Population.Count}.",
            sol));
    }
}
