using Godot;
using Outpost3.Core.Domain;

namespace Outpost3.UI;

/// <summary>
/// Component representing a single save file entry in the save list.
/// </summary>
public partial class SaveEntryComponent : PanelContainer
{
    [Export] private Label _saveNameLabel;
    [Export] private Label _gameTimeLabel;
    [Export] private Label _saveTimeLabel;
    [Export] private Label _versionLabel;
    [Export] private Label _eventCountLabel;
    
    [Signal]
    public delegate void SelectedEventHandler();
    
    private SaveMetadata _saveData;
    private bool _isSelected;
    
    public string SaveSlot => _saveData?.SaveSlot;
    
    public override void _Ready()
    {
        _saveNameLabel = GetNode<Label>("MarginContainer/HBoxContainer/VBoxContainer/SaveNameLabel");
        _gameTimeLabel = GetNode<Label>("MarginContainer/HBoxContainer/VBoxContainer/HBoxContainer/GameTimeLabel");
        _saveTimeLabel = GetNode<Label>("MarginContainer/HBoxContainer/VBoxContainer/HBoxContainer2/SaveTimeLabel");
        _versionLabel = GetNode<Label>("MarginContainer/HBoxContainer/VBoxContainer/HBoxContainer3/VersionLabel");
        _eventCountLabel = GetNode<Label>("MarginContainer/HBoxContainer/VBoxContainer2/EventCountLabel");
        GuiInput += OnGuiInput;
    }
    
    /// <summary>
    /// Sets the save data to display in this entry.
    /// </summary>
    public void SetSaveData(SaveMetadata saveData)
    {
        _saveData = saveData;
        
        _saveNameLabel.Text = saveData.DisplayName;
        
        var day = (int)(saveData.GameTime / 24.0);
        var hour = saveData.GameTime % 24.0;
        _gameTimeLabel.Text = $"Day {day}, {hour:F1}h";
        
        _saveTimeLabel.Text = saveData.SaveTime.ToLocalTime().ToString("yyyy-MM-dd HH:mm");
        _versionLabel.Text = saveData.GameVersion;
        _eventCountLabel.Text = $"{saveData.TotalEvents} events";
    }
    
    /// <summary>
    /// Sets the visual selection state of this entry.
    /// </summary>
    public void SetSelected(bool selected)
    {
        _isSelected = selected;
        
        // Visual feedback
        var styleBox = new StyleBoxFlat();
        styleBox.BgColor = selected ? new Color(0.3f, 0.5f, 0.7f, 0.3f) : new Color(0.2f, 0.2f, 0.2f, 0.3f);
        styleBox.BorderColor = selected ? new Color(0.5f, 0.7f, 1.0f) : new Color(0.4f, 0.4f, 0.4f);
        styleBox.BorderWidthAll = selected ? 2 : 1;
        AddThemeStyleboxOverride("panel", styleBox);
    }
    
    private void OnGuiInput(InputEvent @event)
    {
        if (@event is InputEventMouseButton mouseEvent && mouseEvent.Pressed && mouseEvent.ButtonIndex == MouseButton.Left)
        {
            EmitSignal(SignalName.Selected);
        }
    }
}
