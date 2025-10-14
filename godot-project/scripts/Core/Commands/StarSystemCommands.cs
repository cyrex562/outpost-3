using System;
using Outpost3.Core.Domain;

namespace Outpost3.Core.Commands;

/// <summary>
/// Generate detailed orbital parameters and properties for a star system.
/// This is triggered when first viewing a system map.
/// </summary>
public record GenerateSystemDetails(Ulid SystemId, double CurrentGameTime) : ICommand;

/// <summary>
/// Select a celestial body for UI display.
/// Null BodyId means deselect.
/// </summary>
public record SelectCelestialBody(Ulid? BodyId) : ICommand;

/// <summary>
/// Update the camera state for a specific system.
/// </summary>
public record UpdateCamera(Ulid SystemId, CameraState State) : ICommand;

/// <summary>
/// Reset the camera to default view for a specific system.
/// </summary>
public record ResetCamera(Ulid SystemId) : ICommand;

/// <summary>
/// Toggle the system overview panel open/closed.
/// </summary>
public record ToggleSystemOverviewPanel() : ICommand;

/// <summary>
/// Set the game speed multiplier.
/// </summary>
public record SetGameSpeed(GameSpeed Speed) : ICommand;

/// <summary>
/// Toggle game pause state.
/// </summary>
public record TogglePause() : ICommand;
