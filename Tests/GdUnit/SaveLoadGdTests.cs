using GdUnit4;
using Outpost3.Core.Domain;
using Outpost3.Core.Persistence;
using Outpost3.Core.Services;
using Outpost3.Core;
using Outpost3.Core.Commands;
using System;
using System.IO;
using System.Linq;

namespace Outpost3.Tests.GdUnit;

/// <summary>
/// GdUnit4 tests for the Save/Load system (Session 1.4).
/// Tests snapshot storage, serialization, and save/load service operations.
/// NOTE: Tests that instantiate StateStore require Godot runtime and should use GdUnit4.
/// These tests focus on JsonSnapshotStore and SaveMetadata (no Godot dependencies).
/// </summary>
[TestSuite]
public class SaveLoadGdTests
{
    private string _testSavesPath = null!;

    [Before]
    public void Setup()
    {
        // Use a temporary directory for test saves
        _testSavesPath = Path.Combine(Path.GetTempPath(), "outpost3_test_saves", Guid.NewGuid().ToString());
        Directory.CreateDirectory(_testSavesPath);
    }

    [After]
    public void Cleanup()
    {
        if (Directory.Exists(_testSavesPath))
        {
            try
            {
                Directory.Delete(_testSavesPath, recursive: true);
            }
            catch
            {
                // Ignore cleanup errors
            }
        }
    }

    #region JsonSnapshotStore Tests

    [TestCase]
    public void SaveSnapshot_CreatesFileWithCorrectStructure()
    {
        // Arrange
        var store = new JsonSnapshotStore(_testSavesPath);
        var state = CreateTestGameState();
        var saveSlot = "test_slot_1";
        var displayName = "Test Save #1";
        long eventOffset = 10;

        // Act
        store.SaveSnapshot(state, eventOffset, SaveMetadata.Create(saveSlot, displayName, state, eventOffset));

        // Assert - Verify file exists
        var expectedPath = Path.Combine(_testSavesPath, saveSlot, "save.json");
        Assertions.AssertThat(File.Exists(expectedPath)).IsTrue();

        // Verify file contains JSON with camelCase property names
        var json = File.ReadAllText(expectedPath);
        Assertions.AssertThat(json).Contains("\"metadata\":");  // Fixed: camelCase, not PascalCase
        Assertions.AssertThat(json).Contains("\"State\":");
        Assertions.AssertThat(json).Contains("\"GameTime\":");
    }

    [TestCase]
    public void LoadSnapshot_RestoresExactGameState()
    {
        // Arrange
        var store = new JsonSnapshotStore(_testSavesPath);
        var originalState = CreateTestGameState();
        var saveSlot = "restore_test";
        long eventOffset = 42;

        // Act
        var metadata = SaveMetadata.Create(saveSlot, "Restore Test", originalState, eventOffset);
        store.SaveSnapshot(originalState, eventOffset, metadata);
        var loadResult = store.LoadSnapshot(saveSlot);

        Assertions.AssertThat(loadResult).IsNotNull();
        var (loadedState, lastEventNumber, loadedMetadata) = loadResult!.Value;

        // Assert - State equality
        Assertions.AssertThat(loadedState.GameTime).IsEqual(originalState.GameTime);
        Assertions.AssertThat(loadedState.Systems.Count).IsEqual(originalState.Systems.Count);
        Assertions.AssertThat(loadedState.ProbesInFlight.Count).IsEqual(originalState.ProbesInFlight.Count);

        // Assert - Metadata
        Assertions.AssertThat(loadedMetadata.SaveSlot).IsEqual(saveSlot);
        Assertions.AssertThat(lastEventNumber).IsEqual(eventOffset);
    }

    [TestCase]
    public void LoadSnapshot_ReturnsNull_WhenSaveDoesNotExist()
    {
        // Arrange
        var store = new JsonSnapshotStore(_testSavesPath);
        var nonExistentSlot = "does_not_exist";

        // Act
        var result = store.LoadSnapshot(nonExistentSlot);

        // Assert - Fixed: LoadSnapshot returns null, not exception
        Assertions.AssertThat(result).IsNull();
    }

