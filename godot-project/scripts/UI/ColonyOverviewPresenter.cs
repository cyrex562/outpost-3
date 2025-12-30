using Godot;
using System;
using System.Linq;

namespace Outpost3.UI;
using Outpost3.Core.Domain;
using Outpost3.Core.Commands;
using Outpost3.Autoload;

/// <summary>
/// Presenter for the Colony Overview screen.
/// Main dashboard showing colony status, population, resources, power, and structures.
/// </summary>
public partial class ColonyOverviewPresenter : Control
{
    // UI Elements
    private Label _colonyNameLabel;
    private Label _populationLabel;
    private Label _moraleLabel;
    private ProgressBar _moraleBar;

    // Power section
    private Label _powerStatusLabel;
    private Label _powerLabel;
    private ProgressBar _powerBar;

    // Life support section
    private Label _oxygenStatusLabel;
    private Label _oxygenLabel;
    private ProgressBar _oxygenBar;

    // Resources
    private ItemList _resourceList;
    private RichTextLabel _resourceDetailsText;

    // Structures
    private ItemList _structureList;
    private Label _structureCountLabel;

    // Colonists
    private Label _colonistCountLabel;
    private RichTextLabel _colonistRolesText;

    // Vehicles & Robots
    private Label _vehicleCountLabel;
    private Label _robotCountLabel;

    // Action buttons
    private Button _viewSurfaceMapButton;
    private Button _buildStructureButton;
    private Button _manageColonistsButton;
    private Button _backButton;

    // Build structure panel
    private Panel _buildPanel;
    private ItemList _buildableStructuresList;
    private Button _confirmBuildButton;
    private Button _cancelBuildButton;

    private Colony? _colony;
    private Ulid _colonyId;

    public override void _Ready()
    {
        // Get UI elements
        _colonyNameLabel = GetNode<Label>("%ColonyNameLabel");
        _populationLabel = GetNode<Label>("%PopulationLabel");
        _moraleLabel = GetNode<Label>("%MoraleLabel");
        _moraleBar = GetNode<ProgressBar>("%MoraleBar");

        _powerStatusLabel = GetNode<Label>("%PowerStatusLabel");
        _powerLabel = GetNode<Label>("%PowerLabel");
        _powerBar = GetNode<ProgressBar>("%PowerBar");

        _oxygenStatusLabel = GetNode<Label>("%OxygenStatusLabel");
        _oxygenLabel = GetNode<Label>("%OxygenLabel");
        _oxygenBar = GetNode<ProgressBar>("%OxygenBar");

        _resourceList = GetNode<ItemList>("%ResourceList");
        _resourceDetailsText = GetNode<RichTextLabel>("%ResourceDetailsText");

        _structureList = GetNode<ItemList>("%StructureList");
        _structureCountLabel = GetNode<Label>("%StructureCountLabel");

        _colonistCountLabel = GetNode<Label>("%ColonistCountLabel");
        _colonistRolesText = GetNode<RichTextLabel>("%ColonistRolesText");

        _vehicleCountLabel = GetNode<Label>("%VehicleCountLabel");
        _robotCountLabel = GetNode<Label>("%RobotCountLabel");

        _viewSurfaceMapButton = GetNode<Button>("%ViewSurfaceMapButton");
        _buildStructureButton = GetNode<Button>("%BuildStructureButton");
        _manageColonistsButton = GetNode<Button>("%ManageColonistsButton");
        _backButton = GetNode<Button>("%BackButton");

        _buildPanel = GetNode<Panel>("%BuildPanel");
        _buildableStructuresList = GetNode<ItemList>("%BuildableStructuresList");
        _confirmBuildButton = GetNode<Button>("%ConfirmBuildButton");
        _cancelBuildButton = GetNode<Button>("%CancelBuildButton");

        // Connect signals
        _viewSurfaceMapButton.Pressed += OnViewSurfaceMapPressed;
        _buildStructureButton.Pressed += OnBuildStructurePressed;
        _manageColonistsButton.Pressed += OnManageColonistsPressed;
        _backButton.Pressed += OnBackPressed;
        _confirmBuildButton.Pressed += OnConfirmBuildPressed;
        _cancelBuildButton.Pressed += OnCancelBuildPressed;

        _buildPanel.Visible = false;

        // Load colony data
        LoadColony();
        UpdateDisplay();

        // Set up update timer
        var timer = new Timer
        {
            WaitTime = 1.0,
            Autostart = true
        };
        timer.Timeout += UpdateDisplay;
        AddChild(timer);
    }

