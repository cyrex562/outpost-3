using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using Outpost3.Core.Domain;
using Outpost3.Core.Events;
using Outpost3.Core.Persistence;

namespace Outpost3.Core.Services;

/// <summary>
/// Coordinates save and load operations for the game.
/// Manages snapshots, event logs, and state restoration.
/// </summary>
public class SaveLoadService
{
    private readonly StateStore _stateStore;
    private readonly IEventStore _eventStore;
    private readonly ISnapshotStore _snapshotStore;
    
    public SaveLoadService(StateStore stateStore, IEventStore eventStore, ISnapshotStore snapshotStore)
    {
        _stateStore = stateStore ?? throw new ArgumentNullException(nameof(stateStore));
        _eventStore = eventStore ?? throw new ArgumentNullException(nameof(eventStore));
        _snapshotStore = snapshotStore ?? throw new ArgumentNullException(nameof(snapshotStore));
    }
    
    /// <summary>
    /// Saves the current game state to a named slot.
    /// </summary>
    public void SaveGame(string saveSlot, string displayName)
    {
        var currentState = _stateStore.State;
        var currentOffset = _eventStore.CurrentOffset;
        
        var metadata = SaveMetadata.Create(saveSlot, displayName, currentState, currentOffset);
        
        _snapshotStore.SaveSnapshot(currentState, currentOffset, metadata);
        
        GD.Print($"💾 Game saved: {displayName} (offset: {currentOffset}, game time: {currentState.GameTime:F1}h)");
    }
    
    /// <summary>
    /// Loads a saved game, reconstructing state from snapshot.
    /// </summary>
    public bool LoadGame(string saveSlot)
    {
        var snapshot = _snapshotStore.LoadSnapshot(saveSlot);
        if (snapshot == null)
        {
            GD.PrintErr($"Save slot '{saveSlot}' not found");
            return false;
        }
        
        var (savedState, snapshotOffset, metadata) = snapshot.Value;
        
        // For now, we load the exact snapshot state
        // In a full event-sourced system, we would replay tail events here
        // TODO: Implement tail event replay if events exist after snapshot offset
        
        // Replace state in StateStore
        _stateStore.LoadState(savedState);
        
        GD.Print($"📂 Game loaded: {metadata.DisplayName} (game time: {metadata.GameTime:F1}h)");
        return true;
    }
    
    /// <summary>
    /// Lists all available save files.
    /// </summary>
    public IEnumerable<SaveMetadata> ListSaves() => _snapshotStore.ListSaves();
    
    /// <summary>
    /// Deletes a save slot.
    /// </summary>
    public void DeleteSave(string saveSlot)
    {
        _snapshotStore.DeleteSave(saveSlot);
    }
    
    /// <summary>
    /// Quick save to the quicksave slot.
    /// </summary>
    public void QuickSave()
    {
        var displayName = $"Quick Save - {DateTime.Now:yyyy-MM-dd HH:mm}";
        SaveGame("quicksave", displayName);
    }
    
    /// <summary>
    /// Auto save to the autosave slot.
    /// </summary>
    public void AutoSave()
    {
        var day = (int)(_stateStore.State.GameTime / 24.0);
        var displayName = $"Auto Save - Day {day}";
        SaveGame("autosave", displayName);
    }
    
    /// <summary>
    /// Quick load from the quicksave slot.
    /// </summary>
    public bool QuickLoad()
    {
        return LoadGame("quicksave");
    }
}
