using Outpost3.Core.Domain;

namespace Outpost3.Core.Commands;

/// <summary>
/// Push a new screen onto the navigation stack.
/// </summary>
public record PushScreen(ScreenId Screen) : ICommand;

/// <summary>
/// Pop the current screen from the navigation stack (go back).
/// </summary>
public record PopScreen() : ICommand;

/// <summary>
/// Navigate directly to a specific screen (clears stack and pushes this screen).
/// </summary>
public record NavigateToScreen(ScreenId Screen) : ICommand;
