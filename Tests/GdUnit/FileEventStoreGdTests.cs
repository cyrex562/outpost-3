using System;
using System.IO;
using System.Linq;
using GdUnit4;
using Outpost3.Core.Events;
using Outpost3.Core.Persistence;

namespace Outpost3.Tests.GdUnit;

/// <summary>
/// GdUnit4 tests for FileEventStore implementation.
/// Tests event persistence, reading, and offset management using GdUnit4 framework.
/// </summary>
[TestSuite]
public class FileEventStoreGdTests
{
    /// <summary>
    /// Creates a unique test file path for a test method.
    /// </summary>
    private string CreateUniqueTestFilePath()
    {
        return Path.Combine(Path.GetTempPath(), $"gdunit_test_{Guid.NewGuid()}.log");
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
    public void Append_SingleEvent_WritesToFile()
    {
        // Arrange
        var testFilePath = CreateUniqueTestFilePath();
        
        try
        {
            var store = new FileEventStore(testFilePath);
            var testEvent = new ShipDepartedEvent
            {
                DestinationSystemId = Ulid.NewUlid(),
                ShipName = "TestShip",
                ColonistCount = 100
            };

            // Act
            var startOffset = store.Append(testEvent);

            // Assert
            Assertions.AssertThat(File.Exists(testFilePath)).IsTrue();
            var lines = File.ReadAllLines(testFilePath);
            Assertions.AssertThat(lines.Length).IsEqual(1);
            Assertions.AssertThat(startOffset).IsEqual(0);
            Assertions.AssertThat(store.CurrentOffset).IsEqual(0);
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }

    [TestCase]
    public void Append_MultipleEvents_AssignsSequentialOffsets()
    {
        // Arrange
        var testFilePath = CreateUniqueTestFilePath();
        
        try
        {
            var store = new FileEventStore(testFilePath);
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
            var lines = File.ReadAllLines(testFilePath);
            Assertions.AssertThat(lines.Length).IsEqual(3);
            Assertions.AssertThat(store.CurrentOffset).IsEqual(2);

            // Verify offsets in file
            Assertions.AssertThat(lines[0]).StartsWith("0|");
            Assertions.AssertThat(lines[1]).StartsWith("1|");
            Assertions.AssertThat(lines[2]).StartsWith("2|");
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }

    [TestCase]
    public void ReadFrom_ValidOffset_ReturnsCorrectEvents()
    {
        // Arrange
        var testFilePath = CreateUniqueTestFilePath();
        
        try
        {
            var store = new FileEventStore(testFilePath);
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
            Assertions.AssertThat(result.Count).IsEqual(3); // Should return events at offsets 2, 3, 4
            Assertions.AssertThat(result[0].Offset).IsEqual(2);
            Assertions.AssertThat(result[1].Offset).IsEqual(3);
            Assertions.AssertThat(result[2].Offset).IsEqual(4);

            // Verify content
            var ship2 = result[0] as ShipDepartedEvent;
            Assertions.AssertThat(ship2).IsNotNull();
            Assertions.AssertThat(ship2!.ShipName).IsEqual("Ship2");
            Assertions.AssertThat(ship2.ColonistCount).IsEqual(30);
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }

    [TestCase]
    public void ReadFrom_ZeroOffset_ReturnsAllEvents()
    {
        // Arrange
        var testFilePath = CreateUniqueTestFilePath();
        
        try
        {
            var store = new FileEventStore(testFilePath);
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
            Assertions.AssertThat(result.Count).IsEqual(3);
            Assertions.AssertThat(result[0].Offset).IsEqual(0);
            Assertions.AssertThat(result[1].Offset).IsEqual(1);
            Assertions.AssertThat(result[2].Offset).IsEqual(2);
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }

    [TestCase]
    public void ReadFrom_OffsetBeyondEnd_ReturnsEmpty()
    {
        // Arrange
        var testFilePath = CreateUniqueTestFilePath();
        
        try
        {
            var store = new FileEventStore(testFilePath);
            store.Append(
                new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship1", ColonistCount = 100 },
                new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship2", ColonistCount = 200 }
            );

            // Act
            var result = store.ReadFrom(10).ToList();

            // Assert
            Assertions.AssertThat(result).IsEmpty();
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }

    [TestCase]
    public void Constructor_ExistingFile_RestoresOffset()
    {
        // Arrange - Create store and add events
        var testFilePath = CreateUniqueTestFilePath();
        
        try
        {
            var store1 = new FileEventStore(testFilePath);
            store1.Append(
                new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship1", ColonistCount = 100 },
                new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship2", ColonistCount = 200 },
                new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship3", ColonistCount = 300 }
            );

            // Act - Create new store with same file
            var store2 = new FileEventStore(testFilePath);

            // Assert
            Assertions.AssertThat(store2.CurrentOffset).IsEqual(2);
            Assertions.AssertThat(store2.Count).IsEqual(3);

            // Verify can read all events
            var events = store2.ReadFrom(0).ToList();
            Assertions.AssertThat(events.Count).IsEqual(3);
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }

    [TestCase]
    public void Count_ReturnsCorrectCount()
    {
        // Arrange
        var testFilePath = CreateUniqueTestFilePath();
        
        try
        {
            var store = new FileEventStore(testFilePath);

            // Act & Assert - Empty store
            Assertions.AssertThat(store.Count).IsEqual(0);

            // Add events
            store.Append(
                new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship1", ColonistCount = 100 }
            );
            Assertions.AssertThat(store.Count).IsEqual(1);

            store.Append(
                new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship2", ColonistCount = 200 },
                new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship3", ColonistCount = 300 }
            );
            Assertions.AssertThat(store.Count).IsEqual(3);
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }

    [TestCase]
    public void Append_CorruptedLine_ThrowsException()
    {
        // Arrange - Create store with valid event
        var testFilePath = CreateUniqueTestFilePath();
        
        try
        {
            var store = new FileEventStore(testFilePath);
            store.Append(new ShipDepartedEvent 
            { 
                DestinationSystemId = Ulid.NewUlid(), 
                ShipName = "Ship1", 
                ColonistCount = 100 
            });

            // Corrupt the file by appending invalid data
            File.AppendAllText(testFilePath, "CORRUPTED|DATA|HERE|{invalid json}\n");

            // Act & Assert - Reading should throw EventStoreException
            var newStore = new FileEventStore(testFilePath);
            
            // GdUnit4 exception assertion
            try
            {
                newStore.ReadFrom(0).ToList();
                Assertions.AssertBool(false).OverrideFailureMessage("Expected EventStoreException to be thrown");
            }
            catch (EventStoreException)
            {
                // Expected exception
                Assertions.AssertBool(true).IsTrue();
            }
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }

    [TestCase]
    public void ReadFrom_PreservesEventTypes()
    {
        // Arrange
        var testFilePath = CreateUniqueTestFilePath();
        
        try
        {
            var store = new FileEventStore(testFilePath);
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
            Assertions.AssertThat(result.Count).IsEqual(5);
            Assertions.AssertThat(result[0]).IsInstanceOf<ShipDepartedEvent>();
            Assertions.AssertThat(result[1]).IsInstanceOf<MechanicalFailureEvent>();
            Assertions.AssertThat(result[2]).IsInstanceOf<SocialConflictEvent>();
            Assertions.AssertThat(result[3]).IsInstanceOf<AnomalyDetectedEvent>();
            Assertions.AssertThat(result[4]).IsInstanceOf<ShipArrivedEvent>();
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }

    [TestCase]
    public void Append_UpdatesCurrentOffset()
    {
        // Arrange
        var testFilePath = CreateUniqueTestFilePath();
        
        try
        {
            var store = new FileEventStore(testFilePath);

            // Act & Assert
            Assertions.AssertThat(store.CurrentOffset).IsEqual(-1); // Empty store

            store.Append(new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship1", ColonistCount = 100 });
            Assertions.AssertThat(store.CurrentOffset).IsEqual(0);

            store.Append(new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship2", ColonistCount = 200 });
            Assertions.AssertThat(store.CurrentOffset).IsEqual(1);

            store.Append(
                new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship3", ColonistCount = 300 },
                new ShipDepartedEvent { DestinationSystemId = Ulid.NewUlid(), ShipName = "Ship4", ColonistCount = 400 }
            );
            Assertions.AssertThat(store.CurrentOffset).IsEqual(3);
        }
        finally
        {
            CleanupTestFile(testFilePath);
        }
    }
}
