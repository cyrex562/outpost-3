using Xunit;
using Outpost3.Core.Domain;
using Outpost3.Core.Persistence;
using Outpost3.Core.Services;
using Outpost3.Core;
using Outpost3.Core.Commands;
using System;
using System.IO;
using System.Linq;

namespace Outpost3.Tests;

/// <summary>
/// Unit tests for the Save/Load system (Session 1.4).
/// Tests snapshot storage, serialization, and save/load service operations.
/// </summary>
public class SaveLoadTests
{
    private readonly string _testSavesPath;

    public SaveLoadTests()
    {
        // Use a temporary directory for test saves
        _testSavesPath = Path.Combine(Path.GetTempPath(), "outpost3_test_saves", Guid.NewGuid().ToString());
        Directory.CreateDirectory(_testSavesPath);
    }

    private void Cleanup()
    {
        if (Directory.Exists(_testSavesPath))
        {
            Directory.Delete(_testSavesPath, recursive: true);
        }
    }

    #region JsonSnapshotStore Tests

    [Fact]
    public void SaveSnapshot_CreatesFileWithCorrectStructure()
    {
        // Arrange
        var store = new JsonSnapshotStore(_testSavesPath);
        var state = CreateTestGameState();
        var saveSlot = "test_slot_1";
        var displayName = "Test Save #1";
        long eventOffset = 10;

        try
        {
            // Act
            store.SaveSnapshot(state, eventOffset, SaveMetadata.Create(saveSlot, displayName, state, eventOffset));

            // Assert - Verify file exists
            var expectedPath = Path.Combine(_testSavesPath, saveSlot, "save.json");
            Assert.True(File.Exists(expectedPath));

            // Verify file contains JSON
            var json = File.ReadAllText(expectedPath);
            Assert.Contains("\"Metadata\":", json);
            Assert.Contains("\"State\":", json);
            Assert.Contains("\"GameTime\":", json);
        }
        finally
        {
            Cleanup();
        }
    }

    [Fact]
    public void LoadSnapshot_RestoresExactGameState()
    {
        // Arrange
        var store = new JsonSnapshotStore(_testSavesPath);
        var originalState = CreateTestGameState();
        var saveSlot = "restore_test";
        long eventOffset = 42;

        try
        {
            // Act
            var metadata = SaveMetadata.Create(saveSlot, "Restore Test", originalState, eventOffset);
            store.SaveSnapshot(originalState, eventOffset, metadata);
            var loadResult = store.LoadSnapshot(saveSlot);

            Assert.NotNull(loadResult);
            var (loadedState, lastEventNumber, loadedMetadata) = loadResult.Value;

            // Assert - State equality
            Assert.Equal(originalState.GameTime, loadedState.GameTime);
            Assert.Equal(originalState.Systems.Count, loadedState.Systems.Count);
            Assert.Equal(originalState.ProbesInFlight.Count, loadedState.ProbesInFlight.Count);

            // Assert - Metadata
            Assert.Equal(saveSlot, loadedMetadata.SaveSlot);
            Assert.Equal(eventOffset, lastEventNumber);
        }
        finally
        {
            Cleanup();
        }
    }

    [Fact]
    public void LoadSnapshot_ThrowsException_WhenSaveDoesNotExist()
    {
        // Arrange
        var store = new JsonSnapshotStore(_testSavesPath);
        var nonExistentSlot = "does_not_exist";

        try
        {
            // Act & Assert
            var ex = Assert.Throws<InvalidOperationException>(() =>
                store.LoadSnapshot(nonExistentSlot));

            Assert.Contains("not found", ex.Message);
        }
        finally
        {
            Cleanup();
        }
    }

