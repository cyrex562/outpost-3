using System;

namespace Outpost3.Core.Events;

/// <summary>
/// Event emitted when social conflict arises among colonists during the journey.
/// Represents interpersonal tensions, disputes, or cultural clashes that affect morale.
/// </summary>
public record SocialConflictEvent : GameEvent
{
    /// <summary>
    /// The type or category of social conflict (e.g., "Ideological", "Resource Dispute", "Leadership Challenge").
    /// </summary>
    public string ConflictType { get; init; } = string.Empty;

    /// <summary>
    /// A detailed description of the social conflict and its context.
    /// </summary>
    public string Description { get; init; } = string.Empty;

    /// <summary>
    /// The impact on crew morale (negative values reduce morale, positive values improve it).
    /// Typical range: -1.0 to 1.0.
    /// </summary>
    public float MoraleImpact { get; init; }

    /// <summary>
    /// Creates a new SocialConflictEvent.
    /// </summary>
    public SocialConflictEvent()
    {
    }

    /// <summary>
    /// Creates a new SocialConflictEvent with specified values.
    /// </summary>
    public SocialConflictEvent(string conflictType, string description, float moraleImpact)
    {
        ConflictType = conflictType;
        Description = description;
        MoraleImpact = moraleImpact;
    }
}
