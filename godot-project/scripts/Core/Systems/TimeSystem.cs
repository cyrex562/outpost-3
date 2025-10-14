using System.Collections.Generic;
using System.Linq;
using Outpost3.Core.Commands;
using Outpost3.Core.Domain;
using Outpost3.Core.Events;
using System;

namespace Outpost3.Core.Systems;

public static class TimeSystem
{
    public static (GameState, List<IGameEvent>) Reduce(GameState state, Commands.ICommand command)
    {
        return command switch
        {
            // Time & Core
            AdvanceTime cmd => ReduceAdvanceTime(state, cmd),
            LaunchProbe cmd => ReduceLaunchProbe(state, cmd),
            InitializeGalaxy cmd => ReduceInitializeGalaxy(state, cmd),

            // System Selection (legacy)
            SelectSystemCommand cmd => SystemSelectionSystem.HandleSelectSystem(state, cmd),

            // Navigation
            PushScreen cmd => NavigationReducer.HandlePushScreen(state, cmd),
            PopScreen cmd => NavigationReducer.HandlePopScreen(state, cmd),
            NavigateToScreen cmd => NavigationReducer.HandleNavigateToScreen(state, cmd),

            // Star System
            GenerateSystemDetails cmd => StarSystemReducer.HandleGenerateSystemDetails(state, cmd),
            SelectCelestialBody cmd => StarSystemReducer.HandleSelectCelestialBody(state, cmd),
            UpdateCamera cmd => StarSystemReducer.HandleUpdateCamera(state, cmd),
            ResetCamera cmd => StarSystemReducer.HandleResetCamera(state, cmd),
            ToggleSystemOverviewPanel cmd => StarSystemReducer.HandleToggleSystemOverviewPanel(state, cmd),
            SetGameSpeed cmd => StarSystemReducer.HandleSetGameSpeed(state, cmd),
            TogglePause cmd => StarSystemReducer.HandleTogglePause(state, cmd),

            _ => (state, new List<IGameEvent>())
        };
    }

    private static (GameState, List<IGameEvent>) ReduceAdvanceTime(
        GameState state,
        AdvanceTime command)
    {
        var newTime = state.GameTime + command.Dt;
        var events = new List<IGameEvent>
        {
            new TimeAdvanced(command.Dt) { GameTime = (float)newTime }
        };

        var newState = state.WithAdvanceTime(command.Dt);

        var arrivedProbeIds = new List<Ulid>();
        foreach (var probe in state.ProbesInFlight)
        {
            if (probe.ArrivalTime <= newTime)
            {
                arrivedProbeIds.Add(probe.Id);
                events.Add(new ProbeArrived(probe.Id, probe.TargetSystemId) { GameTime = (float)newTime });

                // Handle probe arrival: scan the system
                var (scannedState, scanEvents) = HandleProbeArrival(newState, probe);
                newState = scannedState;
                events.AddRange(scanEvents);
            }
        }

        // remove arrived probes
        if (arrivedProbeIds.Count > 0)
        {
            newState = newState.WithProbesRemoved(arrivedProbeIds);
        }

        return (newState, events);
    }

    private static (GameState, List<IGameEvent>) ReduceLaunchProbe(
        GameState state,
        LaunchProbe command
    )
    {
        // Find the target system to get its distance from Sol
        var targetSystem = state.Systems.FirstOrDefault(s => s.Id == command.TargetSystemId);
        if (targetSystem == null)
        {
            // If system not found, use a default travel time (shouldn't happen in normal gameplay)
            var defaultTravelTime = PhysicsConstants.CalculateProbeTraverTime(10.0); // 10 light-years default
            var defaultArrivalTime = state.GameTime + defaultTravelTime;

            var defaultState = state.WithProbeLaunched(command.TargetSystemId, defaultArrivalTime, out var defaultProbeId);
            var defaultEvents = new List<IGameEvent>
            {
                new ProbeLaunched(defaultProbeId, command.TargetSystemId, defaultArrivalTime)
                { GameTime = (float)state.GameTime }
            };
            return (defaultState, defaultEvents);
        }

        // Calculate realistic travel time based on distance and 0.9c probe speed
        var distanceLightYears = (double)targetSystem.DistanceFromSol;
        var travelTimeHours = PhysicsConstants.CalculateProbeTraverTime(distanceLightYears);
        var arrivalTime = state.GameTime + travelTimeHours;

        var newState = state.WithProbeLaunched(
            command.TargetSystemId,
            arrivalTime,
            out var probeId
        );

        var events = new List<IGameEvent>
        {
            new ProbeLaunched(probeId, command.TargetSystemId, arrivalTime)
            {
                GameTime = (float)state.GameTime
            }
        };

        return (newState, events);
    }

    private static (GameState, List<IGameEvent>) ReduceInitializeGalaxy(
        GameState state,
        InitializeGalaxy command)
    {
        var systems = GalaxyGenerationSystem.GenerateGalaxy(command.Seed, command.StarCount);
        var newState = state.WithGalaxyInitialized(systems);

        var events = new List<IGameEvent>
        {
            new GalaxyInitialized(systems.Count, command.Seed) { GameTime = (float)state.GameTime }
        };

        return (newState, events);
    }