    [Fact]
    public void ListSaves_ReturnsAllSavedGames()
    {
        // Arrange
        var store = new JsonSnapshotStore(_testSavesPath);
        var state = CreateTestGameState();

        try
        {
            // Act
            var meta1 = SaveMetadata.Create("save_1", "First Save", state, 10);
            var meta2 = SaveMetadata.Create("save_2", "Second Save", state, 20);
            var meta3 = SaveMetadata.Create("save_3", "Third Save", state, 30);
            
            store.SaveSnapshot(state, 10, meta1);
            store.SaveSnapshot(state, 20, meta2);
            store.SaveSnapshot(state, 30, meta3);

            var saves = store.ListSaves().ToList();

            // Assert
            Assert.Equal(3, saves.Count);
            Assert.Contains(saves, m => m.SaveSlot == "save_1" && m.DisplayName == "First Save");
            Assert.Contains(saves, m => m.SaveSlot == "save_2" && m.DisplayName == "Second Save");
            Assert.Contains(saves, m => m.SaveSlot == "save_3" && m.DisplayName == "Third Save");
        }
        finally
        {
            Cleanup();
        }
    }

    [Fact]
    public void ListSaves_ReturnsEmptyList_WhenNoSavesExist()
    {
        // Arrange
        var store = new JsonSnapshotStore(_testSavesPath);

        try
        {
            // Act
            var saves = store.ListSaves().ToList();

            // Assert
            Assert.Empty(saves);
        }
        finally
        {
            Cleanup();
        }
    }

    [Fact]
    public void DeleteSave_RemovesAllFiles()
    {
        // Arrange
        var store = new JsonSnapshotStore(_testSavesPath);
        var state = CreateTestGameState();
        var saveSlot = "delete_test";

        try
        {
            // Act
            var metadata = SaveMetadata.Create(saveSlot, "To Be Deleted", state, 5);
            store.SaveSnapshot(state, 5, metadata);
            var savePathBefore = Path.Combine(_testSavesPath, saveSlot);
            Assert.True(Directory.Exists(savePathBefore));

            store.DeleteSave(saveSlot);

            // Assert
            Assert.False(Directory.Exists(savePathBefore));
        }
        finally
        {
            Cleanup();
        }
    }

    [Fact]
    public void DeleteSave_DoesNotThrow_WhenSaveDoesNotExist()
    {
        // Arrange
        var store = new JsonSnapshotStore(_testSavesPath);

        try
        {
            // Act & Assert (should not throw)
            store.DeleteSave("nonexistent_save");
        }
        finally
        {
            Cleanup();
        }
    }

    #endregion

    #region SaveMetadata Tests

    [Fact]
    public void SaveMetadata_Create_GeneratesCorrectMetadata()
    {
        // Arrange
        var state = CreateTestGameState();
        var saveSlot = "metadata_test";
        var displayName = "Metadata Test Save";
        long eventOffset = 99;

        // Act
        var metadata = SaveMetadata.Create(saveSlot, displayName, state, eventOffset);

        // Assert
        Assert.Equal(saveSlot, metadata.SaveSlot);
        Assert.Equal(displayName, metadata.DisplayName);
        Assert.Equal(state.GameTime, metadata.GameTime);
        Assert.Equal(100, metadata.TotalEvents); // eventOffset + 1
        Assert.True(metadata.SaveTime > DateTime.MinValue);
        Assert.False(string.IsNullOrWhiteSpace(metadata.GameVersion));
    }

    #endregion

    #region SaveLoadService Integration Tests

    [Fact]
    public void SaveLoadService_SaveGame_CreatesSnapshot()
    {
        // Arrange
        var eventStorePath = Path.Combine(_testSavesPath, "events.jsonl");
        var eventStore = new FileEventStore(eventStorePath);
        var stateStore = new StateStore(eventStore);
        var snapshotStore = new JsonSnapshotStore(_testSavesPath);
        var service = new SaveLoadService(stateStore, eventStore, snapshotStore);

        try
        {
            // Act - Apply some commands first
            stateStore.ApplyCommand(new AdvanceTime(100.0));
            stateStore.ApplyCommand(new AdvanceTime(50.0));

            service.SaveGame("service_test", "Service Integration Test");

            // Assert - Verify snapshot exists
            var saves = snapshotStore.ListSaves().ToList();
            Assert.Contains(saves, m => m.SaveSlot == "service_test");
            Assert.Single(saves);
        }
        finally
        {
            Cleanup();
        }
    }

