using System;

namespace Outpost3.Core.Events;

/// <summary>
/// Event emitted when a mechanical failure occurs on the colony ship during the journey.
/// Represents technical problems that may require crew intervention or resources.
/// </summary>
public record MechanicalFailureEvent : GameEvent
{
    /// <summary>
    /// The ship system that experienced the failure (e.g., "Life Support", "Engine", "Navigation").
    /// </summary>
    public string SystemAffected { get; init; } = string.Empty;

    /// <summary>
    /// The severity level of the failure (e.g., "Minor", "Major", "Critical").
    /// </summary>
    public string Severity { get; init; } = string.Empty;

    /// <summary>
    /// A detailed description of the mechanical failure and its effects.
    /// </summary>
    public string Description { get; init; } = string.Empty;

    /// <summary>
    /// Creates a new MechanicalFailureEvent.
    /// </summary>
    public MechanicalFailureEvent()
    {
    }

    /// <summary>
    /// Creates a new MechanicalFailureEvent with specified values.
    /// </summary>
    public MechanicalFailureEvent(string systemAffected, string severity, string description)
    {
        SystemAffected = systemAffected;
        Severity = severity;
        Description = description;
    }
}
