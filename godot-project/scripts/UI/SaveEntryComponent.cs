using Godot;
using Outpost3.Core.Domain;

namespace Outpost3.UI;

/// <summary>
/// Component representing a single save file entry in the save list.
/// </summary>
public partial class SaveEntryComponent : PanelContainer
{
    [Export] private Label? _saveNameLabel;
    [Export] private Label? _gameTimeLabel;
    [Export] private Label? _saveTimeLabel;
    [Export] private Label? _versionLabel;
    [Export] private Label? _eventCountLabel;
    
    [Signal]
    public delegate void SelectedEventHandler();
    
    private SaveMetadata? _saveData;
    private bool _isSelected;
    
    public string? SaveSlot => _saveData?.SaveSlot;
    
    public override void _Ready()
    {
        // Get node references if not set via exports
        _saveNameLabel ??= GetNode<Label>("MarginContainer/HBoxContainer/LeftSection/SaveNameLabel");
        _gameTimeLabel ??= GetNode<Label>("MarginContainer/HBoxContainer/LeftSection/GameTimeContainer/GameTimeLabel");
        _saveTimeLabel ??= GetNode<Label>("MarginContainer/HBoxContainer/LeftSection/SaveTimeContainer/SaveTimeLabel");
        _versionLabel ??= GetNode<Label>("MarginContainer/HBoxContainer/LeftSection/VersionContainer/VersionLabel");
        _eventCountLabel ??= GetNode<Label>("MarginContainer/HBoxContainer/RightSection/EventCountLabel");
        
        GuiInput += OnGuiInput;
    }
    
    /// <summary>
    /// Sets the save data to display in this entry.
    /// </summary>
    public void SetSaveData(SaveMetadata saveData)
    {
        _saveData = saveData;
        
        if (_saveNameLabel != null)
            _saveNameLabel.Text = saveData.DisplayName;
        
        var day = (int)(saveData.GameTime / 24.0);
        var hour = saveData.GameTime % 24.0;
        if (_gameTimeLabel != null)
            _gameTimeLabel.Text = $"Day {day}, {hour:F1}h";
        
        if (_saveTimeLabel != null)
            _saveTimeLabel.Text = saveData.SaveTime.ToLocalTime().ToString("yyyy-MM-dd HH:mm");
        if (_versionLabel != null)
            _versionLabel.Text = saveData.GameVersion;
        if (_eventCountLabel != null)
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
        
        // Set border width on all sides
        int borderWidth = selected ? 2 : 1;
        styleBox.BorderWidthLeft = borderWidth;
        styleBox.BorderWidthRight = borderWidth;
        styleBox.BorderWidthTop = borderWidth;
        styleBox.BorderWidthBottom = borderWidth;
        
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
