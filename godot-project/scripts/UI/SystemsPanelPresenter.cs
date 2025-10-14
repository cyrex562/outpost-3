using System;
using System.Linq;
using Godot;
using Outpost3.Core;
using Outpost3.Core.Commands;
using Outpost3.Core.Domain;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the systems list panel.
/// Subscribes to StateStore and displays discovered systems.
/// </summary>
public partial class SystemsPanelPresenter : MarginContainer
{
    [Export] public PackedScene SystemListItemScene { get; set; }

    private VBoxContainer _systemListContainer;
    private StateStore _stateStore;
    private Ulid? _currentSelectedSystemId;
    private GalaxyMapSystemDetailsModalPresenter _systemDetailsModal;

    public override void _Ready()
    {
        // Get node references
        _systemListContainer = GetNode<VBoxContainer>("PanelContainer/MarginContainer/VBoxContainer/ScrollContainer/SystemListContainer");

        // Get StateStore from GameServices autoload
        var gameServices = GetNode<GameServices>("/root/GameServices");
        _stateStore = gameServices.StateStore;

        // Create and add system details modal to the scene tree root
        _systemDetailsModal = GD.Load<PackedScene>("res://Scenes/UI/SystemDetailsModal.tscn").Instantiate<GalaxyMapSystemDetailsModalPresenter>();
        GetTree().Root.AddChild(_systemDetailsModal);

        // Subscribe to state changes
        _stateStore.StateChanged += OnStateChanged;

        // Initial render
        RefreshSystemsList();
    }

    public override void _ExitTree()
    {
        if (_stateStore != null)
        {
            _stateStore.StateChanged -= OnStateChanged;
        }

        if (_systemDetailsModal != null && !_systemDetailsModal.IsQueuedForDeletion())
        {
            _systemDetailsModal.QueueFree();
        }
    }

    private void OnStateChanged()
    {
        var state = _stateStore.State;

        // Check if selection changed
        if (state.SelectedSystemId != _currentSelectedSystemId)
        {
            _currentSelectedSystemId = state.SelectedSystemId;

            // Update visual selection
            UpdateSelectionVisuals();

            // Show details modal if system selected
            if (_currentSelectedSystemId.HasValue)
            {
                var system = state.Systems.FirstOrDefault(s => s.Id == _currentSelectedSystemId.Value);
                if (system != null)
                {
                    _systemDetailsModal.ShowSystem(system);
                }
            }
        }

        // Refresh list if systems added/changed
        RefreshSystemsList();
    }

    private void RefreshSystemsList()
    {
        var state = _stateStore.State;
        var existingItems = _systemListContainer.GetChildren()
            .OfType<SystemListItemComponent>()
            .ToList();

        // Sort systems by name for now (TODO: by distance when we have positions)
        var sortedSystems = state.Systems.OrderBy(s => s.Name).ToList();

        // Remove excess items
        while (existingItems.Count > sortedSystems.Count)
        {
            var lastItem = existingItems[^1];
            existingItems.RemoveAt(existingItems.Count - 1);
            lastItem.QueueFree();
        }

        // Update or create items
        for (int i = 0; i < sortedSystems.Count; i++)
        {
            var system = sortedSystems[i];

            SystemListItemComponent item;
            if (i < existingItems.Count)
            {
                item = existingItems[i];
            }
            else
            {
                item = SystemListItemScene.Instantiate<SystemListItemComponent>();
                item.SystemClicked += OnSystemClicked;
                _systemListContainer.AddChild(item);
            }

            item.SetSystemData(system);
            item.SetSelected(system.Id == _currentSelectedSystemId);
        }
    }

    private void UpdateSelectionVisuals()
    {
        foreach (var item in _systemListContainer.GetChildren().OfType<SystemListItemComponent>())
        {
            item.SetSelected(item.SystemId == _currentSelectedSystemId);
        }
    }

    private void OnSystemClicked(string systemIdString)
    {
        // Parse string back to Ulid (Godot signal limitation)
        if (Ulid.TryParse(systemIdString, out var systemId))
        {
            var command = new SelectSystemCommand(systemId);
            _stateStore.ApplyCommand(command);
        }
    }
}
