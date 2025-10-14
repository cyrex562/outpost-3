using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using Outpost3.Core;
using Outpost3.Core.Commands;
using Outpost3.Core.Domain;
using Outpost3.Core.Projections;
using Outpost3.UI.Common;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the Star System Map screen.
/// Subscribes to StateStore and renders the procedurally generated system.
/// Handles camera controls, body selection, and orbital visualization.
/// </summary>
public partial class StarSystemMapPresenter : Control
{
    // Scene node references - mapped to StarSystemMapScreen.tscn
    private SubViewport _subViewport = null!;
    private Camera2D _camera = null!;
    private Node2D _starSystemView = null!;
    private Sprite2D _star = null!;
    private Node2D _oortCloudVisual = null!;
    private Node2D _beltsContainer = null!;
    private Node2D _orbitsContainer = null!;
    private Node2D _bodiesContainer = null!;
    private Sprite2D _selectionRing = null!;

    // Top bar
    private Label _systemNameLabel = null!;
    private Label _spectralInfoLabel = null!;
    private Label _zoomLabel = null!;
    private Label _centerLabel = null!;
    private Label _gameTimeLabel = null!;
    private Label _gameSpeedLabel = null!;
    private Button _slowerButton = null!;
    private Button _pauseButton = null!;
    private Button _fasterButton = null!;

    // Bottom bar
    private Button _backButton = null!;
    private Button _resetCameraButton = null!;
    private Button _toggleOverviewButton = null!;

    // System overview panel
    private PanelContainer _systemOverviewPanel = null!;
    private VBoxContainer _bodiesListContainer = null!;
    private VBoxContainer _messagesListContainer = null!;

    // Tooltip
    private PanelContainer _tooltipPanel = null!;
    private Label _tooltipBodyNameLabel = null!;
    private Label _tooltipBodyTypeLabel = null!;
    private Label _tooltipDistanceLabel = null!;
    private Label _tooltipMassLabel = null!;
    private Label _tooltipPeriodLabel = null!;

    // State
    private StateStore _stateStore = null!;
    private StarSystemMapViewModel? _currentViewModel;
    private Dictionary<Ulid, Node2D> _bodyNodes = new();
    private Ulid? _hoveredBodyId;

    // Shared utilities
    private CameraController? _cameraController;
    private TimeManager? _timeManager;

    // Constants
    private const float MIN_ZOOM = 0.1f;
    private const float MAX_ZOOM = 5.0f;
    private const float ZOOM_STEP = 0.1f;
    private const float BASE_PIXELS_PER_AU = 100.0f; // 1 AU = 100 pixels at zoom 1.0

    public override void _Ready()
    {
        GetNodeReferences();
        GetStateStore();
        WireUpSignals();

        // Initialize shared utilities
        _cameraController = new CameraController(_camera, _subViewport, _stateStore);
        _cameraController.MinZoom = MIN_ZOOM;
        _cameraController.MaxZoom = MAX_ZOOM;
        _cameraController.ZoomStep = ZOOM_STEP;

        // Use System context for faster day/month progression but not extreme galaxy speeds
        _timeManager = new TimeManager(_stateStore, TimeScaleContext.System);

        // Subscribe to state changes
        _stateStore.StateChanged += OnStateChanged;

        // Trigger initial generation and navigation
        TriggerSystemGeneration();

        // Initial render
        OnStateChanged();
    }

    public override void _ExitTree()
    {
        if (_stateStore != null)
        {
            _stateStore.StateChanged -= OnStateChanged;
        }
    }

