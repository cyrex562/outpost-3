using Godot;
using System;
using System.Collections.Generic;
using System.Linq;

namespace Outpost3.UI;
using Outpost3.Core.Domain;
using Outpost3.Core.Commands;
using Outpost3.Core.Services;
using Outpost3.Autoload;

/// <summary>
/// Presenter for the Ship Configuration screen.
/// Allows player to configure their colony ship before launching.
/// </summary>
public partial class ShipConfigurationPresenter : Control
{
    // UI Elements
    private LineEdit _shipNameInput;
    private OptionButton _shipTypeDropdown;
    private SpinBox _colonistCountInput;
    private Button _launchButton;
    private Button _backButton;

    // Cargo/Equipment
    private ItemList _cargoList;
    private ItemList _vehicleList;
    private ItemList _robotList;

    // Capacity displays
    private Label _massLabel;
    private Label _volumeLabel;
    private Label _powerLabel;
    private ProgressBar _massBar;
    private ProgressBar _volumeBar;
    private ProgressBar _powerBar;

    // Summary
    private Label _summaryLabel;
    private RichTextLabel _manifestText;

    private Ship? _currentShip;
    private const double MaxMassTons = 10000.0;
    private const double MaxVolumeM3 = 50000.0;
    private const double MaxPowerKW = 5000.0;

    public override void _Ready()
    {
        // Get UI elements
        _shipNameInput = GetNode<LineEdit>("%ShipNameInput");
        _shipTypeDropdown = GetNode<OptionButton>("%ShipTypeDropdown");
        _colonistCountInput = GetNode<SpinBox>("%ColonistCountInput");
        _launchButton = GetNode<Button>("%LaunchButton");
        _backButton = GetNode<Button>("%BackButton");

        _cargoList = GetNode<ItemList>("%CargoList");
        _vehicleList = GetNode<ItemList>("%VehicleList");
        _robotList = GetNode<ItemList>("%RobotList");

        _massLabel = GetNode<Label>("%MassLabel");
        _volumeLabel = GetNode<Label>("%VolumeLabel");
        _powerLabel = GetNode<Label>("%PowerLabel");
        _massBar = GetNode<ProgressBar>("%MassBar");
        _volumeBar = GetNode<ProgressBar>("%VolumeBar");
        _powerBar = GetNode<ProgressBar>("%PowerBar");

        _summaryLabel = GetNode<Label>("%SummaryLabel");
        _manifestText = GetNode<RichTextLabel>("%ManifestText");

        // Set up dropdowns
        _shipTypeDropdown.AddItem("Sleeper Ship", (int)ShipType.SleeperShip);
        _shipTypeDropdown.AddItem("Generation Ship", (int)ShipType.GenerationShip);

        // Set defaults
        _shipNameInput.Text = "Ark One";
        _colonistCountInput.Value = 100;
        _colonistCountInput.MinValue = 50;
        _colonistCountInput.MaxValue = 500;

        // Connect signals
        _launchButton.Pressed += OnLaunchPressed;
        _backButton.Pressed += OnBackPressed;
        _shipNameInput.TextChanged += OnShipNameChanged;
        _shipTypeDropdown.ItemSelected += OnShipTypeChanged;
        _colonistCountInput.ValueChanged += OnColonistCountChanged;

        // Initialize default ship
        CreateDefaultShip();
        UpdateDisplay();
    }

    private void CreateDefaultShip()
    {
        var command = new CreateShipCommand
        {
            ShipName = _shipNameInput.Text,
            ShipType = (ShipType)_shipTypeDropdown.GetSelectedId(),
            ColonistCount = (int)_colonistCountInput.Value,
            CargoManifest = new Dictionary<CargoType, int>(),
            VehicleTypes = new List<VehicleType>
            {
                VehicleType.Rover,
                VehicleType.Rover,
                VehicleType.Excavator,
                VehicleType.Hauler
            },
            RobotTypes = new List<RobotType>
            {
                RobotType.GeneralPurpose,
                RobotType.GeneralPurpose,
                RobotType.Constructor,
                RobotType.Miner
            }
        };

        var gameServices = GetNode<GameServices>("/root/GameServices");
        var events = command.Execute(gameServices.GameState);
        gameServices.ApplyEvents(events);

        // Get the created ship
        _currentShip = gameServices.GameState.Ships.LastOrDefault();
    }

