namespace OutpostGame.Core.Colony;

public enum EventTriggerType
{
    Strategic,      // Fires on a strategic turn with given probability
    RandomPerSol,   // Fires every sol with given probability
    Conditional,    // Fires when a condition becomes true (future)
}

public sealed record EventTrigger(
    EventTriggerType Type,
    float Probability,
    int MinSol,
    IReadOnlyDictionary<string, float> RequiredResources
);

public sealed record EventOutcome(
    string Kind,                  // "resource_change", "log", "damage_random", "dust_storm", "population_add", "spawn_decision"
    string? Resource = null,      // For resource_change / population_add
    float Amount = 0f,            // Signed amount for resource_change / population_add
    string? Message = null,       // For log
    int Duration = 0,             // Sols (e.g. dust storm length)
    string? DecisionId = null     // For spawn_decision — points to another EventDefinition
);

public sealed record EventChoiceDefinition(
    string Label,
    string ConsequenceSummary,
    IReadOnlyList<EventOutcome> Outcomes
);

public sealed record EventDefinition(
    string Id,
    string Title,
    ColonyEventSeverity Severity,
    EventTrigger Trigger,
    IReadOnlyList<EventOutcome> Outcomes,
    IReadOnlyList<EventChoiceDefinition> Choices,
    string Description = ""
);