    private void GetNodeReferences()
    {
        // Viewport
        _subViewport = GetNode<SubViewport>("ViewportContainer/SubViewport");
        _camera = GetNode<Camera2D>("ViewportContainer/SubViewport/Camera2D");
        _starSystemView = GetNode<Node2D>("ViewportContainer/SubViewport/StarSystemView");
        _star = GetNode<Sprite2D>("ViewportContainer/SubViewport/StarSystemView/Star");
        _oortCloudVisual = GetNode<Node2D>("ViewportContainer/SubViewport/StarSystemView/OortCloudVisual");
        _beltsContainer = GetNode<Node2D>("ViewportContainer/SubViewport/StarSystemView/BeltsContainer");
        _orbitsContainer = GetNode<Node2D>("ViewportContainer/SubViewport/StarSystemView/OrbitsContainer");
        _bodiesContainer = GetNode<Node2D>("ViewportContainer/SubViewport/StarSystemView/BodiesContainer");
        _selectionRing = GetNode<Sprite2D>("ViewportContainer/SubViewport/StarSystemView/SelectionRing");

        // Top bar
        _systemNameLabel = GetNode<Label>("TopBar/MarginContainer/HBoxContainer/SystemInfoVBox/SystemNameLabel");
        _spectralInfoLabel = GetNode<Label>("TopBar/MarginContainer/HBoxContainer/SystemInfoVBox/SpectralInfoLabel");
        _zoomLabel = GetNode<Label>("TopBar/MarginContainer/HBoxContainer/CameraInfoVBox/ZoomLabel");
        _centerLabel = GetNode<Label>("TopBar/MarginContainer/HBoxContainer/CameraInfoVBox/CenterLabel");
        _gameTimeLabel = GetNode<Label>("TopBar/MarginContainer/HBoxContainer/TimeControlsVBox/GameTimeLabel");
        _gameSpeedLabel = GetNode<Label>("TopBar/MarginContainer/HBoxContainer/TimeControlsVBox/TimeControlsHBox/GameSpeedLabel");
        _slowerButton = GetNode<Button>("TopBar/MarginContainer/HBoxContainer/TimeControlsVBox/TimeControlsHBox/SlowerButton");
        _pauseButton = GetNode<Button>("TopBar/MarginContainer/HBoxContainer/TimeControlsVBox/TimeControlsHBox/PauseButton");
        _fasterButton = GetNode<Button>("TopBar/MarginContainer/HBoxContainer/TimeControlsVBox/TimeControlsHBox/FasterButton");

        // Bottom bar
        _backButton = GetNode<Button>("BottomBar/MarginContainer/HBoxContainer/BackButton");
        _resetCameraButton = GetNode<Button>("BottomBar/MarginContainer/HBoxContainer/ResetCameraButton");
        _toggleOverviewButton = GetNode<Button>("BottomBar/MarginContainer/HBoxContainer/ToggleOverviewButton");

        // System overview panel
        _systemOverviewPanel = GetNode<PanelContainer>("SystemOverviewPanel");
        _bodiesListContainer = GetNode<VBoxContainer>("SystemOverviewPanel/MarginContainer/VBoxContainer/BodiesScrollContainer/BodiesListContainer");
        _messagesListContainer = GetNode<VBoxContainer>("SystemOverviewPanel/MarginContainer/VBoxContainer/MessagesScrollContainer/MessagesListContainer");

        // Tooltip
        _tooltipPanel = GetNode<PanelContainer>("TooltipPanel");
        _tooltipBodyNameLabel = GetNode<Label>("TooltipPanel/MarginContainer/VBoxContainer/BodyNameLabel");
        _tooltipBodyTypeLabel = GetNode<Label>("TooltipPanel/MarginContainer/VBoxContainer/BodyTypeLabel");
        _tooltipDistanceLabel = GetNode<Label>("TooltipPanel/MarginContainer/VBoxContainer/DistanceLabel");
        _tooltipMassLabel = GetNode<Label>("TooltipPanel/MarginContainer/VBoxContainer/MassLabel");
        _tooltipPeriodLabel = GetNode<Label>("TooltipPanel/MarginContainer/VBoxContainer/PeriodLabel");
    }

    private void GetStateStore()
    {
        var gameServices = GetNode<GameServices>("/root/GameServices");
        _stateStore = gameServices.StateStore;
    }

    private void WireUpSignals()
    {
        _backButton.Pressed += OnBackPressed;
        _resetCameraButton.Pressed += OnResetCameraPressed;
        _toggleOverviewButton.Pressed += OnToggleOverviewPressed;
        _pauseButton.Pressed += OnPausePressed;
        _slowerButton.Pressed += OnSlowerPressed;
        _fasterButton.Pressed += OnFasterPressed;
    }

