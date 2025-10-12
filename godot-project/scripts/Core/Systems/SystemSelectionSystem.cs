using System;
using System.Collections.Generic;
using Outpost3.Core.Commands;
using Outpost3.Core.Domain;
using Outpost3.Core.Events;

namespace Outpost3.Core.Systems;

/// <summary>
/// Pure reducer for system selection logic.
/// </summary>
public static class SystemSelectionSystem
{
    /// <summary>
    /// Handles system selection commands.
    /// </summary>
    /// <param name="state">The current game state.</param>
    /// <param name="command">The select system command.</param>
    /// <returns>A tuple of the new state and events generated.</returns>
    public static (GameState newState, List<IGameEvent> events) HandleSelectSystem(
        GameState state,
        SelectSystemCommand command)
    {
        // Validate system exists
        var systemExists = state.Systems.Exists(s => s.Id == command.SystemId);
        if (!systemExists)
        {
            // Invalid system ID - ignore command
            return (state, new List<IGameEvent>());
        }

        // Update state
        var newState = state.WithSelectedSystem(command.SystemId);

        // Emit event
        var evt = new SystemSelected(command.SystemId)
        {
            GameTime = (float)state.GameTime
        };

        return (newState, new List<IGameEvent> { evt });
    }

    /// <summary>
    /// Handles deselection (e.g., closing details modal).
    /// </summary>
    /// <param name="state">The current game state.</param>
    /// <returns>A tuple of the new state with no system selected and empty events list.</returns>
    public static (GameState newState, List<IGameEvent> events) HandleDeselectSystem(GameState state)
    {
        var newState = state.WithSelectedSystem(null);
        return (newState, new List<IGameEvent>());
    }
}
