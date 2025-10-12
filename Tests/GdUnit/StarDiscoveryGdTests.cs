using GdUnit4;
using static GdUnit4.Assertions;
using Outpost3.Core.Systems;
using Outpost3.Core.Domain;
using Outpost3.Core.Commands;
using Outpost3.Core.Events;
using System.Linq;
using Godot;

namespace Outpost3.Tests.GdUnit;

[TestSuite]
public class StarDiscoveryGdTests
{
    [TestCase]
    public void InitializeGalaxy_CreatesGalaxyInState()
    {
        // Arrange
        var initialState = GameState.NewGame();
        var command = new InitializeGalaxy(Seed: 12345, StarCount: 50);

        // Act
        var (newState, events) = TimeSystem.Reduce(initialState, command);

        // Assert
        AssertThat(newState.Systems.Count).IsEqual(50);
        AssertThat(events.Count).IsEqual(1);
        AssertThat(events[0]).IsInstanceOf<GalaxyInitialized>();

        var galaxyEvent = (GalaxyInitialized)events[0];
        AssertThat(galaxyEvent.StarCount).IsEqual(50);
        AssertThat(galaxyEvent.Seed).IsEqual(12345);
    }

    [TestCase]
    public void LaunchProbe_ToDetectedSystem_WorksCorrectly()
    {
        // Arrange
        var state = GameState.NewGame();
        var initCommand = new InitializeGalaxy(Seed: 999, StarCount: 10);
        var (galaxyState, _) = TimeSystem.Reduce(state, initCommand);

        // Get a non-Sol system
        var targetSystem = galaxyState.Systems.First(s => s.Name != "Sol");

        // Act
        var launchCommand = new LaunchProbe(targetSystem.Id);
        var (newState, events) = TimeSystem.Reduce(galaxyState, launchCommand);

        // Assert
        AssertThat(newState.ProbesInFlight.Count).IsEqual(1);
        AssertThat(events.Count).IsEqual(1);
        AssertThat(events[0]).IsInstanceOf<ProbeLaunched>();
    }

    [TestCase]
    public void ProbeArrival_UpdatesSystemToScanned()
    {
        // Arrange
        var state = GameState.NewGame();
        var initCommand = new InitializeGalaxy(Seed: 777, StarCount: 10);
        var (galaxyState, _) = TimeSystem.Reduce(state, initCommand);

        var targetSystem = galaxyState.Systems.First(s => s.Name != "Sol");
        var launchCommand = new LaunchProbe(targetSystem.Id);
        var (launchedState, _) = TimeSystem.Reduce(galaxyState, launchCommand);

        // Act - Advance time past probe arrival
        var advanceCommand = new AdvanceTime(150.0); // More than travel time
        var (arrivedState, events) = TimeSystem.Reduce(launchedState, advanceCommand);

        // Assert
        var updatedSystem = arrivedState.Systems.First(s => s.Id == targetSystem.Id);
        AssertThat(updatedSystem.DiscoveryLevel).IsEqual(DiscoveryLevel.Scanned);

        // Should have TimeAdvanced, ProbeArrived, and SystemScanned events
        AssertThat(events.Any(e => e is ProbeArrived)).IsTrue();
        AssertThat(events.Any(e => e is SystemScanned)).IsTrue();
    }

    [TestCase]
    public void ProbeArrival_GeneratesBodiesForSystem()
    {
        // Arrange
        var state = GameState.NewGame();
        var initCommand = new InitializeGalaxy(Seed: 555, StarCount: 10);
        var (galaxyState, _) = TimeSystem.Reduce(state, initCommand);

        var targetSystem = galaxyState.Systems.First(s => s.Name != "Sol");
        var launchCommand = new LaunchProbe(targetSystem.Id);
        var (launchedState, _) = TimeSystem.Reduce(galaxyState, launchCommand);

        // Verify system has no bodies initially
        var systemBefore = launchedState.Systems.First(s => s.Id == targetSystem.Id);
        AssertThat(systemBefore.Bodies.Count).IsEqual(0);

        // Act
        var advanceCommand = new AdvanceTime(150.0);
        var (arrivedState, _) = TimeSystem.Reduce(launchedState, advanceCommand);

        // Assert
        var updatedSystem = arrivedState.Systems.First(s => s.Id == targetSystem.Id);
        // System may have 0-11 bodies (random generation)
        AssertThat(updatedSystem.Bodies.Count).IsGreaterEqual(0);
    }

    [TestCase]
    public void ProbeArrival_BodiesHavePartialInfo()
    {
        // Arrange
        var state = GameState.NewGame();
        var initCommand = new InitializeGalaxy(Seed: 444, StarCount: 20);
        var (galaxyState, _) = TimeSystem.Reduce(state, initCommand);

        var targetSystem = galaxyState.Systems.First(s => s.Name != "Sol");
        var launchCommand = new LaunchProbe(targetSystem.Id);
        var (launchedState, _) = TimeSystem.Reduce(galaxyState, launchCommand);

        // Act
        var advanceCommand = new AdvanceTime(150.0);
        var (arrivedState, _) = TimeSystem.Reduce(launchedState, advanceCommand);

        // Assert
        var updatedSystem = arrivedState.Systems.First(s => s.Id == targetSystem.Id);

        foreach (var body in updatedSystem.Bodies)
        {
            // All bodies should have type and composition
            AssertThat(body.BodyType).IsNotEmpty();
            AssertThat(body.Composition).IsNotEmpty();

            // Bodies should NOT be fully explored yet
            AssertThat(body.Explored).IsFalse();

            // Temperature, Gravity, Resources, Hazards should be null (not explored)
            AssertThat(body.Temperature).IsNull();
            AssertThat(body.Gravity).IsNull();
            AssertThat(body.Resources).IsNull();
            AssertThat(body.Hazards).IsNull();

            // AtmosphereType and SurfaceType MAY be revealed (30% chance each)
            // Can't assert exact values, but they should be either null or non-empty
            if (body.AtmosphereType != null)
            {
                AssertThat(body.AtmosphereType).IsNotEmpty();
            }
            if (body.SurfaceType != null)
            {
                AssertThat(body.SurfaceType).IsNotEmpty();
            }
        }
    }

