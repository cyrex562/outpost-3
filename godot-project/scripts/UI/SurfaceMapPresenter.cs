using Godot;
using System;
using System.Collections.Generic;
using System.Linq;

namespace Outpost3.UI;
using Outpost3.Core.Domain;
using Outpost3.Core.Commands;
using Outpost3.Autoload;

/// <summary>
/// Presenter for the Surface Map screen.
/// Displays a grid-based planetary surface map with sectors.
/// </summary>
public partial class SurfaceMapPresenter : Control
{
    // UI Elements
    private GridContainer _sectorGrid;
    private Panel _selectedSectorPanel;
    private Label _selectedSectorLabel;
    private RichTextLabel _sectorDetailsText;
    private Button _selectLandingSiteButton;
    private Button _landButton;
    private Button _backButton;

    // Camera controls
    private Control _mapContainer;
    private Vector2 _cameraOffset = Vector2.Zero;
    private float _zoomLevel = 1.0f;
    private bool _isDragging = false;
    private Vector2 _dragStart = Vector2.Zero;

    // State
    private Ulid _bodyId;
    private SurfaceMap? _surfaceMap;
    private SurfaceSector? _selectedSector;
    private Ship? _currentShip;

    // Display settings
    private const int SectorSize = 32; // pixels
    private OptionButton _layerDropdown;
    private string _currentLayer = "Terrain";

    public override void _Ready()
    {
        // Get UI elements
        _sectorGrid = GetNode<GridContainer>("%SectorGrid");
        _selectedSectorPanel = GetNode<Panel>("%SelectedSectorPanel");
        _selectedSectorLabel = GetNode<Label>("%SelectedSectorLabel");
        _sectorDetailsText = GetNode<RichTextLabel>("%SectorDetailsText");
        _selectLandingSiteButton = GetNode<Button>("%SelectLandingSiteButton");
        _landButton = GetNode<Button>("%LandButton");
        _backButton = GetNode<Button>("%BackButton");
        _mapContainer = GetNode<Control>("%MapContainer");
        _layerDropdown = GetNode<OptionButton>("%LayerDropdown");

        // Set up layer dropdown
        _layerDropdown.AddItem("Terrain");
        _layerDropdown.AddItem("Resources");
        _layerDropdown.AddItem("Hazards");
        _layerDropdown.AddItem("Suitability");
        _layerDropdown.AddItem("Elevation");

        // Connect signals
        _selectLandingSiteButton.Pressed += OnSelectLandingSitePressed;
        _landButton.Pressed += OnLandPressed;
        _backButton.Pressed += OnBackPressed;
        _layerDropdown.ItemSelected += OnLayerChanged;

        // Initialize
        LoadSurfaceMap();
        RenderMap();

        _selectedSectorPanel.Visible = false;
        _selectLandingSiteButton.Visible = false;
        _landButton.Visible = false;
    }

    private void LoadSurfaceMap()
    {
        var gameServices = GetNode<GameServices>("/root/GameServices");

        // Get the current ship (should be in orbit)
        _currentShip = gameServices.GameState.Ships.FirstOrDefault();
        if (_currentShip == null)
        {
            GD.PrintErr("No ship found");
            return;
        }

        if (_currentShip.Location.State != ShipLocationState.InOrbit)
        {
            GD.PrintErr("Ship is not in orbit");
            return;
        }

        _bodyId = _currentShip.Location.OrbitBodyId!.Value;

        // Check if surface map exists
        if (!gameServices.GameState.SurfaceMaps.TryGetValue(_bodyId, out _surfaceMap))
        {
            // Generate surface map
            var command = new GenerateSurfaceMapCommand
            {
                BodyId = _bodyId,
                Width = 32,
                Height = 24
            };

            var events = command.Execute(gameServices.GameState);
            gameServices.ApplyEvents(events);

            // Get the generated map
            _surfaceMap = gameServices.GameState.SurfaceMaps[_bodyId];
        }
    }

    private void RenderMap()
    {
        if (_surfaceMap == null) return;

        _sectorGrid.Columns = _surfaceMap.Width;
        _sectorGrid.ClearChildren();

        for (int y = 0; y < _surfaceMap.Height; y++)
        {
            for (int x = 0; x < _surfaceMap.Width; x++)
            {
                var sector = _surfaceMap.GetSector(x, y);
                if (sector == null) continue;

                var sectorButton = CreateSectorButton(sector);
                _sectorGrid.AddChild(sectorButton);
            }
        }
    }

