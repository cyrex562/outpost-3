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
}