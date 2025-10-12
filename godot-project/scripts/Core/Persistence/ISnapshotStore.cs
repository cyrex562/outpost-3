using System.Collections.Generic;
using Outpost3.Core.Domain;

namespace Outpost3.Core.Persistence;

/// <summary>
/// Interface for persisting and loading game state snapshots.
/// </summary>
public interface ISnapshotStore
{
    /// <summary>
    /// Saves a snapshot of the game state with metadata.
    /// </summary>
    void SaveSnapshot(GameState state, long eventOffset, SaveMetadata metadata);
    
    /// <summary>
    /// Loads the most recent snapshot for a save slot.
    /// Returns null if the save slot doesn't exist.
    /// </summary>
    (GameState state, long eventOffset, SaveMetadata metadata)? LoadSnapshot(string saveSlot);
    
    /// <summary>
    /// Lists all available save slots.
    /// </summary>
    IEnumerable<SaveMetadata> ListSaves();
    
    /// <summary>
    /// Deletes a save slot and all associated files.
    /// </summary>
    void DeleteSave(string saveSlot);
}