    private void LoadColony()
    {
        var gameServices = GetNode<GameServices>("/root/GameServices");

        // Get the first (and likely only) colony
        _colony = gameServices.GameState.Colonies.FirstOrDefault();

        if (_colony == null)
        {
            GD.PrintErr("No colony found");
            return;
        }

        _colonyId = _colony.Id;
    }

    private void UpdateDisplay()
    {
        if (_colony == null) return;

        var gameServices = GetNode<GameServices>("/root/GameServices");
        _colony = gameServices.GameState.Colonies.FirstOrDefault(c => c.Id == _colonyId);

        if (_colony == null) return;

        // Colony name and location
        _colonyNameLabel.Text = _colony.Name;

        // Population
        var colonists = gameServices.GameState.Colonists.Where(c => _colony.ColonistIds.Contains(c.Id)).ToList();
        _populationLabel.Text = $"Population: {colonists.Count}";
        _colonistCountLabel.Text = $"Total Colonists: {colonists.Count}";

        // Morale
        _moraleBar.Value = _colony.Morale;
        _moraleLabel.Text = $"Morale: {_colony.Morale}/100";
        _moraleBar.Modulate = _colony.Morale switch
        {
            >= 75 => Colors.Green,
            >= 50 => Colors.Yellow,
            >= 25 => Colors.Orange,
            _ => Colors.Red
        };

        // Power
        var powerStatus = _colony.GetPowerStatus();
        _powerStatusLabel.Text = $"Power: {powerStatus}";
        _powerLabel.Text = $"{_colony.PowerGeneration:F1} KW / {_colony.PowerConsumption:F1} KW";

        var powerPercent = _colony.PowerConsumption > 0
            ? (_colony.PowerGeneration / _colony.PowerConsumption) * 100.0
            : 100.0;
        _powerBar.Value = Math.Min(100.0, powerPercent);
        _powerBar.Modulate = powerStatus switch
        {
            PowerStatus.Surplus => Colors.Green,
            PowerStatus.Balanced => Colors.Yellow,
            PowerStatus.Deficit => Colors.Red,
            _ => Colors.Gray
        };

        // Oxygen
        var oxygenStatus = _colony.GetOxygenStatus();
        _oxygenStatusLabel.Text = $"Life Support: {oxygenStatus}";
        _oxygenLabel.Text = $"{_colony.OxygenGeneration:F1} / {_colony.OxygenConsumption:F1} units/day";

        var oxygenPercent = _colony.OxygenConsumption > 0
            ? (_colony.OxygenGeneration / _colony.OxygenConsumption) * 100.0
            : 100.0;
        _oxygenBar.Value = Math.Min(100.0, oxygenPercent);
        _oxygenBar.Modulate = oxygenStatus switch
        {
            LifeSupportStatus.Healthy => Colors.Green,
            LifeSupportStatus.Stable => Colors.Yellow,
            LifeSupportStatus.Critical => Colors.Red,
            _ => Colors.Gray
        };

        // Resources
        _resourceList.Clear();
        _resourceDetailsText.Clear();

        if (_colony.Resources.Count > 0)
        {
            _resourceDetailsText.AppendText("[b]Resource Stockpiles:[/b]\n\n");
            foreach (var resource in _colony.Resources.OrderBy(r => r.Key.ToString()))
            {
                _resourceList.AddItem($"{resource.Key}: {resource.Value:F0}");
                _resourceDetailsText.AppendText($"[b]{resource.Key}:[/b] {resource.Value:F0}\n");
            }
        }
        else
        {
            _resourceDetailsText.AppendText("No resources stockpiled.");
        }

        // Structures
        _structureList.Clear();
        _structureCountLabel.Text = $"Structures: {_colony.Structures.Count}";

        foreach (var structure in _colony.Structures)
        {
            var status = structure.IsOperational ? "✓" : "✗";
            var condition = structure.Condition;
            _structureList.AddItem($"{status} {structure.Name} ({structure.Type}) - {condition}%");
        }

        // Colonist roles
        _colonistRolesText.Clear();
        _colonistRolesText.AppendText("[b]Colonist Roles:[/b]\n\n");

        var roleGroups = colonists.GroupBy(c => c.Role).OrderByDescending(g => g.Count());
        foreach (var group in roleGroups)
        {
            _colonistRolesText.AppendText($"{group.Key}: {group.Count()}\n");
        }

        // Vehicles and robots
        _vehicleCountLabel.Text = $"Vehicles: {_colony.VehicleIds.Count}";
        _robotCountLabel.Text = $"Robots: {_colony.RobotIds.Count}";
    }

