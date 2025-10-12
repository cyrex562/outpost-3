using System;
using System.IO;
using System.Linq;
using Xunit;
using Outpost3.Core.Events;
using Outpost3.Core.Persistence;

namespace Outpost3.Tests;

/// <summary>
/// Unit tests for FileEventStore implementation.
/// Tests event persistence, reading, and offset management.
/// </summary>
public class FileEventStoreTests : IDisposable
{
    private string _testFilePath;

    public FileEventStoreTests()
    {
        // Create unique temp file path for each test
        _testFilePath = Path.Combine(Path.GetTempPath(), $"test_events_{Guid.NewGuid()}.log");
    }

    public void Dispose()
    {
        // Clean up test file
        if (File.Exists(_testFilePath))
        {
            File.Delete(_testFilePath);
        }
    }

    [Fact]
    public void Append_SingleEvent_WritesToFile()
    {
        // Arrange
        var store = new FileEventStore(_testFilePath);
        var testEvent = new ShipDepartedEvent
        {
            DestinationSystemId = Ulid.NewUlid(),
            ShipName = "TestShip",
            ColonistCount = 100
        };

        // Act
        var startOffset = store.Append(testEvent);

        // Assert
        Assert.True(File.Exists(_testFilePath));
        var lines = File.ReadAllLines(_testFilePath);
        Assert.Single(lines);
        Assert.Equal(0, startOffset);
        Assert.Equal(0, store.CurrentOffset);
    }

    [Fact]
    public void Append_MultipleEvents_AssignsSequentialOffsets()
    {
        // Arrange
        var store = new FileEventStore(_testFilePath);
        var event1 = new ShipDepartedEvent
        {
            DestinationSystemId = Ulid.NewUlid(),
            ShipName = "Ship1",
            ColonistCount = 50
        };
        var event2 = new MechanicalFailureEvent
        {
            SystemAffected = "Engine",
            Severity = "Minor",
            Description = "Coolant leak"
        };
        var event3 = new SocialConflictEvent
        {
            ConflictType = "Dispute",
            Description = "Resource allocation disagreement",
            MoraleImpact = -0.1f
        };

        // Act
        store.Append(event1);
        store.Append(event2);
        store.Append(event3);

        // Assert
        var lines = File.ReadAllLines(_testFilePath);
        Assert.Equal(3, lines.Length);
        Assert.Equal(2, store.CurrentOffset);

        // Verify offsets in file
        Assert.StartsWith("0|", lines[0]);
        Assert.StartsWith("1|", lines[1]);
        Assert.StartsWith("2|", lines[2]);
    }

    [Fact]
    public void ReadFrom_ValidOffset_ReturnsCorrectEvents()
    {
        // Arrange
        var store = new FileEventStore(_testFilePath);
        var events = new GameEvent[]
        {
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship0", ColonistCount = 10 },
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship1", ColonistCount = 20 },
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship2", ColonistCount = 30 },
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship3", ColonistCount = 40 },
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship4", ColonistCount = 50 }
        };
        store.Append(events);

        // Act
        var result = store.ReadFrom(2).ToList();

        // Assert
        Assert.Equal(3, result.Count); // Should return events at offsets 2, 3, 4
        Assert.Equal(2, result[0].Offset);
        Assert.Equal(3, result[1].Offset);
        Assert.Equal(4, result[2].Offset);

        // Verify content
        var ship2 = result[0] as ShipDepartedEvent;
        Assert.NotNull(ship2);
        Assert.Equal("Ship2", ship2.ShipName);
        Assert.Equal(30, ship2.ColonistCount);
    }

    [Fact]
    public void ReadFrom_ZeroOffset_ReturnsAllEvents()
    {
        // Arrange
        var store = new FileEventStore(_testFilePath);
        var events = new GameEvent[]
        {
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship1", ColonistCount = 100 },
            new MechanicalFailureEvent { SystemAffected = "Engine", Severity = "Minor", Description = "Test" },
            new SocialConflictEvent { ConflictType = "Test", Description = "Test", MoraleImpact = -0.1f }
        };
        store.Append(events);

        // Act
        var result = store.ReadFrom(0).ToList();

        // Assert
        Assert.Equal(3, result.Count);
        Assert.Equal(0, result[0].Offset);
        Assert.Equal(1, result[1].Offset);
        Assert.Equal(2, result[2].Offset);
    }

