using Godot;
using System.Collections.Generic;
using System;

namespace Outpost3.Core.Domain;

///
/// Immutable root game state
/// 
public record GameState
{
    public double GameTime { get; init; } = 0.0;
    public List<StarSystem> Systems { get; init; } = new();
    public List<ProbeInFlight> ProbesInFlight { get; init; } = new();

    /// <summary>
    /// The currently selected star system (for UI display).
    /// Null if no system is selected.
    /// </summary>
    public Ulid? SelectedSystemId { get; init; }

    /// <summary>
    /// Create new initial game state
    /// </summary>
    /// <returns></returns>
    public static GameState NewGame()
    {
        return new GameState();
    }

    /// <summary>
    /// Advance game time
    /// </summary>
    /// <param name="dt"></param>
    /// <returns></returns>
    public GameState WithAdvanceTime(double dt)
    {
        return this with { GameTime = this.GameTime + dt };
    }

    public GameState WithProbeLaunched(Ulid targetSystemId, double arrivalTime, out Ulid probeId)
    {
        probeId = Ulid.NewUlid();
        var probe = new ProbeInFlight
        {
            Id = probeId,
            TargetSystemId = targetSystemId,
            ArrivalTime = arrivalTime
        };

        var newProbes = new List<ProbeInFlight>(this.ProbesInFlight) { probe };
        return this with { ProbesInFlight = newProbes };
    }

    /// <summary>
    /// Remove probes that have arrived
    /// </summary>
    /// <param name="probeIds"></param>
    /// <returns></returns>
    public GameState WithProbesRemoved(List<Ulid> probeIds)
    {
        var newProbes = ProbesInFlight.FindAll(p => !probeIds.Contains(p.Id));
        return this with { ProbesInFlight = newProbes };
    }

    /// <summary>
    /// Add a discovered star system
    /// </summary>
    /// <param name="system"></param>
    /// <returns></returns>
    public GameState WithSystemDiscovered(StarSystem system)
    {
        // dont add duplicates
        if (Systems.Exists(s => s.Id == system.Id))
        {
            return this;
        }
        var newSystems = new List<StarSystem>(this.Systems) { system };
        return this with { Systems = newSystems };
    }

    /// <summary>
    /// Updates an existing system in the state (e.g., after probe scan).
    /// </summary>
    /// <param name="updatedSystem">The updated system.</param>
    /// <returns>A new GameState with the system updated.</returns>
    public GameState WithSystemUpdated(StarSystem updatedSystem)
    {
        var newSystems = new List<StarSystem>();
        bool found = false;

        foreach (var system in Systems)
        {
            if (system.Id == updatedSystem.Id)
            {
                newSystems.Add(updatedSystem);
                found = true;
            }
            else
            {
                newSystems.Add(system);
            }
        }

        // If not found, add it (shouldn't happen, but defensive)
        if (!found)
        {
            newSystems.Add(updatedSystem);
        }

        return this with { Systems = newSystems };
    }

    /// <summary>
    /// Initializes the galaxy with a list of star systems.
    /// Replaces all existing systems.
    /// </summary>
    /// <param name="systems">The list of systems in the galaxy.</param>
    /// <returns>A new GameState with the galaxy initialized.</returns>
    public GameState WithGalaxyInitialized(List<StarSystem> systems)
    {
        return this with { Systems = systems };
    }

    /// <summary>
    /// Creates a new state with the selected system updated.
    /// </summary>
    /// <param name="systemId">The ID of the system to select, or null to deselect.</param>
    /// <returns>A new GameState with the updated selection.</returns>
    public GameState WithSelectedSystem(Ulid? systemId)
    {
        return this with { SelectedSystemId = systemId };
    }
}