    private Button CreateSectorButton(SurfaceSector sector)
    {
        var button = new Button
        {
            CustomMinimumSize = new Vector2(SectorSize, SectorSize),
            TooltipText = GetSectorTooltip(sector)
        };

        // Set color based on current layer
        button.Modulate = GetSectorColor(sector, _currentLayer);

        // Connect click event
        button.Pressed += () => OnSectorClicked(sector);

        return button;
    }

    private Color GetSectorColor(SurfaceSector sector, string layer)
    {
        return layer switch
        {
            "Terrain" => GetTerrainColor(sector.Terrain),
            "Resources" => GetResourceColor(sector.ResourceDeposits.Count),
            "Hazards" => GetHazardColor(sector.Hazards.Sum(h => h.Severity)),
            "Suitability" => GetSuitabilityColor(sector.SuitabilityScore),
            "Elevation" => GetElevationColor(sector.Elevation),
            _ => Colors.Gray
        };
    }

    private Color GetTerrainColor(TerrainType terrain)
    {
        return terrain switch
        {
            TerrainType.Plains => new Color(0.8f, 0.9f, 0.6f),
            TerrainType.Hills => new Color(0.7f, 0.7f, 0.5f),
            TerrainType.Mountains => new Color(0.5f, 0.5f, 0.5f),
            TerrainType.Crater => new Color(0.6f, 0.5f, 0.4f),
            TerrainType.Valley => new Color(0.6f, 0.8f, 0.5f),
            TerrainType.Canyon => new Color(0.7f, 0.5f, 0.3f),
            TerrainType.Desert => new Color(0.9f, 0.8f, 0.6f),
            TerrainType.Polar => new Color(0.9f, 0.95f, 1.0f),
            TerrainType.Volcanic => new Color(0.6f, 0.3f, 0.2f),
            TerrainType.Lava => new Color(1.0f, 0.3f, 0.1f),
            TerrainType.Rocky => new Color(0.6f, 0.6f, 0.6f),
            TerrainType.SaltFlat => new Color(0.95f, 0.95f, 0.9f),
            TerrainType.Dunes => new Color(0.9f, 0.7f, 0.5f),
            _ => Colors.Gray
        };
    }

    private Color GetResourceColor(int resourceCount)
    {
        return resourceCount switch
        {
            0 => new Color(0.3f, 0.3f, 0.3f),
            1 => new Color(0.5f, 0.8f, 0.5f),
            >= 2 => new Color(0.2f, 1.0f, 0.2f),
            _ => Colors.Gray
        };
    }

    private Color GetHazardColor(int totalSeverity)
    {
        if (totalSeverity == 0) return Colors.Green;
        if (totalSeverity < 30) return Colors.Yellow;
        if (totalSeverity < 60) return Colors.Orange;
        return Colors.Red;
    }

    private Color GetSuitabilityColor(int suitability)
    {
        // Green gradient: dark green (low) to bright green (high)
        var t = suitability / 100.0f;
        return new Color(0.2f, t * 0.8f + 0.2f, 0.2f);
    }

    private Color GetElevationColor(int elevation)
    {
        // Blue (low) to white (high)
        var normalized = (elevation + 100) / 600.0f; // Normalize -100 to 500
        normalized = Mathf.Clamp(normalized, 0, 1);
        return new Color(normalized, normalized, 1.0f);
    }

    private string GetSectorTooltip(SurfaceSector sector)
    {
        var coords = $"({sector.Coordinates.X}, {sector.Coordinates.Y})";
        var terrain = sector.Terrain.ToString();
        var suitability = $"{sector.SuitabilityScore}%";
        var resources = sector.ResourceDeposits.Count > 0 ? $"{sector.ResourceDeposits.Count} resource(s)" : "No resources";
        var hazards = sector.Hazards.Count > 0 ? $"{sector.Hazards.Count} hazard(s)" : "No hazards";

        return $"{coords} | {terrain} | Suitability: {suitability}\n{resources} | {hazards}";
    }

    private void OnSectorClicked(SurfaceSector sector)
    {
        _selectedSector = sector;
        _selectedSectorPanel.Visible = true;
        _selectLandingSiteButton.Visible = true;

        _selectedSectorLabel.Text = $"Sector ({sector.Coordinates.X}, {sector.Coordinates.Y})";

        _sectorDetailsText.Clear();
        _sectorDetailsText.AppendText($"[b]Terrain:[/b] {sector.Terrain}\n");
        _sectorDetailsText.AppendText($"[b]Elevation:[/b] {sector.Elevation}m\n");
        _sectorDetailsText.AppendText($"[b]Temperature:[/b] {sector.Temperature}°C\n");
        _sectorDetailsText.AppendText($"[b]Suitability:[/b] {sector.SuitabilityScore}/100\n\n");

        if (sector.ResourceDeposits.Count > 0)
        {
            _sectorDetailsText.AppendText("[b]Resources:[/b]\n");
            foreach (var deposit in sector.ResourceDeposits)
            {
                _sectorDetailsText.AppendText($"  • {deposit.ResourceType} (Richness: {deposit.Richness})\n");
            }
            _sectorDetailsText.AppendText("\n");
        }

        if (sector.Hazards.Count > 0)
        {
            _sectorDetailsText.AppendText("[b]Hazards:[/b]\n");
            foreach (var hazard in sector.Hazards)
            {
                _sectorDetailsText.AppendText($"  • {hazard.Type} (Severity: {hazard.Severity})\n");
            }
        }
    }