    private void TriggerSystemGeneration()
    {
        var state = _stateStore.State;

        // If a system is selected, generate its details (if not already generated)
        if (state.SelectedSystemId.HasValue)
        {
            var selectedSystem = state.Systems.FirstOrDefault(s => s.Id == state.SelectedSystemId.Value);

            // Only generate if system doesn't have details yet
            if (selectedSystem != null && selectedSystem.Seed == null)
            {
                _stateStore.ApplyCommand(new GenerateSystemDetails(state.SelectedSystemId.Value, state.GameTime));
            }
        }
    }

    private void OnStateChanged()
    {
        GD.Print("=== StarSystemMapPresenter.OnStateChanged() START ===");

        var state = _stateStore.State;
        GD.Print($"State: SelectedSystemId = {state.SelectedSystemId}");
        GD.Print($"State: Systems count = {state.Systems.Count}");

        var viewModel = StarSystemMapProjection.Project(state);

        if (viewModel == null)
        {
            // No system selected - shouldn't happen, but handle gracefully
            GD.PrintErr("StarSystemMapPresenter: No system selected! ViewModel is null");
            GD.PrintErr($"  SelectedSystemId.HasValue = {state.SelectedSystemId.HasValue}");
            if (state.SelectedSystemId.HasValue)
            {
                var selectedSystem = state.Systems.FirstOrDefault(s => s.Id == state.SelectedSystemId.Value);
                GD.PrintErr($"  Selected system found = {selectedSystem != null}");
                if (selectedSystem != null)
                {
                    GD.PrintErr($"  System.Seed = {selectedSystem.Seed}");
                    GD.PrintErr($"  System.Bodies count = {selectedSystem.Bodies.Count}");
                }
            }
            return;
        }

        GD.Print($"ViewModel: SystemName = {viewModel.SystemName}");
        GD.Print($"ViewModel: Bodies count = {viewModel.Bodies.Count}");
        GD.Print($"ViewModel: Belts count = {viewModel.Belts.Count}");
        GD.Print($"ViewModel: OortCloud = {viewModel.OortCloud != null}");

        _currentViewModel = viewModel;

        // Update UI elements
        GD.Print("Calling UpdateHeader...");
        UpdateHeader();
        GD.Print("Calling UpdateCamera...");
        UpdateCamera();
        GD.Print("Calling UpdateSystemVisuals...");
        UpdateSystemVisuals();
        GD.Print("Calling UpdateOverviewPanel...");
        UpdateOverviewPanel();

        GD.Print("=== StarSystemMapPresenter.OnStateChanged() END ===");
    }

    private void UpdateHeader()
    {
        if (_currentViewModel == null) return;

        _systemNameLabel.Text = $"{_currentViewModel.SystemName} ({_currentViewModel.SpectralClass}-class)";
        _spectralInfoLabel.Text = $"{DisplayFormatter.FormatSpectralClass(_currentViewModel.SpectralClass)} - {DisplayFormatter.FormatLuminosity(_currentViewModel.StarLuminosity)}";
        _gameTimeLabel.Text = DisplayFormatter.FormatGameTime(_currentViewModel.GameTime);

        if (_currentViewModel.IsPaused)
        {
            _gameSpeedLabel.Text = "PAUSED";
            _pauseButton.Text = "Resume";
        }
        else
        {
            // Show contextual speed labels for system map
            _gameSpeedLabel.Text = DisplayFormatter.FormatGameSpeedWithContext(_currentViewModel.CurrentSpeed, isGalaxyContext: false);
            _pauseButton.Text = "Pause";
        }

        _zoomLabel.Text = $"Zoom: {_cameraController?.CurrentZoom:F2}x";
        _centerLabel.Text = $"Center: {_camera.Position.X / BASE_PIXELS_PER_AU:F1}, {_camera.Position.Y / BASE_PIXELS_PER_AU:F1} AU";
    }

    private void UpdateCamera()
    {
        if (_currentViewModel == null || _cameraController == null) return;

        // Apply camera state from view model
        var cameraState = _currentViewModel.CameraState;
        _cameraController.SetPositionAndZoom(cameraState.PanPosition, cameraState.ZoomLevel);
    }

