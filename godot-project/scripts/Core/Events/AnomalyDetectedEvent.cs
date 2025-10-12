using System;

namespace Outpost3.Core.Events;

/// <summary>
/// Event emitted when the ship's sensors detect a spatial anomaly during the journey.
/// Represents opportunities for discovery, danger, or narrative events.
/// </summary>
public record AnomalyDetectedEvent : GameEvent
{
    /// <summary>
    /// The type or category of anomaly detected (e.g., "Wormhole", "Derelict Ship", "Energy Signature").
    /// </summary>
    public string AnomalyType { get; init; } = string.Empty;

    /// <summary>
    /// A detailed description of the anomaly and what was detected.
    /// </summary>
    public string Description { get; init; } = string.Empty;

    /// <summary>
    /// Creates a new AnomalyDetectedEvent.
    /// </summary>
    public AnomalyDetectedEvent()
    {
    }

    /// <summary>
    /// Creates a new AnomalyDetectedEvent with specified values.
    /// </summary>
    public AnomalyDetectedEvent(string anomalyType, string description)
    {
        AnomalyType = anomalyType;
        Description = description;
    }
}