    private static (GameState, List<IGameEvent>) HandleProbeArrival(GameState state, ProbeInFlight probe)
    {
        var events = new List<IGameEvent>();

        // Find existing system (should be Detected level from galaxy initialization)
        var existingSystem = state.Systems.Find(s => s.Id == probe.TargetSystemId);

        if (existingSystem == null)
        {
            // System not found - shouldn't happen, but defensive
            // Generate new system as fallback (old behavior)
            var newSystem = GenerateSystemForUnknownTarget(probe.TargetSystemId);
            var newState = state.WithSystemDiscovered(newSystem);
            events.Add(new SystemDiscovered(newSystem.Id, newSystem.Name) { GameTime = (float)state.GameTime });
            return (newState, events);
        }

        // Re-scan the system: potentially re-roll characteristics
        var scannedSystem = RescanStarSystem(existingSystem, state.GameTime);

        // Generate bodies with partial discovery
        var bodiesWithPartialInfo = GenerateBodiesWithPartialDiscovery(scannedSystem.Id, (int)state.GameTime);

        // Update system to Scanned with bodies
        var updatedSystem = scannedSystem with
        {
            DiscoveryLevel = DiscoveryLevel.Scanned,
            Bodies = bodiesWithPartialInfo
        };

        // Update in state
        var resultState = state.WithSystemUpdated(updatedSystem);

        events.Add(new SystemScanned(updatedSystem.Id, updatedSystem.Name) { GameTime = (float)state.GameTime });

        return (resultState, events);
    }

    private static StarSystem RescanStarSystem(StarSystem existingSystem, double gameTime)
    {
        // Small chance to re-roll characteristics (5% chance)
        var rng = new Random((int)gameTime + existingSystem.Id.GetHashCode());

        if (rng.NextDouble() < 0.05)
        {
            // Re-roll spectral class (star changed or initial detection was wrong)
            var spectralClasses = new[] { "O", "B", "A", "F", "G", "K", "M" };
            var newSpectralClass = spectralClasses[rng.Next(spectralClasses.Length)];

            // Recalculate luminosity
            var newLuminosity = GalaxyGenerationSystem.CalculateLuminosity(newSpectralClass, rng);

            return existingSystem with
            {
                SpectralClass = newSpectralClass,
                Luminosity = newLuminosity
            };
        }

        return existingSystem;
    }

    private static List<CelestialBody> GenerateBodiesWithPartialDiscovery(Ulid systemId, int seed)
    {
        var rng = new Random(seed + systemId.GetHashCode());
        var bodies = new List<CelestialBody>();

        var bodyTypes = new[] { "Planet", "Moon", "Asteroid Belt", "Cometary Belt" };
        var compositions = new[] { "Rocky", "Gas Giant", "Ice Giant", "Asteroid", "Comet", "Unknown" };
        var atmosphereTypes = new[] { "None", "Thin CO2", "Thick CO2", "Nitrogen-Oxygen", "Hydrogen-Helium", "Toxic", "Unknown" };
        var surfaceTypes = new[] { "Barren", "Cratered", "Terrestrial", "Ice", "Lava", "Desert", "Ocean", "Unknown" };

        int bodyCount = rng.Next(0, 12); // 0-11 bodies (not including star)

        for (int i = 0; i < bodyCount; i++)
        {
            var bodyType = bodyTypes[rng.Next(bodyTypes.Length)];
            var composition = compositions[rng.Next(compositions.Length)];

            // 30% chance to discover atmosphere type
            string? atmosphereType = null;
            if (rng.NextDouble() < 0.3)
            {
                atmosphereType = atmosphereTypes[rng.Next(atmosphereTypes.Length)];
            }

            // 30% chance to discover surface type
            string? surfaceType = null;
            if (rng.NextDouble() < 0.3)
            {
                surfaceType = surfaceTypes[rng.Next(surfaceTypes.Length)];
            }

            bodies.Add(new CelestialBody
            {
                Id = Ulid.NewUlid(),
                Name = $"Body {i + 1}",
                BodyType = bodyType,
                Composition = composition,
                Explored = false,
                AtmosphereType = atmosphereType,
                SurfaceType = surfaceType
            });
        }

        return bodies;
    }

    private static StarSystem GenerateSystemForUnknownTarget(Ulid id)
    {
        // Fallback for when probe targets unknown system (shouldn't happen in normal flow)
        Random rand = new Random(id.GetHashCode());

        var spectralClasses = new[] { "O", "B", "A", "F", "G", "K", "M" };
        var spectralClass = spectralClasses[rand.Next(spectralClasses.Length)];

        return new StarSystem
        {
            Id = id,
            Name = $"Unknown-{id.ToString().Substring(0, 6)}",
            SpectralClass = spectralClass,
            Luminosity = 1.0f,
            DiscoveryLevel = DiscoveryLevel.Scanned,
            Bodies = new List<CelestialBody>()
        };
    }
}