    private void UpdateSystemVisuals()
    {
        GD.Print("=== UpdateSystemVisuals START ===");

        if (_currentViewModel == null)
        {
            GD.PrintErr("UpdateSystemVisuals: _currentViewModel is null!");
            return;
        }

        GD.Print($"Star node: {_star != null}");
        GD.Print($"StarColor: {_currentViewModel.StarColor}");

        // Update star
        if (_star != null)
        {
            _star.Modulate = _currentViewModel.StarColor;
            // Ensure star has a visible representation
            if (_star.GetChildCount() == 0)
            {
                // Add a filled circle for the star
                var starCircle = new Polygon2D();
                starCircle.Color = Colors.White; // Will be modulated by parent

                // Use shared utility for star size calculation
                var starRadius = CelestialRenderingUtils.CalculateStarRadius(_currentViewModel.StarLuminosity);
                starCircle.Polygon = CelestialRenderingUtils.CreateCirclePolygon(starRadius, 24);
                _star.AddChild(starCircle);
            }
        }
        else
        {
            GD.PrintErr("_star node is null!");
        }

        // Clear existing bodies
        GD.Print($"Clearing {_bodyNodes.Count} existing body nodes...");
        foreach (var node in _bodyNodes.Values)
        {
            node.QueueFree();
        }
        _bodyNodes.Clear();

        // Clear orbits and belts
        GD.Print($"Clearing orbit/belt/oort containers...");
        foreach (var child in _orbitsContainer.GetChildren())
        {
            child.QueueFree();
        }
        foreach (var child in _beltsContainer.GetChildren())
        {
            child.QueueFree();
        }
        foreach (var child in _oortCloudVisual.GetChildren())
        {
            child.QueueFree();
        }

        // Render Oort cloud (if present)
        if (_currentViewModel.OortCloud != null)
        {
            GD.Print($"Rendering Oort cloud at {_currentViewModel.OortCloud.RadiusAU} AU");
            RenderOortCloud(_currentViewModel.OortCloud);
        }

        // Render asteroid belts
        GD.Print($"Rendering {_currentViewModel.Belts.Count} asteroid belts...");
        foreach (var belt in _currentViewModel.Belts)
        {
            GD.Print($"  Belt: {belt.InnerRadiusAU} - {belt.OuterRadiusAU} AU");
            RenderAsteroidBelt(belt);
        }

        // Render orbits and bodies
        GD.Print($"Rendering {_currentViewModel.Bodies.Count} bodies...");
        foreach (var body in _currentViewModel.Bodies)
        {
            // Skip asteroid belt bodies - they're rendered as belts, not individual bodies
            if (body.BodyType.Contains("Asteroid Belt", StringComparison.OrdinalIgnoreCase))
            {
                GD.Print($"  Skipping belt body: {body.Name}");
                continue;
            }

            GD.Print($"  Body: {body.Name} at {body.OrbitalParams?.SemiMajorAxisAU} AU");
            if (body.OrbitalParams != null)
            {
                RenderOrbit(body);
            }
            RenderBody(body);
        }

        GD.Print("=== UpdateSystemVisuals END ===");
    }

    private void RenderOortCloud(OortCloudViewModel oortCloud)
    {
        var circle = new Line2D();
        circle.DefaultColor = oortCloud.Color;
        circle.Width = 3.0f; // Thicker line for visibility

        var radius = (float)(oortCloud.RadiusAU * BASE_PIXELS_PER_AU);
        var segments = 64;

        for (int i = 0; i <= segments; i++)
        {
            var angle = i * Mathf.Pi * 2.0f / segments;
            var point = new Vector2(Mathf.Cos(angle), Mathf.Sin(angle)) * radius;
            circle.AddPoint(point);
        }

        _oortCloudVisual.AddChild(circle);
    }

    private void RenderAsteroidBelt(AsteroidBeltViewModel belt)
    {
        // Render as filled annulus (ring shape)
        var innerRadius = (float)(belt.InnerRadiusAU * BASE_PIXELS_PER_AU);
        var outerRadius = (float)(belt.OuterRadiusAU * BASE_PIXELS_PER_AU);

        // Create a filled ring using Polygon2D
        var ring = new Polygon2D();
        ring.Color = belt.Color;

        var segments = 64;
        var points = new List<Vector2>();

        // Outer circle
        for (int i = 0; i <= segments; i++)
        {
            var angle = i * Mathf.Pi * 2.0f / segments;
            points.Add(new Vector2(Mathf.Cos(angle), Mathf.Sin(angle)) * outerRadius);
        }

        // Inner circle (reverse direction to create hole)
        for (int i = segments; i >= 0; i--)
        {
            var angle = i * Mathf.Pi * 2.0f / segments;
            points.Add(new Vector2(Mathf.Cos(angle), Mathf.Sin(angle)) * innerRadius);
        }

        ring.Polygon = points.ToArray();
        _beltsContainer.AddChild(ring);
    }

