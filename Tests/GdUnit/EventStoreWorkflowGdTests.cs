using System;
using System.IO;
using System.Linq;
using GdUnit4;
using Outpost3.Core.Events;
using Outpost3.Core.Persistence;

namespace Outpost3.Tests.GdUnit;

/// <summary>
/// GdUnit4 workflow tests for the event store system.
/// Tests complete user workflows without Godot dependencies using GdUnit4 framework.
/// </summary>
[TestSuite]
public class EventStoreWorkflowGdTests
{
    /// <summary>
    /// Creates a unique test file path for a test method.
    /// </summary>
    private string CreateUniqueTestFilePath()
    {
        return Path.Combine(Path.GetTempPath(), $"gdunit_workflow_{Guid.NewGuid()}.log");
    }

    /// <summary>
    /// Cleans up a test file if it exists.
    /// </summary>
    private void CleanupTestFile(string filePath)
    {
        if (File.Exists(filePath))
        {
            try
            {
                File.Delete(filePath);
            }
            catch
            {
                // Ignore cleanup errors
            }
        }
    }

    [TestCase]
    public void CompleteJourney_AllEventsPersistedCorrectly()
    {
        // Arrange: Create a complete journey scenario
        var testFilePath = CreateUniqueTestFilePath();
        var eventStore = new FileEventStore(testFilePath);
        
        try
        {
            var destinationId = Ulid.NewUlid();
            var events = new GameEvent[]
            {
                new ShipDepartedEvent(destinationId, "Starseeker", 150) { GameTime = 0f },
                new MechanicalFailureEvent("Life Support", "Warning", "Coolant leak detected") { GameTime = 5000f },
                new SocialConflictEvent("Resource Dispute", "Disagreement over water rationing", -0.2f) { GameTime = 10000f },
                new AnomalyDetectedEvent { GameTime = 15000f },
                new ShipArrivedEvent(destinationId, "Alpha Centauri", 25000f) { GameTime = 25000f }
            };

            // Act: Append all events
            eventStore.Append(events);

            // Assert
            var stored = eventStore.ReadFrom(0).ToList();
            Assertions.AssertThat(stored.Count).IsEqual(5);
            
            // Verify first and last events
            var departure = stored[0] as ShipDepartedEvent;
            Assertions.AssertThat(departure).IsNotNull();
            Assertions.AssertThat(departure!.ShipName).IsEqual("Starseeker");
            Assertions.AssertThat(departure.ColonistCount).IsEqual(150);

            var arrival = stored[4] as ShipArrivedEvent;
            Assertions.AssertThat(arrival).IsNotNull();
            Assertions.AssertThat(arrival!.SystemName).IsEqual("Alpha Centauri");
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }

    [TestCase]
    public void FilterByEventType_ReturnsCorrectSubset()
    {
        // Arrange
        var testFilePath = CreateUniqueTestFilePath();
        var eventStore = new FileEventStore(testFilePath);
        
        try
        {
            var systemId = Ulid.NewUlid();
            var events = new GameEvent[]
            {
                new ShipDepartedEvent(systemId, "Ship1", 100) { GameTime = 0f },
                new MechanicalFailureEvent("Engine", "Critical", "Engine failure") { GameTime = 1000f },
                new ShipArrivedEvent(systemId, "Mars", 2000f) { GameTime = 2000f },
                new ProbeArrived { GameTime = 3000f }
            };

            eventStore.Append(events);

            // Act: Filter for ship events
            var shipEvents = eventStore.ReadFrom(0)
                .Where(e => e is ShipDepartedEvent or ShipArrivedEvent)
                .ToList();

            // Assert
            Assertions.AssertThat(shipEvents.Count).IsEqual(2);
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }

    [TestCase]
    public void FilterBySeverity_ReturnsHighPriorityEvents()
    {
        // Arrange
        var testFilePath = CreateUniqueTestFilePath();
        var eventStore = new FileEventStore(testFilePath);
        
        try
        {
            var events = new GameEvent[]
            {
                new MechanicalFailureEvent("C1", "Minor", "Minor issue") { GameTime = 0f },
                new MechanicalFailureEvent("C2", "Warning", "Warning issue") { GameTime = 1000f },
                new MechanicalFailureEvent("C3", "Critical", "Critical issue") { GameTime = 2000f }
            };

            eventStore.Append(events);

            // Act: Get warning+ events
            var highPriority = eventStore.ReadFrom(0)
                .Where(e =>
                {
                    if (e is MechanicalFailureEvent failure)
                        return failure.Severity != "Minor";
                    return false;
                })
                .ToList();

            // Assert
            Assertions.AssertThat(highPriority.Count).IsEqual(2);
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }

    [TestCase]
    public void JsonSerialization_PreservesEventData()
    {
        // Arrange
        var testFilePath = CreateUniqueTestFilePath();
        var eventStore = new FileEventStore(testFilePath);
        
        try
        {
            var systemId = Ulid.NewUlid();
            var events = new GameEvent[]
            {
                new ShipDepartedEvent(systemId, "TestShip", 100) { GameTime = 0f },
                new ShipArrivedEvent(systemId, "Dest", 5000f) { GameTime = 5000f }
            };

            eventStore.Append(events);

            // Act
            var stored = eventStore.ReadFrom(0).ToList();
            
            // Assert - verify events were persisted and can be read back
            Assertions.AssertThat(stored.Count).IsEqual(2);
            
            var departure = stored[0] as ShipDepartedEvent;
            Assertions.AssertThat(departure).IsNotNull();
            Assertions.AssertThat(departure!.ShipName).IsEqual("TestShip");
            
            var arrival = stored[1] as ShipArrivedEvent;
            Assertions.AssertThat(arrival).IsNotNull();
            Assertions.AssertThat(arrival!.SystemName).IsEqual("Dest");
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }

    [TestCase]
    public void ChronologicalOrder_MaintainedAcrossReads()
    {
        // Arrange & Act
        var testFilePath = CreateUniqueTestFilePath();
        var eventStore = new FileEventStore(testFilePath);
        
        try
        {
            var times = new[] { 0f, 5000f, 10000f, 15000f };
            var eventArray = times.Select((time, i) => 
                new ShipDepartedEvent(Ulid.NewUlid(), $"Ship{i}", 50) { GameTime = time } as GameEvent
            ).ToArray();
            
            eventStore.Append(eventArray);

            // Assert
            var stored = eventStore.ReadFrom(0).ToList();
            for (int i = 1; i < stored.Count; i++)
            {
                Assertions.AssertThat(stored[i].GameTime)
                    .IsGreater(stored[i - 1].GameTime)
                    .OverrideFailureMessage($"Event {i} should have later game time than event {i - 1}");
            }
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }
}
