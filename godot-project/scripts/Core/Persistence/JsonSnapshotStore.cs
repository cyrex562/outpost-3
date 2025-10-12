using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Serialization;
using Outpost3.Core.Domain;
using Outpost3.Core.Events;

namespace Outpost3.Core.Persistence;

/// <summary>
/// JSON-based implementation of snapshot storage.
/// Saves complete game state snapshots to individual files.
/// NOTE: Uses Console.WriteLine for logging to support testing outside Godot runtime.
/// </summary>
public class JsonSnapshotStore : ISnapshotStore
{
    private readonly string _savesDirectory;
    private readonly JsonSerializerOptions _jsonOptions;

    public JsonSnapshotStore(string savesDirectory)
    {
        _savesDirectory = savesDirectory;
        _jsonOptions = new JsonSerializerOptions
        {
            WriteIndented = true,
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            Converters =
            {
                new UlidJsonConverter(),
                new JsonStringEnumConverter()
            }
        };

        EnsureDirectoryExists(_savesDirectory);
    }

    public void SaveSnapshot(GameState state, long eventOffset, SaveMetadata metadata)
    {
        var saveDir = Path.Combine(_savesDirectory, metadata.SaveSlot);
        EnsureDirectoryExists(saveDir);

        var saveData = new SaveFile
        {
            Version = metadata.GameVersion,
            Metadata = metadata,
            SnapshotOffset = eventOffset,
            GameState = state
        };

        try
        {
            var json = JsonSerializer.Serialize(saveData, _jsonOptions);
            var savePath = Path.Combine(saveDir, "save.json");
            File.WriteAllText(savePath, json);

            Console.WriteLine($"✓ Saved game to '{metadata.SaveSlot}' at offset {eventOffset}");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"ERROR: Failed to save game: {ex.Message}");
            throw new EventStoreException($"Failed to save snapshot: {ex.Message}", ex);
        }
    }

    public (GameState state, long eventOffset, SaveMetadata metadata)? LoadSnapshot(string saveSlot)
    {
        var savePath = Path.Combine(_savesDirectory, saveSlot, "save.json");

        if (!File.Exists(savePath))
        {
            Console.WriteLine($"ERROR: Save file not found: {savePath}");
            return null;
        }

        try
        {
            var json = File.ReadAllText(savePath);
            var saveData = JsonSerializer.Deserialize<SaveFile>(json, _jsonOptions);

            if (saveData == null)
            {
                Console.WriteLine($"ERROR: Failed to deserialize save file: {savePath}");
                return null;
            }

            Console.WriteLine($"✓ Loaded game from '{saveSlot}' (offset: {saveData.SnapshotOffset})");
            return (saveData.GameState, saveData.SnapshotOffset, saveData.Metadata);
        }
        catch (Exception ex)
        {
            Console.WriteLine($"ERROR: Failed to load save: {ex.Message}");
            return null;
        }
    }

    public IEnumerable<SaveMetadata> ListSaves()
    {
        if (!Directory.Exists(_savesDirectory))
            return Enumerable.Empty<SaveMetadata>();

        var saves = new List<SaveMetadata>();

        foreach (var saveDir in Directory.GetDirectories(_savesDirectory))
        {
            var savePath = Path.Combine(saveDir, "save.json");
            if (!File.Exists(savePath))
                continue;

            try
            {
                var json = File.ReadAllText(savePath);
                var saveData = JsonSerializer.Deserialize<SaveFile>(json, _jsonOptions);

                if (saveData?.Metadata != null)
                {
                    saves.Add(saveData.Metadata);
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"ERROR: Failed to read save metadata from {saveDir}: {ex.Message}");
            }
        }

        return saves.OrderByDescending(s => s.SaveTime);
    }

    public void DeleteSave(string saveSlot)
    {
        var saveDir = Path.Combine(_savesDirectory, saveSlot);

        if (Directory.Exists(saveDir))
        {
            try
            {
                Directory.Delete(saveDir, recursive: true);
                Console.WriteLine($"✓ Deleted save: {saveSlot}");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"ERROR: Failed to delete save: {ex.Message}");
                throw new EventStoreException($"Failed to delete save: {ex.Message}", ex);
            }
        }
    }

    private void EnsureDirectoryExists(string path)
    {
        if (!Directory.Exists(path))
        {
            Directory.CreateDirectory(path);
        }
    }
}

/// <summary>
/// Container for save file data.
/// </summary>
public record SaveFile
{
    public string Version { get; init; }
    public SaveMetadata Metadata { get; init; }
    public long SnapshotOffset { get; init; }
    public GameState GameState { get; init; }
}