    private void UpdateDisplay()
    {
        if (_currentShip == null) return;

        // Update capacity bars
        var mass = _currentShip.GetTotalMass();
        var volume = _currentShip.GetTotalVolume();
        var power = _currentShip.GetTotalPowerConsumption();

        _massBar.Value = (mass / MaxMassTons) * 100.0;
        _volumeBar.Value = (volume / MaxVolumeM3) * 100.0;
        _powerBar.Value = (power / MaxPowerKW) * 100.0;

        _massLabel.Text = $"Mass: {mass:F1}/{MaxMassTons:F0} tons ({_massBar.Value:F1}%)";
        _volumeLabel.Text = $"Volume: {volume:F1}/{MaxVolumeM3:F0} m³ ({_volumeBar.Value:F1}%)";
        _powerLabel.Text = $"Power: {power:F1}/{MaxPowerKW:F0} KW ({_powerBar.Value:F1}%)";

        // Update cargo list
        _cargoList.Clear();
        foreach (var cargo in _currentShip.Cargo)
        {
            _cargoList.AddItem($"{cargo.Name} x{cargo.Quantity} ({cargo.MassKg * cargo.Quantity:F0} kg)");
        }

        // Update vehicle list
        _vehicleList.Clear();
        foreach (var vehicle in _currentShip.Vehicles)
        {
            _vehicleList.AddItem($"{vehicle.Name} ({vehicle.Type})");
        }

        // Update robot list
        _robotList.Clear();
        foreach (var robot in _currentShip.Robots)
        {
            _robotList.AddItem($"{robot.Name} ({robot.Type})");
        }

        // Update summary
        _summaryLabel.Text = $"Ship: {_currentShip.Name} | Type: {_currentShip.Type} | Colonists: {_currentShip.Colonists.Count}";

        // Update manifest
        _manifestText.Clear();
        _manifestText.AppendText($"[b]Ship Manifest: {_currentShip.Name}[/b]\n\n");
        _manifestText.AppendText($"[b]Type:[/b] {_currentShip.Type}\n");
        _manifestText.AppendText($"[b]Colonists:[/b] {_currentShip.Colonists.Count}\n");
        _manifestText.AppendText($"[b]Cargo Items:[/b] {_currentShip.Cargo.Count}\n");
        _manifestText.AppendText($"[b]Vehicles:[/b] {_currentShip.Vehicles.Count}\n");
        _manifestText.AppendText($"[b]Robots:[/b] {_currentShip.Robots.Count}\n\n");

        // Role breakdown
        _manifestText.AppendText("[b]Crew Roles:[/b]\n");
        var roleGroups = _currentShip.Colonists.GroupBy(c => c.Role);
        foreach (var group in roleGroups.OrderByDescending(g => g.Count()))
        {
            _manifestText.AppendText($"  {group.Key}: {group.Count()}\n");
        }

        // Check if over capacity
        bool overCapacity = _massBar.Value > 100 || _volumeBar.Value > 100 || _powerBar.Value > 100;
        _launchButton.Disabled = overCapacity;

        if (overCapacity)
        {
            _massBar.Modulate = _massBar.Value > 100 ? Colors.Red : Colors.White;
            _volumeBar.Modulate = _volumeBar.Value > 100 ? Colors.Red : Colors.White;
            _powerBar.Modulate = _powerBar.Value > 100 ? Colors.Red : Colors.White;
        }
        else
        {
            _massBar.Modulate = Colors.Green;
            _volumeBar.Modulate = Colors.Green;
            _powerBar.Modulate = Colors.Green;
        }
    }

    private void OnShipNameChanged(string newName)
    {
        if (_currentShip != null && !string.IsNullOrWhiteSpace(newName))
        {
            var gameServices = GetNode<GameServices>("/root/GameServices");
            var updatedShip = _currentShip with { Name = newName };
            var newState = gameServices.GameState.WithShipUpdated(updatedShip);
            gameServices.SetGameState(newState);
            _currentShip = updatedShip;
            UpdateDisplay();
        }
    }

    private void OnShipTypeChanged(long index)
    {
        if (_currentShip != null)
        {
            var shipType = (ShipType)_shipTypeDropdown.GetSelectedId();
            var gameServices = GetNode<GameServices>("/root/GameServices");
            var updatedShip = _currentShip with { Type = shipType };
            var newState = gameServices.GameState.WithShipUpdated(updatedShip);
            gameServices.SetGameState(newState);
            _currentShip = updatedShip;
            UpdateDisplay();
        }
    }

    private void OnColonistCountChanged(double value)
    {
        // Recreate ship with new colonist count
        CreateDefaultShip();
        UpdateDisplay();
    }

    private void OnLaunchPressed()
    {
        if (_currentShip == null) return;

        var gameServices = GetNode<GameServices>("/root/GameServices");

        // For now, just navigate to galaxy map to select destination
        // In full implementation, this would go to a system selection screen
        GetTree().ChangeSceneToFile("res://scenes/GalaxyMapScreen.tscn");
    }

    private void OnBackPressed()
    {
        GetTree().ChangeSceneToFile("res://scenes/MainMenuScreen.tscn");
    }
}