    private void RenderOrbit(CelestialBodyViewModel body)
    {
        if (body.OrbitalParams == null) return;

        var radius = (float)(body.OrbitalParams.SemiMajorAxisAU * BASE_PIXELS_PER_AU);
        var color = new Color(0.4f, 0.4f, 0.4f, 0.8f); // Gray, more opaque

        RenderCircle(_orbitsContainer, radius, color);
    }

    private void RenderCircle(Node2D parent, float radius, Color color)
    {
        var circle = new Line2D();
        circle.DefaultColor = color;
        circle.Width = 1.5f; // Slightly thicker for visibility

        var segments = 64;
        for (int i = 0; i <= segments; i++)
        {
            var angle = i * Mathf.Pi * 2.0f / segments;
            var point = new Vector2(Mathf.Cos(angle), Mathf.Sin(angle)) * radius;
            circle.AddPoint(point);
        }

        parent.AddChild(circle);
    }

    private void RenderBody(CelestialBodyViewModel body)
    {
        // Create a simple circular sprite for the body
        var bodyNode = new Node2D();
        bodyNode.Name = body.Name;

        // Create visual representation using Polygon2D (filled circle)
        var circle = new Polygon2D();
        circle.Color = body.Color;

        // Use shared utility for visual radius calculation
        var visualRadius = CelestialRenderingUtils.CalculateVisualRadius(body.BodyType, body.RadiusKm);

        // Use shared utility for circle polygon
        circle.Polygon = CelestialRenderingUtils.CreateCirclePolygon(visualRadius);

        bodyNode.AddChild(circle);

        // Position at current orbital position
        if (body.OrbitalParams != null)
        {
            var position = OrbitalMath.CalculateOrbitalPosition(body.OrbitalParams, _currentViewModel!.GameTime);
            bodyNode.Position = position * BASE_PIXELS_PER_AU;
        }

        _bodiesContainer.AddChild(bodyNode);
        _bodyNodes[body.Id] = bodyNode;
    }



    private void UpdateOverviewPanel()
    {
        if (_currentViewModel == null) return;

        _systemOverviewPanel.Visible = _currentViewModel.OverviewPanelOpen;
        _toggleOverviewButton.ButtonPressed = _currentViewModel.OverviewPanelOpen;

        if (_currentViewModel.OverviewPanelOpen)
        {
            RefreshBodiesList();
        }
    }

    private void RefreshBodiesList()
    {
        if (_currentViewModel == null) return;

        // Clear existing
        foreach (var child in _bodiesListContainer.GetChildren())
        {
            child.QueueFree();
        }

        // Add body entries
        foreach (var body in _currentViewModel.Bodies)
        {
            var label = new Label();
            label.Text = $"{body.Name} ({body.BodyType})";
            if (body.OrbitalParams != null)
            {
                label.Text += $" - {DisplayFormatter.FormatDistance(body.OrbitalParams.SemiMajorAxisAU)}";
            }

            if (body.IsSelected)
            {
                label.Modulate = new Color(0, 1, 1); // Cyan for selected
            }

            _bodiesListContainer.AddChild(label);
        }
    }

    public override void _Process(double delta)
    {
        if (_currentViewModel == null) return;

        // Handle time advancement (similar to StarMapPresenter)
        HandleTimeAdvancement(delta);

        // Update orbital positions (animate bodies)
        UpdateOrbitalPositions();

        // Update selection ring
        UpdateSelectionRing();
    }

    private void HandleTimeAdvancement(double delta)
    {
        if (_currentViewModel == null || _timeManager == null) return;

        // Update time using the shared TimeManager
        _timeManager.Update(delta, _currentViewModel.CurrentSpeed, _currentViewModel.IsPaused);
    }

