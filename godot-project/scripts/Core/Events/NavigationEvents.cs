using System;
using Outpost3.Core.Domain;

namespace Outpost3.Core.Events;

/// <summary>
/// Event emitted when a screen is pushed onto the navigation stack.
/// </summary>
public record ScreenPushed : GameEvent
{
    /// <summary>
    /// The screen that was pushed.
    /// </summary>
    public ScreenId Screen { get; init; }

    public ScreenPushed() { }

    public ScreenPushed(ScreenId screen)
    {
        Screen = screen;
    }
}

/// <summary>
/// Event emitted when a screen is popped from the navigation stack.
/// </summary>
public record ScreenPopped : GameEvent
{
    /// <summary>
    /// The screen that was popped (the one that was just left).
    /// </summary>
    public ScreenId? PoppedScreen { get; init; }

    public ScreenPopped() { }

    public ScreenPopped(ScreenId? poppedScreen)
    {
        PoppedScreen = poppedScreen;
    }
}

/// <summary>
/// Event emitted when navigating directly to a screen (clearing stack).
/// </summary>
public record NavigatedToScreen : GameEvent
{
    /// <summary>
    /// The screen that was navigated to.
    /// </summary>
    public ScreenId Screen { get; init; }

    public NavigatedToScreen() { }

    public NavigatedToScreen(ScreenId screen)
    {
        Screen = screen;
    }
}
