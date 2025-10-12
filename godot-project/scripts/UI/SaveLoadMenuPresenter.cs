using System;
using System.Linq;
using Godot;
using Outpost3.Core.Services;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the save/load menu screen.
/// Manages displaying, loading, and deleting save files.
/// </summary>
public partial class SaveLoadMenuPresenter : Control
{
    [Export] private VBoxContainer _saveListContainer;
    [Export] private Button _newSaveButton;
    [Export] private Button _refreshButton;
    [Export] private Button _loadButton;
    [Export] private Button _deleteButton;
    [Export] private Button _backButton;
    [Export] private Label _detailsLabel;
    [Export] private PackedScene _saveEntryScene;
    
    private SaveLoadService _saveLoadService;
    private string _selectedSaveSlot;
    
    public override void _Ready()
    {
        // Get service from App singleton
        var app = GetNode<App>("/root/App");
        if (app == null)
        {
            GD.PrintErr("Failed to get App singleton");
            return;
        }
        
        _saveLoadService = app.GetSaveLoadService();
        if (_saveLoadService == null)
        {
            GD.PrintErr("Failed to get SaveLoadService from App");
            return;
        }
        
        // Connect signals
        _newSaveButton.Pressed += OnNewSavePressed;
        _refreshButton.Pressed += RefreshSaveList;
        _loadButton.Pressed += OnLoadPressed;
        _deleteButton.Pressed += OnDeletePressed;
        _backButton.Pressed += OnBackPressed;
        
        // Load save list
        RefreshSaveList();
    }
    
    private void RefreshSaveList()
    {
        // Clear existing entries
        foreach (var child in _saveListContainer.GetChildren())
        {
            child.QueueFree();
        }
        
        var saves = _saveLoadService.ListSaves().ToList();
        
        if (saves.Count == 0)
        {
            _detailsLabel.Text = "No save files found";
            return;
        }
        
        foreach (var save in saves)
        {
            var entry = _saveEntryScene.Instantiate<SaveEntryComponent>();
            entry.SetSaveData(save);
            entry.Selected += () => OnSaveSelected(save.SaveSlot);
            _saveListContainer.AddChild(entry);
        }
        
        _detailsLabel.Text = $"{saves.Count} save file(s)";
    }
    
    private void OnSaveSelected(string saveSlot)
    {
        _selectedSaveSlot = saveSlot;
        _loadButton.Disabled = false;
        _deleteButton.Disabled = false;
        
        // Update visual selection
        foreach (var child in _saveListContainer.GetChildren().Cast<SaveEntryComponent>())
        {
            child.SetSelected(child.SaveSlot == saveSlot);
        }
        
        _detailsLabel.Text = $"Selected: {saveSlot}";
    }
    
    private void OnLoadPressed()
    {
        if (string.IsNullOrEmpty(_selectedSaveSlot))
            return;
        
        if (_saveLoadService.LoadGame(_selectedSaveSlot))
        {
            // Return to main game scene
            GetTree().ChangeSceneToFile("res://scenes/Main.tscn");
        }
        else
        {
            ShowError("Failed to load save file");
        }
    }
    
    private void OnNewSavePressed()
    {
        ShowSaveNameDialog();
    }
    
    private void OnDeletePressed()
    {
        if (string.IsNullOrEmpty(_selectedSaveSlot))
            return;
        
        ShowConfirmDialog($"Delete save '{_selectedSaveSlot}'?", () =>
        {
            _saveLoadService.DeleteSave(_selectedSaveSlot);
            _selectedSaveSlot = null;
            _loadButton.Disabled = true;
            _deleteButton.Disabled = true;
            RefreshSaveList();
        });
    }
    
    private void OnBackPressed()
    {
        // Check if StartMenu exists, otherwise go to Main
        if (ResourceLoader.Exists("res://scenes/game_start/StartMenu.tscn"))
        {
            GetTree().ChangeSceneToFile("res://scenes/game_start/StartMenu.tscn");
        }
        else
        {
            GetTree().ChangeSceneToFile("res://scenes/Main.tscn");
        }
    }
    
    private void ShowSaveNameDialog()
    {
        var dialog = new AcceptDialog();
        dialog.Title = "New Save";
        dialog.DialogText = "Enter save name:";
        
        var lineEdit = new LineEdit();
        lineEdit.PlaceholderText = "My Save";
        lineEdit.Text = $"Save {DateTime.Now:yyyy-MM-dd HH:mm}";
        dialog.AddChild(lineEdit);
        
        dialog.Confirmed += () =>
        {
            var saveName = lineEdit.Text;
            if (!string.IsNullOrWhiteSpace(saveName))
            {
                var saveSlot = $"manual_{DateTime.Now:yyyyMMdd_HHmmss}";
                _saveLoadService.SaveGame(saveSlot, saveName);
                RefreshSaveList();
            }
        };
        
        AddChild(dialog);
        dialog.PopupCentered(new Vector2I(400, 150));
    }
    
    private void ShowConfirmDialog(string message, Action onConfirm)
    {
        var dialog = new ConfirmationDialog();
        dialog.DialogText = message;
        dialog.Confirmed += () => onConfirm();
        
        AddChild(dialog);
        dialog.PopupCentered();
    }
    
    private void ShowError(string message)
    {
        var dialog = new AcceptDialog();
        dialog.DialogText = message;
        AddChild(dialog);
        dialog.PopupCentered();
    }
}
