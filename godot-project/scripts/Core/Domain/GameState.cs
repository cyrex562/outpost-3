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

    // NEW: Session 2.2 - Star System Map additions

    /// <summary>
    /// The currently selected celestial body (for UI display).
    /// Null if no body is selected.
    /// </summary>
    public Ulid? SelectedBodyId { get; init; }

    /// <summary>
    /// Navigation history stack (FILO - First In, Last Out).
    /// Used for back button navigation between screens.
    /// </summary>
    public Stack<ScreenId> NavigationStack { get; init; } = new();

    /// <summary>
    /// Per-system camera state persistence.
    /// Key: SystemId (as Ulid), Value: CameraState (pan/zoom).
    /// </summary>
    public Dictionary<Ulid, CameraState> CameraStates { get; init; } = new();

    /// <summary>
    /// Whether the system overview panel is open.
    /// </summary>
    public bool SystemOverviewPanelOpen { get; init; } = false;

    /// <summary>
    /// Current game speed multiplier.
    /// </summary>
    public GameSpeed CurrentSpeed { get; init; } = GameSpeed.Normal;

    /// <summary>
    /// Whether the game is paused.
    /// </summary>
    public bool IsPaused { get; init; } = false;

    // NEW: Phase 3 - Colony Landing & Establishment

    /// <summary>
    /// All ships in the game (colony ships, supply ships, etc.).
    /// </summary>
    public List<Ship> Ships { get; init; } = new();

    /// <summary>
    /// All established colonies.
    /// </summary>
    public List<Colony> Colonies { get; init; } = new();

    /// <summary>
    /// All colonists in the game (across all ships and colonies).
    /// </summary>
    public List<Colonist> Colonists { get; init; } = new();

    /// <summary>
    /// Surface maps for explored celestial bodies.
    /// Key: BodyId
    /// </summary>
    public Dictionary<Ulid, SurfaceMap> SurfaceMaps { get; init; } = new();

    /// <summary>
    /// All vehicles in the game.
    /// </summary>
    public List<Vehicle> Vehicles { get; init; } = new();

    /// <summary>
    /// All robots in the game.
    /// </summary>
    public List<Robot> Robots { get; init; } = new();

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

    // NEW: Phase 3 - Helper methods for ships and colonies

    /// <summary>
    /// Add a new ship to the game state.
    /// </summary>
    public GameState WithShipAdded(Ship ship)
    {
        var newShips = new List<Ship>(Ships) { ship };
        return this with { Ships = newShips };
    }

    /// <summary>
    /// Update an existing ship.
    /// </summary>
    public GameState WithShipUpdated(Ship ship)
    {
        var newShips = new List<Ship>();
        foreach (var s in Ships)
        {
            newShips.Add(s.Id == ship.Id ? ship : s);
        }
        return this with { Ships = newShips };
    }

    /// <summary>
    /// Add a new colony to the game state.
    /// </summary>
    public GameState WithColonyAdded(Colony colony)
    {
        var newColonies = new List<Colony>(Colonies) { colony };
        return this with { Colonies = newColonies };
    }

    /// <summary>
    /// Update an existing colony.
    /// </summary>
    public GameState WithColonyUpdated(Colony colony)
    {
        var newColonies = new List<Colony>();
        foreach (var c in Colonies)
        {
            newColonies.Add(c.Id == colony.Id ? colony : c);
        }
        return this with { Colonies = newColonies };
    }

    /// <summary>
    /// Add a surface map for a body.
    /// </summary>
    public GameState WithSurfaceMapAdded(Ulid bodyId, SurfaceMap surfaceMap)
    {
        var newMaps = new Dictionary<Ulid, SurfaceMap>(SurfaceMaps)
        {
            [bodyId] = surfaceMap
        };
        return this with { SurfaceMaps = newMaps };
    }

    /// <summary>
    /// Update a surface map.
    /// </summary>
    public GameState WithSurfaceMapUpdated(Ulid bodyId, SurfaceMap surfaceMap)
    {
        var newMaps = new Dictionary<Ulid, SurfaceMap>(SurfaceMaps)
        {
            [bodyId] = surfaceMap
        };
        return this with { SurfaceMaps = newMaps };
    }

    /// <summary>
    /// Add colonists to the game state.
    /// </summary>
    public GameState WithColonistsAdded(List<Colonist> colonists)
    {
        var newColonists = new List<Colonist>(Colonists);
        newColonists.AddRange(colonists);
        return this with { Colonists = newColonists };
    }

    /// <summary>
    /// Update a colonist.
    /// </summary>
    public GameState WithColonistUpdated(Colonist colonist)
    {
        var newColonists = new List<Colonist>();
        foreach (var c in Colonists)
        {
            newColonists.Add(c.Id == colonist.Id ? colonist : c);
        }
        return this with { Colonists = newColonists };
    }

    /// <summary>
    /// Add vehicles to the game state.
    /// </summary>
    public GameState WithVehiclesAdded(List<Vehicle> vehicles)
    {
        var newVehicles = new List<Vehicle>(Vehicles);
        newVehicles.AddRange(vehicles);
        return this with { Vehicles = newVehicles };
    }

    /// <summary>
    /// Add robots to the game state.
    /// </summary>
    public GameState WithRobotsAdded(List<Robot> robots)
    {
        var newRobots = new List<Robot>(Robots);
        newRobots.AddRange(robots);
        return this with { Robots = newRobots };
    }
}