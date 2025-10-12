using Godot;
using Outpost3.Core.Services;

namespace Outpost3.Core;

/// <summary>
/// Handles global input like quick save/load hotkeys.
/// </summary>
public partial class GlobalInputHandler : Node
{
    private SaveLoadService? _saveLoadService;

    public override void _Ready()
    {
        var gameServices = GetNode<GameServices>("/root/GameServices");
        if (gameServices != null)
        {
            _saveLoadService = gameServices.SaveLoadService;
            GD.Print("GlobalInputHandler ready - F5: Quick Save, F9: Quick Load");
        }
        else
        {
            GD.PrintErr("GlobalInputHandler: Failed to get GameServices autoload");
        }
    }

    public override void _Input(InputEvent @event)
    {
        if (_saveLoadService == null)
            return;

        if (@event.IsActionPressed("quick_save"))
        {
            _saveLoadService.QuickSave();
            ShowNotification("Quick saved!");
            GetViewport().SetInputAsHandled();
        }
        else if (@event.IsActionPressed("quick_load"))
        {
            if (_saveLoadService.QuickLoad())
            {
                ShowNotification("Quick loaded!");
            }
            else
            {
                ShowNotification("No quick save found!", isError: true);
            }
            GetViewport().SetInputAsHandled();
        }
    }

    private void ShowNotification(string message, bool isError = false)
    {
        // Simple console notification for now
        // TODO: Implement on-screen toast notification UI
        if (isError)
        {
            GD.PrintErr($"[Notification] {message}");
        }
        else
        {
            GD.Print($"[Notification] {message}");
        }
    }
}