    private void UpdateOrbitalPositions()
    {
        if (_currentViewModel == null) return;

        foreach (var body in _currentViewModel.Bodies)
        {
            if (body.OrbitalParams != null && _bodyNodes.TryGetValue(body.Id, out var bodyNode))
            {
                var position = OrbitalMath.CalculateOrbitalPosition(body.OrbitalParams, _currentViewModel.GameTime);
                bodyNode.Position = position * BASE_PIXELS_PER_AU;
            }
        }
    }

    private void UpdateSelectionRing()
    {
        if (_currentViewModel == null) return;

        var selectedBody = _currentViewModel.Bodies.Find(b => b.IsSelected);
        if (selectedBody != null && _bodyNodes.TryGetValue(selectedBody.Id, out var bodyNode))
        {
            _selectionRing.Visible = true;
            _selectionRing.Position = bodyNode.Position;
            // TODO: Set ring size based on body size
        }
        else
        {
            _selectionRing.Visible = false;
        }
    }

    public override void _UnhandledInput(InputEvent @event)
    {
        if (@event is InputEventMouseButton mouseButton)
        {
            // Convert to viewport local coordinates
            var localPos = _subViewport.GetMousePosition();
            var localButton = (InputEventMouseButton)mouseButton.Duplicate();
            localButton.Position = localPos;
            HandleMouseButton(localButton);
        }
        else if (@event is InputEventMouseMotion mouseMotion)
        {
            // Convert to viewport local coordinates
            var localPos = _subViewport.GetMousePosition();
            var localMotion = (InputEventMouseMotion)mouseMotion.Duplicate();
            localMotion.Position = localPos;
            HandleMouseMotion(localMotion);
        }
        else if (@event is InputEventKey keyEvent && keyEvent.Pressed && !keyEvent.Echo)
        {
            HandleKeyPress(keyEvent);
        }
    }

    private void HandleMouseButton(InputEventMouseButton mouseButton)
    {
        // First try to handle camera controls
        if (_cameraController?.HandleMouseButton(mouseButton) == true)
        {
            // Camera controller handled it, save state if needed
            SaveCameraState();
            return;
        }

        // Handle body selection with left mouse button
        if (mouseButton.ButtonIndex == MouseButton.Left && mouseButton.Pressed)
        {
            CheckBodyClick(mouseButton.Position);
        }
    }

    private void HandleMouseMotion(InputEventMouseMotion mouseMotion)
    {
        // First try to handle camera panning
        if (_cameraController?.HandleMouseMotion(mouseMotion) == true)
        {
            // Camera controller handled it, save state if needed
            SaveCameraState();
            return;
        }

        // Otherwise update hover and tooltip
        UpdateHover(mouseMotion.Position);
    }

    private void HandleKeyPress(InputEventKey keyEvent)
    {
        if (keyEvent.Keycode == Key.R)
        {
            OnResetCameraPressed();
        }
        else if (keyEvent.Keycode == Key.O)
        {
            OnToggleOverviewPressed();
        }
        else if (keyEvent.Keycode == Key.Space)
        {
            OnPausePressed();
        }
    }

    private void CheckBodyClick(Vector2 screenPosition)
    {
        // Convert screen position to world position
        var worldPosition = ScreenToWorld(screenPosition);

        // Check which body was clicked (closest within radius)
        Ulid? clickedBodyId = null;
        float closestDistance = float.MaxValue;

        foreach (var kvp in _bodyNodes)
        {
            var distance = worldPosition.DistanceTo(kvp.Value.Position);
            if (distance < 20.0f && distance < closestDistance) // 20 pixel click radius
            {
                clickedBodyId = kvp.Key;
                closestDistance = distance;
            }
        }

        if (clickedBodyId.HasValue)
        {
            _stateStore.ApplyCommand(new SelectCelestialBody(clickedBodyId.Value));
        }
    }

