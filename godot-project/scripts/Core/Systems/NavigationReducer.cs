using System.Collections.Generic;
using System.Linq;
using Outpost3.Core.Commands;
using Outpost3.Core.Domain;
using Outpost3.Core.Events;

namespace Outpost3.Core.Systems;

/// <summary>
/// Pure reducer for navigation commands.
/// Handles screen navigation stack (FILO).
/// </summary>
public static class NavigationReducer
{
    /// <summary>
    /// Handles PushScreen command - adds screen to navigation stack.
    /// </summary>
    public static (GameState newState, List<IGameEvent> events) HandlePushScreen(
        GameState state,
        PushScreen command)
    {
        var newStack = new Stack<ScreenId>(state.NavigationStack.Reverse());
        newStack.Push(command.Screen);

        var newState = state with { NavigationStack = newStack };

        var evt = new ScreenPushed(command.Screen)
        {
            GameTime = (float)state.GameTime
        };

        return (newState, new List<IGameEvent> { evt });
    }

    /// <summary>
    /// Handles PopScreen command - removes top screen from navigation stack.
    /// </summary>
    public static (GameState newState, List<IGameEvent> events) HandlePopScreen(
        GameState state,
        PopScreen command)
    {
        if (state.NavigationStack.Count == 0)
        {
            // Stack is empty - ignore command
            return (state, new List<IGameEvent>());
        }

        var newStack = new Stack<ScreenId>(state.NavigationStack.Reverse());
        var poppedScreen = newStack.Pop();

        var newState = state with { NavigationStack = newStack };

        var evt = new ScreenPopped(poppedScreen)
        {
            GameTime = (float)state.GameTime
        };

        return (newState, new List<IGameEvent> { evt });
    }

    /// <summary>
    /// Handles NavigateToScreen command - clears stack and sets single screen.
    /// Used for direct navigation without back button (e.g., main menu to game).
    /// </summary>
    public static (GameState newState, List<IGameEvent> events) HandleNavigateToScreen(
        GameState state,
        NavigateToScreen command)
    {
        var newStack = new Stack<ScreenId>();
        newStack.Push(command.Screen);

        var newState = state with { NavigationStack = newStack };

        var evt = new NavigatedToScreen(command.Screen)
        {
            GameTime = (float)state.GameTime
        };

        return (newState, new List<IGameEvent> { evt });
    }
}
