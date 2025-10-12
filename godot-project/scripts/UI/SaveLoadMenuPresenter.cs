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
    [Export] private VBoxContainer? _saveListContainer;
    [Export] private Button? _newSaveButton;
    [Export] private Button? _refreshButton;
    [Export] private Button? _loadButton;
    [Export] private Button? _deleteButton;
    [Export] private Button? _backButton;
    [Export] private Label? _detailsLabel;
    [Export] private PackedScene? _saveEntryScene;

    private SaveLoadService? _saveLoadService;
    private string? _selectedSaveSlot;

    public override void _Ready()
    {
        // Get node references if not set via exports
        _saveListContainer ??= GetNode<VBoxContainer>("PanelContainer/MarginContainer/VBoxContainer/ScrollContainer/SaveListContainer");
        _newSaveButton ??= GetNode<Button>("PanelContainer/MarginContainer/VBoxContainer/ControlsBar/NewSaveButton");
        _refreshButton ??= GetNode<Button>("PanelContainer/MarginContainer/VBoxContainer/ControlsBar/RefreshButton");
        _backButton ??= GetNode<Button>("PanelContainer/MarginContainer/VBoxContainer/ControlsBar/BackButton");
        _loadButton ??= GetNode<Button>("PanelContainer/MarginContainer/VBoxContainer/ActionsBar/LoadButton");
        _deleteButton ??= GetNode<Button>("PanelContainer/MarginContainer/VBoxContainer/ActionsBar/DeleteButton");
        _detailsLabel ??= GetNode<Label>("PanelContainer/MarginContainer/VBoxContainer/ActionsBar/DetailsLabel");

        // Get service from GameServices autoload
        var gameServices = GetNode<GameServices>("/root/GameServices");
        if (gameServices == null)
        {
            GD.PrintErr("Failed to get GameServices autoload");
            return;
        }

        _saveLoadService = gameServices.SaveLoadService;
        if (_saveLoadService == null)
        {
            GD.PrintErr("Failed to get SaveLoadService from GameServices");
            return;
        }

        // Connect signals
        if (_newSaveButton != null) _newSaveButton.Pressed += OnNewSavePressed;
        if (_refreshButton != null) _refreshButton.Pressed += RefreshSaveList;
        if (_loadButton != null) _loadButton.Pressed += OnLoadPressed;
        if (_deleteButton != null) _deleteButton.Pressed += OnDeletePressed;
        if (_backButton != null) _backButton.Pressed += OnBackPressed;

        // Load save list
        RefreshSaveList();
    }

    private void RefreshSaveList()
    {
        if (_saveListContainer == null || _saveLoadService == null || _detailsLabel == null || _saveEntryScene == null)
            return;

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
        if (_loadButton == null || _deleteButton == null || _saveListContainer == null || _detailsLabel == null)
            return;

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
        if (_saveLoadService == null || string.IsNullOrEmpty(_selectedSaveSlot))
            return;

        if (_saveLoadService.LoadGame(_selectedSaveSlot))
        {
            GetTree().ChangeSceneToFile("res://Scenes/UI/StarMapScreen.tscn");
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
        if (_saveLoadService == null || _loadButton == null || _deleteButton == null || string.IsNullOrEmpty(_selectedSaveSlot))
            return;

        ShowConfirmDialog($"Delete save '{_selectedSaveSlot}'?", () =>
        {
            if (_saveLoadService == null || string.IsNullOrEmpty(_selectedSaveSlot))
                return;

            _saveLoadService.DeleteSave(_selectedSaveSlot);
            _selectedSaveSlot = null;
            if (_loadButton != null) _loadButton.Disabled = true;
            if (_deleteButton != null) _deleteButton.Disabled = true;
            RefreshSaveList();
        });
    }

    private void OnBackPressed()
    {
        GetTree().ChangeSceneToFile("res://Scenes/MainMenuScreen.tscn");
    }

    private void ShowSaveNameDialog()
    {
        if (_saveLoadService == null)
            return;

        var dialog = new AcceptDialog();
        dialog.Title = "New Save";
        dialog.DialogText = "Enter save name:";

        var lineEdit = new LineEdit();
        lineEdit.PlaceholderText = "My Save";
        lineEdit.Text = $"Save {DateTime.Now:yyyy-MM-dd HH:mm}";
        dialog.AddChild(lineEdit);

        dialog.Confirmed += () =>
        {
            if (_saveLoadService == null)
                return;

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