    private void UpdateHover(Vector2 screenPosition)
    {
        var worldPosition = ScreenToWorld(screenPosition);

        Ulid? hoveredBodyId = null;
        float closestDistance = float.MaxValue;

        foreach (var kvp in _bodyNodes)
        {
            var distance = worldPosition.DistanceTo(kvp.Value.Position);
            if (distance < 20.0f && distance < closestDistance)
            {
                hoveredBodyId = kvp.Key;
                closestDistance = distance;
            }
        }

        if (hoveredBodyId != _hoveredBodyId)
        {
            _hoveredBodyId = hoveredBodyId;
            UpdateTooltip(screenPosition);
        }
        else if (_hoveredBodyId.HasValue)
        {
            // Update tooltip position
            _tooltipPanel.Position = screenPosition + new Vector2(10, 10);
        }
    }

    private void UpdateTooltip(Vector2 screenPosition)
    {
        if (_hoveredBodyId.HasValue && _currentViewModel != null)
        {
            var body = _currentViewModel.Bodies.Find(b => b.Id == _hoveredBodyId.Value);
            if (body != null)
            {
                _tooltipPanel.Visible = true;
                _tooltipPanel.Position = screenPosition + new Vector2(10, 10);

                _tooltipBodyNameLabel.Text = body.Name;
                _tooltipBodyTypeLabel.Text = $"Type: {body.BodyType}";

                if (body.OrbitalParams != null)
                {
                    _tooltipDistanceLabel.Text = $"Distance: {DisplayFormatter.FormatDistance(body.OrbitalParams.SemiMajorAxisAU)}";
                    _tooltipPeriodLabel.Text = $"Period: {DisplayFormatter.FormatOrbitalPeriod(body.OrbitalParams.OrbitalPeriodDays)}";
                }
                else
                {
                    _tooltipDistanceLabel.Text = "Distance: N/A";
                    _tooltipPeriodLabel.Text = "Period: N/A";
                }

                _tooltipMassLabel.Text = $"Mass: {DisplayFormatter.FormatMass(body.MassEarthMasses)}";
            }
        }
        else
        {
            _tooltipPanel.Visible = false;
        }
    }

    private Vector2 ScreenToWorld(Vector2 screenPosition)
    {
        return _cameraController?.ScreenToWorld(screenPosition) ?? Vector2.Zero;
    }

    private void SaveCameraState()
    {
        if (_stateStore.State.SelectedSystemId.HasValue && _cameraController != null)
        {
            var cameraState = new CameraState(_cameraController.CurrentPosition, _cameraController.CurrentZoom);
            _stateStore.ApplyCommand(new UpdateCamera(_stateStore.State.SelectedSystemId.Value, cameraState));
        }
    }

    private void ZoomIn()
    {
        _cameraController?.ZoomIn();
        SaveCameraState();
    }

    private void ZoomOut()
    {
        _cameraController?.ZoomOut();
        SaveCameraState();
    }

    private void OnBackPressed()
    {
        GD.Print("Back button pressed - returning to galaxy map");

        // Pop the screen from navigation stack
        _stateStore.ApplyCommand(new PopScreen());

        // Navigate back to the star map (galaxy map)
        GetTree().ChangeSceneToFile("res://Scenes/UI/StarMapScreen.tscn");
    }

    private void OnResetCameraPressed()
    {
        if (_stateStore.State.SelectedSystemId.HasValue)
        {
            _stateStore.ApplyCommand(new ResetCamera(_stateStore.State.SelectedSystemId.Value));
        }
    }

    private void OnToggleOverviewPressed()
    {
        _stateStore.ApplyCommand(new ToggleSystemOverviewPanel());
    }

    private void OnPausePressed()
    {
        _stateStore.ApplyCommand(new TogglePause());
    }

    private void OnSlowerPressed()
    {
        if (_currentViewModel == null) return;

        var currentSpeed = (int)_currentViewModel.CurrentSpeed;
        if (currentSpeed > (int)GameSpeed.Paused)
        {
            var newSpeed = (GameSpeed)(currentSpeed - 1);
            _stateStore.ApplyCommand(new SetGameSpeed(newSpeed));
        }
    }

    private void OnFasterPressed()
    {
        if (_currentViewModel == null) return;

        var currentSpeed = (int)_currentViewModel.CurrentSpeed;
        if (currentSpeed < (int)GameSpeed.Fastest)
        {
            var newSpeed = (GameSpeed)(currentSpeed + 1);
            _stateStore.ApplyCommand(new SetGameSpeed(newSpeed));
        }
    }
}
