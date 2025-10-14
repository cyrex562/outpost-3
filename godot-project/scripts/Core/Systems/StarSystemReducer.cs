using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using Outpost3.Core.Commands;
using Outpost3.Core.Domain;
using Outpost3.Core.Events;

namespace Outpost3.Core.Systems;

/// <summary>
/// Pure reducer for star system commands.
/// Handles system generation, body selection, camera state, and UI controls.
/// </summary>
public static class StarSystemReducer
{
    /// <summary>
    /// Handles GenerateSystemDetails command - procedurally generates system bodies, belts, and Oort cloud.
    /// Only generates once per system (idempotent based on Seed presence).
    /// </summary>
    public static (GameState newState, List<IGameEvent> events) HandleGenerateSystemDetails(
        GameState state,
        GenerateSystemDetails command)
    {
        // Find the system
        var system = state.Systems.FirstOrDefault(s => s.Id == command.SystemId);
        if (system == null)
        {
            // System not found - ignore command
            return (state, new List<IGameEvent>());
        }

        // Check if already generated (idempotent)
        if (system.Seed != null)
        {
            // Already generated - no-op
            return (state, new List<IGameEvent>());
        }

        // Generate seed from system ID
        var seed = SystemSeed.FromSystemId(command.SystemId);

        // Generate procedural content
        var (bodies, belts, oortCloud) = ProceduralGenerator.GenerateSystemDetails(
            system,
            seed
        );

        // Update system with generated details
        var updatedSystem = system with
        {
            Seed = seed,
            Bodies = bodies.Select(b => new CelestialBody
            {
                Id = b.Id,
                Name = b.Name,
                BodyType = b.BodyType,
                Composition = b.Composition,
                Explored = b.Explored,
                AtmosphereType = b.AtmosphereType,
                SurfaceType = b.SurfaceType,
                OrbitalParams = b.OrbitalParams,
                MassEarthMasses = b.MassEarthMasses,
                RadiusKm = b.RadiusKm
            }).ToList(),
            Belts = belts,
            OortCloud = oortCloud
        };

        // Update state
        var newState = state.WithSystemUpdated(updatedSystem);

        // Create event
        var evt = new SystemDetailsGenerated(
            command.SystemId,
            seed,
            bodies,
            belts,
            oortCloud
        )
        {
            GameTime = (float)state.GameTime
        };

        return (newState, new List<IGameEvent> { evt });
    }

    /// <summary>
    /// Handles SelectCelestialBody command - selects a body for detailed view.
    /// </summary>
    public static (GameState newState, List<IGameEvent> events) HandleSelectCelestialBody(
        GameState state,
        SelectCelestialBody command)
    {
        // Validate body exists in current system
        var currentSystem = state.SelectedSystemId.HasValue
            ? state.Systems.FirstOrDefault(s => s.Id == state.SelectedSystemId.Value)
            : null;

        if (currentSystem == null)
        {
            // No system selected - ignore
            return (state, new List<IGameEvent>());
        }

        var bodyExists = currentSystem.Bodies.Any(b => b.Id == command.BodyId!.Value);
        if (!bodyExists)
        {
            // Body not found in current system - ignore
            return (state, new List<IGameEvent>());
        }

        var newState = state with { SelectedBodyId = command.BodyId };

        var evt = new CelestialBodySelected(command.BodyId)
        {
            GameTime = (float)state.GameTime
        };

        return (newState, new List<IGameEvent> { evt });
    }

    /// <summary>
    /// Handles UpdateCamera command - updates camera state for current system.
    /// </summary>
    public static (GameState newState, List<IGameEvent> events) HandleUpdateCamera(
        GameState state,
        UpdateCamera command)
    {
        if (!state.SelectedSystemId.HasValue)
        {
            // No system selected - ignore
            return (state, new List<IGameEvent>());
        }

        var systemId = state.SelectedSystemId.Value;

        // Update camera state for this system
        var newCameraStates = new Dictionary<Ulid, CameraState>(state.CameraStates)
        {
            [systemId] = command.State
        };

        var newState = state with { CameraStates = newCameraStates };

        var evt = new CameraUpdated(systemId, command.State)
        {
            GameTime = (float)state.GameTime
        };

        return (newState, new List<IGameEvent> { evt });
    }

    /// <summary>
    /// Handles ResetCamera command - resets camera to default view for current system.
    /// </summary>
    public static (GameState newState, List<IGameEvent> events) HandleResetCamera(
        GameState state,
        ResetCamera command)
    {
        if (!state.SelectedSystemId.HasValue)
        {
            // No system selected - ignore
            return (state, new List<IGameEvent>());
        }

        var systemId = state.SelectedSystemId.Value;

        // Default camera state: centered on star, zoom 1.0
        var defaultCamera = new CameraState(
            PanPosition: new Vector2(0, 0),
            ZoomLevel: 1.0f
        );

        // Update camera state for this system
        var newCameraStates = new Dictionary<Ulid, CameraState>(state.CameraStates)
        {
            [systemId] = defaultCamera
        };

        var newState = state with { CameraStates = newCameraStates };

        var evt = new CameraReset(systemId)
        {
            GameTime = (float)state.GameTime
        };

        return (newState, new List<IGameEvent> { evt });
    }

    /// <summary>
    /// Handles ToggleSystemOverviewPanel command - toggles panel visibility.
    /// </summary>
    public static (GameState newState, List<IGameEvent> events) HandleToggleSystemOverviewPanel(
        GameState state,
        ToggleSystemOverviewPanel command)
    {
        var newState = state with
        {
            SystemOverviewPanelOpen = !state.SystemOverviewPanelOpen
        };

        var evt = new SystemOverviewPanelToggled(newState.SystemOverviewPanelOpen)
        {
            GameTime = (float)state.GameTime
        };

        return (newState, new List<IGameEvent> { evt });
    }

    /// <summary>
    /// Handles SetGameSpeed command - changes game speed multiplier.
    /// </summary>
    public static (GameState newState, List<IGameEvent> events) HandleSetGameSpeed(
        GameState state,
        SetGameSpeed command)
    {
        var newState = state with { CurrentSpeed = command.Speed };

        var evt = new GameSpeedChanged(command.Speed)
        {
            GameTime = (float)state.GameTime
        };

        return (newState, new List<IGameEvent> { evt });
    }

    /// <summary>
    /// Handles TogglePause command - toggles pause state.
    /// </summary>
    public static (GameState newState, List<IGameEvent> events) HandleTogglePause(
        GameState state,
        TogglePause command)
    {
        var newState = state with { IsPaused = !state.IsPaused };

        var evt = new GamePauseToggled(newState.IsPaused)
        {
            GameTime = (float)state.GameTime
        };

        return (newState, new List<IGameEvent> { evt });
    }
}
