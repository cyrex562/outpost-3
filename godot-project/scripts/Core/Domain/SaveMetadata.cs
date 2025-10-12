using System;

namespace Outpost3.Core.Domain;

/// <summary>
/// Metadata about a saved game.
/// </summary>
public record SaveMetadata
{
    public string SaveSlot { get; init; }      // "autosave", "quicksave", "manual_001"
    public string DisplayName { get; init; }   // "Autosave - Day 42"
    public DateTime SaveTime { get; init; }
    public double GameTime { get; init; }      // In-game time (hours)
    public int TotalEvents { get; init; }      // Event count at save time
    public string GameVersion { get; init; }   // "0.1.4"
    
    /// <summary>
    /// Creates metadata for a new save.
    /// </summary>
    public static SaveMetadata Create(string saveSlot, string displayName, GameState state, long eventOffset)
    {
        return new SaveMetadata
        {
            SaveSlot = saveSlot,
            DisplayName = displayName,
            SaveTime = DateTime.UtcNow,
            GameTime = state.GameTime,
            TotalEvents = (int)eventOffset + 1,
            GameVersion = "0.1.4"
        };
    }
}
