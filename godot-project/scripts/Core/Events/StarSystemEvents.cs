using System;
using System.Collections.Generic;
using Outpost3.Core.Domain;

namespace Outpost3.Core.Events;

/// <summary>
/// Event emitted when detailed system properties are generated for a star system.
/// </summary>
public record SystemDetailsGenerated : GameEvent
{
    /// <summary>
    /// The ID of the system that was generated.
    /// </summary>
    public Ulid SystemId { get; init; }

    /// <summary>
    /// The seed used for generation (for reproducibility).
    /// </summary>
    public SystemSeed Seed { get; init; }

    /// <summary>
    /// The generated bodies with orbital parameters.
    /// </summary>
    public List<CelestialBody> GeneratedBodies { get; init; } = new();

    /// <summary>
    /// The generated asteroid belts.
    /// </summary>
    public List<AsteroidBelt> GeneratedBelts { get; init; } = new();

    /// <summary>
    /// The generated Oort cloud.
    /// </summary>
    public OortCloud? OortCloud { get; init; }

    public SystemDetailsGenerated() { }

    public SystemDetailsGenerated(
        Ulid systemId,
        SystemSeed seed,
        List<CelestialBody> generatedBodies,
        List<AsteroidBelt> generatedBelts,
        OortCloud? oortCloud)
    {
        SystemId = systemId;
        Seed = seed;
        GeneratedBodies = generatedBodies;
        GeneratedBelts = generatedBelts;
        OortCloud = oortCloud;
    }
}

/// <summary>
/// Event emitted when a celestial body is selected or deselected.
/// </summary>
public record CelestialBodySelected : GameEvent
{
    /// <summary>
    /// The ID of the selected body, or null if deselected.
    /// </summary>
    public Ulid? BodyId { get; init; }

    public CelestialBodySelected() { }

    public CelestialBodySelected(Ulid? bodyId)
    {
        BodyId = bodyId;
    }
}

/// <summary>
/// Event emitted when the camera state is updated for a system.
/// </summary>
public record CameraUpdated : GameEvent
{
    /// <summary>
    /// The ID of the system whose camera was updated.
    /// </summary>
    public Ulid SystemId { get; init; }

    /// <summary>
    /// The new camera state.
    /// </summary>
    public CameraState State { get; init; }

    public CameraUpdated() { }

    public CameraUpdated(Ulid systemId, CameraState state)
    {
        SystemId = systemId;
        State = state;
    }
}

/// <summary>
/// Event emitted when the camera is reset to default for a system.
/// </summary>
public record CameraReset : GameEvent
{
    /// <summary>
    /// The ID of the system whose camera was reset.
    /// </summary>
    public Ulid SystemId { get; init; }

    public CameraReset() { }

    public CameraReset(Ulid systemId)
    {
        SystemId = systemId;
    }
}

/// <summary>
/// Event emitted when the system overview panel is toggled.
/// </summary>
public record SystemOverviewPanelToggled : GameEvent
{
    /// <summary>
    /// The new open state of the panel.
    /// </summary>
    public bool IsOpen { get; init; }

    public SystemOverviewPanelToggled() { }

    public SystemOverviewPanelToggled(bool isOpen)
    {
        IsOpen = isOpen;
    }
}

/// <summary>
/// Event emitted when the game speed is changed.
/// </summary>
public record GameSpeedChanged : GameEvent
{
    /// <summary>
    /// The new game speed.
    /// </summary>
    public GameSpeed NewSpeed { get; init; }

    public GameSpeedChanged() { }

    public GameSpeedChanged(GameSpeed newSpeed)
    {
        NewSpeed = newSpeed;
    }
}

/// <summary>
/// Event emitted when the game pause state is toggled.
/// </summary>
public record GamePauseToggled : GameEvent
{
    /// <summary>
    /// The new pause state.
    /// </summary>
    public bool IsPaused { get; init; }

    public GamePauseToggled() { }

    public GamePauseToggled(bool isPaused)
    {
        IsPaused = isPaused;
    }
}