    [Fact]
    public void SaveLoadService_LoadGame_RestoresStateCorrectly()
    {
        // Arrange
        var eventStorePath = Path.Combine(_testSavesPath, "events.jsonl");
        var eventStore = new FileEventStore(eventStorePath);
        var stateStore = new StateStore(eventStore);
        var snapshotStore = new JsonSnapshotStore(_testSavesPath);
        var service = new SaveLoadService(stateStore, eventStore, snapshotStore);

        try
        {
            // Act - Create initial state
            stateStore.ApplyCommand(new AdvanceTime(100.0));
            stateStore.ApplyCommand(new LaunchProbe(Ulid.NewUlid()));

            var originalState = stateStore.State;
            var originalGameTime = originalState.GameTime;
            var originalProbeCount = originalState.ProbesInFlight.Count;
            
            service.SaveGame("load_test", "Load Test Save");

            // Modify state
            stateStore.ApplyCommand(new AdvanceTime(500.0));
            var modifiedState = stateStore.State;
            Assert.NotEqual(originalGameTime, modifiedState.GameTime);

            // Load saved state
            service.LoadGame("load_test");
            var restoredState = stateStore.State;

            // Assert
            Assert.Equal(originalGameTime, restoredState.GameTime);
            Assert.Equal(originalProbeCount, restoredState.ProbesInFlight.Count);
        }
        finally
        {
            Cleanup();
        }
    }

    [Fact]
    public void SaveLoadService_QuickSave_UsesQuickSaveSlot()
    {
        // Arrange
        var eventStorePath = Path.Combine(_testSavesPath, "events.jsonl");
        var eventStore = new FileEventStore(eventStorePath);
        var stateStore = new StateStore(eventStore);
        var snapshotStore = new JsonSnapshotStore(_testSavesPath);
        var service = new SaveLoadService(stateStore, eventStore, snapshotStore);

        try
        {
            // Act
            stateStore.ApplyCommand(new AdvanceTime(50.0));
            service.QuickSave();

            // Assert
            var saves = snapshotStore.ListSaves().ToList();
            Assert.Single(saves);
            Assert.Equal("quicksave", saves[0].SaveSlot);
            Assert.Equal("Quick Save", saves[0].DisplayName);
        }
        finally
        {
            Cleanup();
        }
    }

    [Fact]
    public void SaveLoadService_AutoSave_UsesAutoSaveSlot()
    {
        // Arrange
        var eventStorePath = Path.Combine(_testSavesPath, "events.jsonl");
        var eventStore = new FileEventStore(eventStorePath);
        var stateStore = new StateStore(eventStore);
        var snapshotStore = new JsonSnapshotStore(_testSavesPath);
        var service = new SaveLoadService(stateStore, eventStore, snapshotStore);

        try
        {
            // Act
            stateStore.ApplyCommand(new AdvanceTime(75.0));
            service.AutoSave();

            // Assert
            var saves = snapshotStore.ListSaves().ToList();
            Assert.Single(saves);
            Assert.Equal("autosave", saves[0].SaveSlot);
            Assert.Equal("Auto Save", saves[0].DisplayName);
        }
        finally
        {
            Cleanup();
        }
    }

    #endregion

    #region UlidJsonConverter Tests

    [Fact]
    public void UlidJsonConverter_RoundTrip_PreservesValue()
    {
        // Arrange
        var originalUlid = Ulid.NewUlid();
        var json = System.Text.Json.JsonSerializer.Serialize(originalUlid, new System.Text.Json.JsonSerializerOptions
        {
            Converters = { new UlidJsonConverter() }
        });

        // Act
        var deserializedUlid = System.Text.Json.JsonSerializer.Deserialize<Ulid>(json, new System.Text.Json.JsonSerializerOptions
        {
            Converters = { new UlidJsonConverter() }
        });

        // Assert
        Assert.Equal(originalUlid, deserializedUlid);
    }

    #endregion

    #region Helper Methods

    private GameState CreateTestGameState()
    {
        // Create a test state with some systems and probes
        var state = GameState.NewGame();
        state = state.WithAdvanceTime(123.45);
        
        // Add a test system
        var system = new StarSystem
        {
            Id = Ulid.NewUlid(),
            Name = "Test System Alpha",
            SpectralClass = "G2V",
            Bodies = new System.Collections.Generic.List<CelestialBody>()
        };
        state = state.WithSystemDiscovered(system);
        
        return state;
    }

    #endregion
}