    private void OnSelectLandingSitePressed()
    {
        if (_selectedSector == null || _currentShip == null) return;

        var gameServices = GetNode<GameServices>("/root/GameServices");

        var command = new SelectLandingSiteCommand
        {
            ShipId = _currentShip.Id,
            BodyId = _bodyId,
            SectorX = _selectedSector.Coordinates.X,
            SectorY = _selectedSector.Coordinates.Y
        };

        var events = command.Execute(gameServices.GameState);
        gameServices.ApplyEvents(events);

        // Show land button
        _landButton.Visible = true;
        _selectLandingSiteButton.Text = "Landing Site Selected ✓";
    }

    private void OnLandPressed()
    {
        if (_selectedSector == null || _currentShip == null) return;

        var gameServices = GetNode<GameServices>("/root/GameServices");

        // Execute landing
        var landCommand = new LandShipCommand
        {
            ShipId = _currentShip.Id,
            BodyId = _bodyId,
            SectorX = _selectedSector.Coordinates.X,
            SectorY = _selectedSector.Coordinates.Y
        };

        var events = landCommand.Execute(gameServices.GameState);
        gameServices.ApplyEvents(events);

        // Check if landing was successful
        var landedEvent = events.OfType<ShipLandedEvent>().FirstOrDefault();
        var failedEvent = events.OfType<LandingFailedEvent>().FirstOrDefault();

        if (landedEvent != null)
        {
            // Successful landing - establish colony
            var colonyCommand = new EstablishColonyCommand
            {
                ShipId = _currentShip.Id,
                ColonyName = $"{_currentShip.Name} Colony",
                BodyId = _bodyId,
                SystemId = _currentShip.Location.TargetSystemId!.Value,
                SectorX = _selectedSector.Coordinates.X,
                SectorY = _selectedSector.Coordinates.Y
            };

            var colonyEvents = colonyCommand.Execute(gameServices.GameState);
            gameServices.ApplyEvents(colonyEvents);

            // Navigate to colony overview
            GetTree().ChangeSceneToFile("res://scenes/ColonyOverviewScreen.tscn");
        }
        else if (failedEvent != null)
        {
            // Show failure message
            var dialog = new AcceptDialog
            {
                DialogText = $"Landing failed: {failedEvent.Reason}\nCasualties: {failedEvent.ColonistCasualties}\nCargo lost: {failedEvent.CargoLoss}%",
                Title = "Landing Failed"
            };
            AddChild(dialog);
            dialog.PopupCentered();
        }
    }

    private void OnLayerChanged(long index)
    {
        _currentLayer = _layerDropdown.GetItemText((int)index);
        RenderMap();
    }

    private void OnBackPressed()
    {
        GetTree().ChangeSceneToFile("res://scenes/StarSystemMapScreen.tscn");
    }

    // Camera controls for panning/zooming
    public override void _Input(InputEvent @event)
    {
        if (@event is InputEventMouseButton mouseButton)
        {
            if (mouseButton.ButtonIndex == MouseButton.Middle)
            {
                _isDragging = mouseButton.Pressed;
                if (_isDragging)
                {
                    _dragStart = mouseButton.Position;
                }
            }
            else if (mouseButton.ButtonIndex == MouseButton.WheelUp)
            {
                _zoomLevel = Mathf.Clamp(_zoomLevel * 1.1f, 0.5f, 3.0f);
                _mapContainer.Scale = new Vector2(_zoomLevel, _zoomLevel);
            }
            else if (mouseButton.ButtonIndex == MouseButton.WheelDown)
            {
                _zoomLevel = Mathf.Clamp(_zoomLevel / 1.1f, 0.5f, 3.0f);
                _mapContainer.Scale = new Vector2(_zoomLevel, _zoomLevel);
            }
        }
        else if (@event is InputEventMouseMotion mouseMotion && _isDragging)
        {
            var delta = mouseMotion.Position - _dragStart;
            _cameraOffset += delta;
            _mapContainer.Position = _cameraOffset;
            _dragStart = mouseMotion.Position;
        }
    }
}