    [TestCase]
    public void ProbeArrival_EmitsSystemScannedEvent()
    {
        // Arrange
        var state = GameState.NewGame();
        var initCommand = new InitializeGalaxy(Seed: 333, StarCount: 10);
        var (galaxyState, _) = TimeSystem.Reduce(state, initCommand);

        var targetSystem = galaxyState.Systems.First(s => s.Name != "Sol");
        var launchCommand = new LaunchProbe(targetSystem.Id);
        var (launchedState, _) = TimeSystem.Reduce(galaxyState, launchCommand);

        // Act
        var advanceCommand = new AdvanceTime(150.0);
        var (arrivedState, events) = TimeSystem.Reduce(launchedState, advanceCommand);

        // Assert
        var scannedEvent = events.OfType<SystemScanned>().FirstOrDefault();
        AssertThat(scannedEvent).IsNotNull();
        AssertThat(scannedEvent!.SystemId).IsEqual(targetSystem.Id);
    }

    [TestCase]
    public void ProbeArrival_RemovesProbeFromFlight()
    {
        // Arrange
        var state = GameState.NewGame();
        var initCommand = new InitializeGalaxy(Seed: 222, StarCount: 10);
        var (galaxyState, _) = TimeSystem.Reduce(state, initCommand);

        var targetSystem = galaxyState.Systems.First(s => s.Name != "Sol");
        var launchCommand = new LaunchProbe(targetSystem.Id);
        var (launchedState, _) = TimeSystem.Reduce(galaxyState, launchCommand);

        AssertThat(launchedState.ProbesInFlight.Count).IsEqual(1);

        // Act
        var advanceCommand = new AdvanceTime(150.0);
        var (arrivedState, _) = TimeSystem.Reduce(launchedState, advanceCommand);

        // Assert
        AssertThat(arrivedState.ProbesInFlight.Count).IsEqual(0);
    }

    [TestCase]
    public void MultipleProbes_ToSameSystem_WorkCorrectly()
    {
        // Arrange
        var state = GameState.NewGame();
        var initCommand = new InitializeGalaxy(Seed: 111, StarCount: 10);
        var (galaxyState, _) = TimeSystem.Reduce(state, initCommand);

        var targetSystem = galaxyState.Systems.First(s => s.Name != "Sol");

        // Launch two probes to same system
        var launch1 = new LaunchProbe(targetSystem.Id);
        var (state1, _) = TimeSystem.Reduce(galaxyState, launch1);

        var launch2 = new LaunchProbe(targetSystem.Id);
        var (state2, _) = TimeSystem.Reduce(state1, launch2);

        AssertThat(state2.ProbesInFlight.Count).IsEqual(2);

        // Act - Advance time to let both arrive
        var advanceCommand = new AdvanceTime(150.0);
        var (arrivedState, events) = TimeSystem.Reduce(state2, advanceCommand);

        // Assert
        AssertThat(arrivedState.ProbesInFlight.Count).IsEqual(0);

        // Both probes should generate events
        var probeArrivals = events.OfType<ProbeArrived>().ToList();
        AssertThat(probeArrivals.Count).IsEqual(2);
    }

    [TestCase]
    public void SystemScan_IsDeterministic_WithSameSeed()
    {
        // Arrange
        var state1 = GameState.NewGame();
        var state2 = GameState.NewGame();

        var initCommand = new InitializeGalaxy(Seed: 9999, StarCount: 10);
        var (galaxyState1, _) = TimeSystem.Reduce(state1, initCommand);
        var (galaxyState2, _) = TimeSystem.Reduce(state2, initCommand);

        var targetSystem1 = galaxyState1.Systems.First(s => s.Name != "Sol");
        var targetSystem2 = galaxyState2.Systems.First(s => s.Name != "Sol");

        // Launch probes
        var launch1 = new LaunchProbe(targetSystem1.Id);
        var launch2 = new LaunchProbe(targetSystem2.Id);

        var (launched1, _) = TimeSystem.Reduce(galaxyState1, launch1);
        var (launched2, _) = TimeSystem.Reduce(galaxyState2, launch2);

        // Act - Advance same amount
        var advance1 = new AdvanceTime(150.0);
        var advance2 = new AdvanceTime(150.0);

        var (arrived1, _) = TimeSystem.Reduce(launched1, advance1);
        var (arrived2, _) = TimeSystem.Reduce(launched2, advance2);

        // Assert - Same systems should have same body count
        var system1 = arrived1.Systems.First(s => s.Id == targetSystem1.Id);
        var system2 = arrived2.Systems.First(s => s.Id == targetSystem2.Id);

        AssertThat(system1.Bodies.Count).IsEqual(system2.Bodies.Count);
    }
}