    [TestCase]
    public void ListSaves_ReturnsAllSavedGames()
    {
        // Arrange
        var store = new JsonSnapshotStore(_testSavesPath);
        var state = CreateTestGameState();

        // Act
        var meta1 = SaveMetadata.Create("save_1", "First Save", state, 10);
        var meta2 = SaveMetadata.Create("save_2", "Second Save", state, 20);
        var meta3 = SaveMetadata.Create("save_3", "Third Save", state, 30);

        store.SaveSnapshot(state, 10, meta1);
        store.SaveSnapshot(state, 20, meta2);
        store.SaveSnapshot(state, 30, meta3);

        var saves = store.ListSaves().ToList();

        // Assert
        Assertions.AssertThat(saves.Count).IsEqual(3);
        Assertions.AssertThat(saves.Any(m => m.SaveSlot == "save_1" && m.DisplayName == "First Save")).IsTrue();
        Assertions.AssertThat(saves.Any(m => m.SaveSlot == "save_2" && m.DisplayName == "Second Save")).IsTrue();
        Assertions.AssertThat(saves.Any(m => m.SaveSlot == "save_3" && m.DisplayName == "Third Save")).IsTrue();
    }

    [TestCase]
    public void ListSaves_ReturnsEmptyList_WhenNoSavesExist()
    {
        // Arrange
        var store = new JsonSnapshotStore(_testSavesPath);

        // Act
        var saves = store.ListSaves().ToList();

        // Assert
        Assertions.AssertThat(saves.Count).IsEqual(0);
    }

    [TestCase]
    public void DeleteSave_RemovesAllFiles()
    {
        // Arrange
        var store = new JsonSnapshotStore(_testSavesPath);
        var state = CreateTestGameState();
        var saveSlot = "delete_test";

        // Act
        var metadata = SaveMetadata.Create(saveSlot, "To Be Deleted", state, 5);
        store.SaveSnapshot(state, 5, metadata);
        var savePathBefore = Path.Combine(_testSavesPath, saveSlot);
        Assertions.AssertThat(Directory.Exists(savePathBefore)).IsTrue();

        store.DeleteSave(saveSlot);

        // Assert
        Assertions.AssertThat(Directory.Exists(savePathBefore)).IsFalse();
    }

    [TestCase]
    public void DeleteSave_DoesNotThrow_WhenSaveDoesNotExist()
    {
        // Arrange
        var store = new JsonSnapshotStore(_testSavesPath);

        // Act & Assert (should not throw)
        store.DeleteSave("nonexistent_save");

        // If we reach here without exception, test passes
        Assertions.AssertThat(true).IsTrue();
    }

    #endregion

    #region SaveMetadata Tests

    [TestCase]
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
        Assertions.AssertThat(metadata.SaveSlot).IsEqual(saveSlot);
        Assertions.AssertThat(metadata.DisplayName).IsEqual(displayName);
        Assertions.AssertThat(metadata.GameTime).IsEqual(state.GameTime);
        Assertions.AssertThat(metadata.TotalEvents).IsEqual(100); // eventOffset + 1
        Assertions.AssertThat(metadata.SaveTime > DateTime.MinValue).IsTrue();
        Assertions.AssertThat(string.IsNullOrWhiteSpace(metadata.GameVersion)).IsFalse();
    }

    #endregion

    #region SaveLoadService Integration Tests
    // =========================================================================
    // NOTE: SaveLoadService integration tests are not included here because
    // they require StateStore, which extends Godot.Node and needs Godot runtime.
    // 
    // These tests should be:
    // 1. Run manually in Godot Editor as integration tests, OR
    // 2. Created as GdUnit4 tests that run inside Godot Editor, OR
    // 3. Wait for StateStore refactoring to extract testable core (IStateStore interface)
    //
    // For now, SaveLoadService is tested via manual testing (see docs/10_manual_testing_save_load.md)
    // =========================================================================
    #endregion

    #region UlidJsonConverter Tests

    [TestCase]
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
        Assertions.AssertThat(deserializedUlid).IsEqual(originalUlid);
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
