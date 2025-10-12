using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using Xunit;
using Outpost3.Core.Events;
using Outpost3.Core.Persistence;

namespace Outpost3.Tests;

/// <summary>
/// Workflow tests for the event store system.
/// Tests complete user workflows without Godot dependencies.
/// </summary>
public class EventStoreWorkflowTests : IDisposable
{
    private readonly string _testFilePath;
    private readonly FileEventStore _eventStore;

    public EventStoreWorkflowTests()
    {
        _testFilePath = Path.Combine(Path.GetTempPath(), $"workflow_test_{Guid.NewGuid()}.log");
        _eventStore = new FileEventStore(_testFilePath);
    }

    public void Dispose()
    {
        if (File.Exists(_testFilePath))
        {
            File.Delete(_testFilePath);
        }
    }

    [Fact]
    public void CompleteJourney_AllEventsPersistedCorrectly()
    {
        // Arrange: Create a complete journey scenario
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
        _eventStore.Append(events);

        // Assert
        var stored = _eventStore.ReadFrom(0).ToList();
        Assert.Equal(5, stored.Count);
        
        // Verify first and last events
        var departure = stored[0] as ShipDepartedEvent;
        Assert.NotNull(departure);
        Assert.Equal("Starseeker", departure.ShipName);
        Assert.Equal(150, departure.ColonistCount);

        var arrival = stored[4] as ShipArrivedEvent;
        Assert.NotNull(arrival);
        Assert.Equal("Alpha Centauri", arrival.SystemName);
    }

    [Fact]
    public void FilterByEventType_ReturnsCorrectSubset()
    {
        // Arrange
        var systemId = Ulid.NewUlid();
        var events = new GameEvent[]
        {
            new ShipDepartedEvent(systemId, "Ship1", 100) { GameTime = 0f },
            new MechanicalFailureEvent("Engine", "Critical", "Engine failure") { GameTime = 1000f },
            new ShipArrivedEvent(systemId, "Mars", 2000f) { GameTime = 2000f },
            new ProbeArrived { GameTime = 3000f }
        };

        _eventStore.Append(events);

        // Act: Filter for ship events
        var shipEvents = _eventStore.ReadFrom(0)
            .Where(e => e is ShipDepartedEvent or ShipArrivedEvent)
            .ToList();

        // Assert
        Assert.Equal(2, shipEvents.Count);
    }

    [Fact]
    public void FilterBySeverity_ReturnsHighPriorityEvents()
    {
        // Arrange
        var events = new GameEvent[]
        {
            new MechanicalFailureEvent("C1", "Minor", "Minor issue") { GameTime = 0f },
            new MechanicalFailureEvent("C2", "Warning", "Warning issue") { GameTime = 1000f },
            new MechanicalFailureEvent("C3", "Critical", "Critical issue") { GameTime = 2000f }
        };

        _eventStore.Append(events);

        // Act: Get warning+ events
        var highPriority = _eventStore.ReadFrom(0)
            .Where(e =>
            {
                if (e is MechanicalFailureEvent failure)
                    return failure.Severity != "Minor";
                return false;
            })
            .ToList();

        // Assert
        Assert.Equal(2, highPriority.Count);
    }

    [Fact]
    public void JsonSerialization_PreservesEventData()
    {
        // Arrange
        var systemId = Ulid.NewUlid();
        var events = new GameEvent[]
        {
            new ShipDepartedEvent(systemId, "TestShip", 100) { GameTime = 0f },
            new ShipArrivedEvent(systemId, "Dest", 5000f) { GameTime = 5000f }
        };

        _eventStore.Append(events);

        // Act
        var stored = _eventStore.ReadFrom(0).ToList();
        
        // Assert - verify events were persisted and can be read back
        Assert.Equal(2, stored.Count);
        
        var departure = stored[0] as ShipDepartedEvent;
        Assert.NotNull(departure);
        Assert.Equal("TestShip", departure.ShipName);
        
        var arrival = stored[1] as ShipArrivedEvent;
        Assert.NotNull(arrival);
        Assert.Equal("Dest", arrival.SystemName);
    }

    [Fact]
    public void ChronologicalOrder_MaintainedAcrossReads()
    {
        // Arrange & Act
        var times = new[] { 0f, 5000f, 10000f, 15000f };
        var eventArray = times.Select((time, i) => 
            new ShipDepartedEvent(Ulid.NewUlid(), $"Ship{i}", 50) { GameTime = time } as GameEvent
        ).ToArray();
        
        _eventStore.Append(eventArray);

        // Assert
        var stored = _eventStore.ReadFrom(0).ToList();
        for (int i = 1; i < stored.Count; i++)
        {
            Assert.True(stored[i].GameTime > stored[i - 1].GameTime,
                $"Event {i} should have later game time than event {i - 1}");
        }
    }
}