    [Fact]
    public void ReadFrom_OffsetBeyondEnd_ReturnsEmpty()
    {
        // Arrange
        var store = new FileEventStore(_testFilePath);
        store.Append(
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship1", ColonistCount = 100 },
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship2", ColonistCount = 200 }
        );

        // Act
        var result = store.ReadFrom(10).ToList();

        // Assert
        Assert.Empty(result);
    }

    [Fact]
    public void Constructor_ExistingFile_RestoresOffset()
    {
        // Arrange - Create store and add events
        var store1 = new FileEventStore(_testFilePath);
        store1.Append(
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship1", ColonistCount = 100 },
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship2", ColonistCount = 200 },
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship3", ColonistCount = 300 }
        );

        // Act - Create new store with same file
        var store2 = new FileEventStore(_testFilePath);

        // Assert
        Assert.Equal(2, store2.CurrentOffset);
        Assert.Equal(3, store2.Count);

        // Verify can read all events
        var events = store2.ReadFrom(0).ToList();
        Assert.Equal(3, events.Count);
    }

    [Fact]
    public void Count_ReturnsCorrectCount()
    {
        // Arrange
        var store = new FileEventStore(_testFilePath);

        // Act & Assert - Empty store
        Assert.Equal(0, store.Count);

        // Add events
        store.Append(
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship1", ColonistCount = 100 }
        );
        Assert.Equal(1, store.Count);

        store.Append(
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship2", ColonistCount = 200 },
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship3", ColonistCount = 300 }
        );
        Assert.Equal(3, store.Count);
    }

    [Fact]
    public void Append_CorruptedLine_ThrowsException()
    {
        // Arrange - Create store with valid event
        var store = new FileEventStore(_testFilePath);
        store.Append(new ShipDepartedEvent 
        { 
            DestinationSystemId = Ulid.NewUlid(), 
            ShipName = "Ship1", 
            ColonistCount = 100 
        });

        // Corrupt the file by appending invalid data
        File.AppendAllText(_testFilePath, "CORRUPTED|DATA|HERE|{invalid json}\n");

        // Act & Assert - Reading should throw EventStoreException
        var newStore = new FileEventStore(_testFilePath);
        Assert.Throws<EventStoreException>(() => newStore.ReadFrom(0).ToList());
    }

    [Fact]
    public void ReadFrom_PreservesEventTypes()
    {
        // Arrange
        var store = new FileEventStore(_testFilePath);
        var events = new GameEvent[]
        {
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship1", ColonistCount = 100 },
            new MechanicalFailureEvent { SystemAffected = "Engine", Severity = "Critical", Description = "Total failure" },
            new SocialConflictEvent { ConflictType = "Fight", Description = "Crew dispute", MoraleImpact = -0.5f },
            new AnomalyDetectedEvent { AnomalyType = "Spatial", Description = "Wormhole detected" },
            new ShipArrivedEvent { SystemId = Ulid.NewUlid(), SystemName = "Alpha Centauri", TravelDuration = 100.5f }
        };
        store.Append(events);

        // Act
        var result = store.ReadFrom(0).ToList();

        // Assert
        Assert.Equal(5, result.Count);
        Assert.IsType<ShipDepartedEvent>(result[0]);
        Assert.IsType<MechanicalFailureEvent>(result[1]);
        Assert.IsType<SocialConflictEvent>(result[2]);
        Assert.IsType<AnomalyDetectedEvent>(result[3]);
        Assert.IsType<ShipArrivedEvent>(result[4]);
    }

    [Fact]
    public void Append_UpdatesCurrentOffset()
    {
        // Arrange
        var store = new FileEventStore(_testFilePath);

        // Act & Assert
        Assert.Equal(-1, store.CurrentOffset); // Empty store

        store.Append(new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship1", ColonistCount = 100 });
        Assert.Equal(0, store.CurrentOffset);

        store.Append(new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship2", ColonistCount = 200 });
        Assert.Equal(1, store.CurrentOffset);

        store.Append(
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship3", ColonistCount = 300 },
            new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship4", ColonistCount = 400 }
        );
        Assert.Equal(3, store.CurrentOffset);
    }
}