    private void OnViewSurfaceMapPressed()
    {
        GetTree().ChangeSceneToFile("res://scenes/SurfaceMapScreen.tscn");
    }

    private void OnBuildStructurePressed()
    {
        _buildPanel.Visible = true;

        // Populate buildable structures
        _buildableStructuresList.Clear();
        _buildableStructuresList.AddItem("Habitat Module");
        _buildableStructuresList.AddItem("Life Support System");
        _buildableStructuresList.AddItem("Solar Panel Array");
        _buildableStructuresList.AddItem("Power Generator");
        _buildableStructuresList.AddItem("Warehouse");
        _buildableStructuresList.AddItem("Research Lab");
        _buildableStructuresList.AddItem("Medical Center");
        _buildableStructuresList.AddItem("Mining Station");
        _buildableStructuresList.AddItem("Refinery");
        _buildableStructuresList.AddItem("Factory");
    }

    private void OnConfirmBuildPressed()
    {
        if (_colony == null) return;

        var selectedIndex = _buildableStructuresList.GetSelectedItems().FirstOrDefault(-1);
        if (selectedIndex == -1) return;

        var structureTypes = new[]
        {
            StructureType.Habitat,
            StructureType.LifeSupport,
            StructureType.SolarPanel,
            StructureType.PowerGenerator,
            StructureType.Warehouse,
            StructureType.ResearchLab,
            StructureType.MedicalCenter,
            StructureType.Mine,
            StructureType.Refinery,
            StructureType.Factory
        };

        var structureType = structureTypes[selectedIndex];
        var structureName = _buildableStructuresList.GetItemText(selectedIndex);

        var gameServices = GetNode<GameServices>("/root/GameServices");

        // Build at landing site for now (in full version, player would choose location)
        var command = new BuildStructureCommand
        {
            ColonyId = _colony.Id,
            StructureType = structureType,
            StructureName = structureName,
            SectorX = _colony.LandingSite.X,
            SectorY = _colony.LandingSite.Y
        };

        var events = command.Execute(gameServices.GameState);
        gameServices.ApplyEvents(events);

        _buildPanel.Visible = false;
        UpdateDisplay();
    }

    private void OnCancelBuildPressed()
    {
        _buildPanel.Visible = false;
    }

    private void OnManageColonistsPressed()
    {
        // Placeholder - would navigate to colonist management screen
        var dialog = new AcceptDialog
        {
            DialogText = "Colonist management screen coming soon!",
            Title = "Manage Colonists"
        };
        AddChild(dialog);
        dialog.PopupCentered();
    }

    private void OnBackPressed()
    {
        GetTree().ChangeSceneToFile("res://scenes/MainMenuScreen.tscn");
    }
}
