using Godot;
using Outpost3.Core;
using Outpost3.Core.Commands;
using Outpost3.Core.Domain;
using System;
using System.Collections.Generic;
using System.Linq;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the galaxy star map screen.
/// Renders stars as a 2D scatter plot with zoom/pan controls.
/// Limited to DEBUG_MAX_STARS (10) for debugging purposes.
/// </summary>
public partial class StarMapPresenter : Control
{


    // UI nodes
    private SubViewportContainer _viewportContainer = null!;
    private SubViewport _subViewport = null!;
    private Camera2D _camera = null!;
    private Node2D _starsContainer = null!;
    private Node2D _probesContainer = null!;
    private Label _titleLabel = null!;
    private Label _zoomLabel = null!;
    private Label _systemInfoLabel = null!;
    private Label _distanceLabel = null!;
    private Label _spectralClassLabel = null!;
    private Button _launchProbeButton = null!;
    private Button _viewSystemButton = null!;
    private Button _launchColonyButton = null!;
    private Button _backButton = null!;
    private Button _closeButton = null!;
    private Label _screenCoordLabel = null!;
    private Label _mapCoordLabel = null!;

    // Time control UI
    private Button _pauseButton = null!;
    private Button _slowDownButton = null!;
    private Button _speedUpButton = null!;
    private Label _timeSpeedLabel = null!;
    private Label _gameTimeLabel = null!;

    // Display options UI
    private CheckBox _showLabelsCheckbox = null!;

    // State
    private StateStore _stateStore = null!;  // Initialized in _Ready() from GameServices autoload
    private List<StarNode> _starNodes = new();
    private List<ProbeNode> _probeNodes = new();
    private StarNode? _selectedStar;
    private bool _hasAutoFitView;
    private int _lastRenderedStarCount;

    // Time control
    private double _timeScale = 1.0;
    private bool _isPaused = false;
    private double _autoAdvanceTimer = 0.0;

    // Display options
    private bool _showLabels = true;

    // System details modal
    private SystemDetailsModalPresenter _systemDetailsModal = null!;  // Initialized in _Ready() by loading scene

    // Camera control
    private const float MIN_ZOOM = 0.1f; // Allow zooming out to see all stars
    private const float MAX_ZOOM = 20.0f; // Allow zooming in close to individual stars
    private const float ZOOM_STEP = 0.1f; // Smoother zoom increments

    // Scale system: pixels per light-year at zoom level 1.0
    public const float BASE_PIXELS_PER_LY = 2.0f; // Base scale: 2 pixels = 1 LY at zoom 1.0
    private const float STAR_RENDER_RADIUS = 15.0f; // Render stars within this radius (screen pixels)

    // Debug settings
    private const int DEBUG_MAX_STARS = 50; // Increase for better testing

    // View constants
    private Vector2 _viewportSize;

    // Panning state
    private bool _isPanning = false;
    private Vector2 _panStartMousePos;
    private Vector2 _panStartCameraPos;

    public override void _Ready()
    {

        GD.Print("=== StarMapPresenter _Ready() START ===");

        // Get UI nodes
        _viewportContainer = GetNode<SubViewportContainer>("ViewportContainer");
        _subViewport = _viewportContainer.GetNode<SubViewport>("SubViewport");
        _camera = _subViewport.GetNode<Camera2D>("Camera2D");
        _starsContainer = _subViewport.GetNode<Node2D>("StarsContainer");
        GD.Print("StarMapPresenter: Got viewport, camera, and stars container");

        // Create probes container (or get if already exists)
        _probesContainer = _subViewport.GetNodeOrNull<Node2D>("ProbesContainer");
        if (_probesContainer == null)
        {
            _probesContainer = new Node2D();
            _probesContainer.Name = "ProbesContainer";
            _probesContainer.ZIndex = 10; // Render above stars
            _subViewport.AddChild(_probesContainer);
            GD.Print("StarMapPresenter: Created ProbesContainer");
        }
        else
        {
            GD.Print("StarMapPresenter: Using existing ProbesContainer");
        }

        _titleLabel = GetNode<Label>("TopBar/HBoxContainer/TitleLabel");
        _zoomLabel = GetNode<Label>("ZoomLabel");
        _systemInfoLabel = GetNode<Label>("SystemInfoLabel");
        _distanceLabel = GetNode<Label>("DistanceLabel");
        _spectralClassLabel = GetNode<Label>("SpectralClassLabel");
        _launchProbeButton = GetNode<Button>("LaunchProbeButton");
        _viewSystemButton = GetNode<Button>("ViewSystemButton");
        _launchColonyButton = GetNode<Button>("LaunchColonyButton");
        _backButton = GetNode<Button>("BackButton");
        _closeButton = GetNode<Button>("CloseButton");
        _screenCoordLabel = GetNode<Label>("ScreenCoordLabel");
        _mapCoordLabel = GetNode<Label>("MapCoordLabel");
        GD.Print("StarMapPresenter: Got all UI labels and buttons");

        // Get time control nodes (create simple inline UI if they don't exist)
        _pauseButton = GetNode<Button>("PauseButton");
        _slowDownButton = GetNode<Button>("SlowDownButton");
        _speedUpButton = GetNode<Button>("SpeedUpButton");
        _timeSpeedLabel = GetNode<Label>("TimeSpeedLabel");
        _gameTimeLabel = GetNode<Label>("GameTimeLabel");

        // If time controls don't exist in scene, create them inline
        // if (_pauseButton == null)
        // {
        //     CreateTimeControlsUI();
        // }

        // Initialize game time display
        UpdateGameTimeDisplay();

        // Get display options checkbox
        _showLabelsCheckbox = GetNode<CheckBox>("ShowLabelsCheckbox");
        _showLabelsCheckbox.Toggled += OnShowLabelsToggled;
        _showLabels = _showLabelsCheckbox.ButtonPressed;


        // Configure Camera2D for proper centering
        // CRITICAL: anchor_mode must be DRAG_CENTER (1) so camera.Position defines the CENTER of view
        // not the top-left corner
        _camera.AnchorMode = Camera2D.AnchorModeEnum.DragCenter;

        // Initialize camera at Sol (origin) with a reasonable starting zoom
        _camera.Position = Vector2.Zero; // Sol is at world origin (0,0), so this centers view on Sol
        _camera.Zoom = new Vector2(1.0f, 1.0f); // Start at 1x zoom (2 pixels per LY)
        GD.Print($"StarMapPresenter: Camera configured with AnchorMode.DragCenter");
        GD.Print($"StarMapPresenter: Camera initialized at Sol (0, 0), zoom {_camera.Zoom.X}x");

        // Connect signals
        _launchProbeButton.Pressed += OnLaunchProbePressed;
        _viewSystemButton.Pressed += OnViewSystemPressed;
        _launchColonyButton.Pressed += OnLaunchColonyPressed;
        _backButton.Pressed += OnBackPressed;
        _closeButton.Pressed += OnClosePressed;

        // Connect time control signals
        if (_pauseButton != null && _slowDownButton != null && _speedUpButton != null)
        {
            _pauseButton.Pressed += OnPausePressed;
            _slowDownButton.Pressed += OnSlowDownPressed;
            _speedUpButton.Pressed += OnSpeedUpPressed;
        }

        // Create system details modal
        GD.Print("StarMapPresenter: Loading SystemDetailsModal...");
        var modalScene = GD.Load<PackedScene>("res://Scenes/UI/SystemDetailsModal.tscn");
        if (modalScene == null)
        {
            throw new InvalidOperationException("StarMapPresenter: Could not load SystemDetailsModal.tscn");
        }

        var modalInstance = modalScene.Instantiate();
        AddChild(modalInstance);

        // The modal should be the root node with the script attached
        _systemDetailsModal = modalInstance as SystemDetailsModalPresenter
            ?? throw new InvalidOperationException($"StarMapPresenter: Modal instance is not SystemDetailsModalPresenter! Type: {modalInstance.GetType().Name}");

        GD.Print("StarMapPresenter: SystemDetailsModal loaded successfully");

        // Get state store from GameServices autoload (it's an autoload so it's always available)
        GD.Print("StarMapPresenter: Attempting to get GameServices...");
        var gameServices = GetNodeOrNull<GameServices>("/root/GameServices");
        if (gameServices == null)
        {
            GD.PrintErr("StarMapPresenter: GameServices autoload not found at /root/GameServices!");
            GD.PrintErr($"StarMapPresenter: Available root children:");
            foreach (var child in GetTree().Root.GetChildren())
            {
                GD.PrintErr($"  - {child.Name} ({child.GetType().Name})");
            }
            throw new InvalidOperationException("StarMapPresenter: GameServices autoload is required but not found at /root/GameServices");
        }

        GD.Print("StarMapPresenter: Found GameServices");
        _stateStore = gameServices.StateStore ?? throw new InvalidOperationException("StarMapPresenter: GameServices.StateStore is required but was null");

        GD.Print($"StarMapPresenter: StateStore found, current systems count: {_stateStore.State.Systems.Count}");
        _stateStore.StateChanged += OnStateChanged;
        GD.Print("StarMapPresenter: Calling RenderGalaxy()");
        RenderGalaxy();
        GD.Print("StarMapPresenter: RenderGalaxy() completed");

        // Add viewport resize handling
        GetTree().Root.SizeChanged += OnViewportResized;
        CallDeferred(MethodName.OnViewportResized);

        // Connect to ViewportContainer's GuiInput to handle star map interactions
        // This is necessary because SubViewport doesn't automatically forward input
        _viewportContainer.GuiInput += OnViewportContainerInput;

        GD.Print("=== StarMapPresenter _Ready() COMPLETE ===");
    }

    public override void _ExitTree()
    {
        _stateStore.StateChanged -= OnStateChanged;
        GetTree().Root.SizeChanged -= OnViewportResized;

        if (_viewportContainer != null)
        {
            _viewportContainer.GuiInput -= OnViewportContainerInput;
        }
    }

    private void OnViewportResized()
    {
        _subViewport.Size = (Vector2I)_viewportContainer.Size;
        _viewportSize = _subViewport.Size;
        GD.Print($"Viewport resized to {_subViewport.Size}");
    }

    public override void _Process(double delta)
    {
        UpdateMouseCoordinates();
        UpdateGameTimeDisplay();

        // Auto-advance time if not paused
        if (!_isPaused && _stateStore != null)
        {
            _autoAdvanceTimer += delta * _timeScale;
            if (_autoAdvanceTimer >= 1.0) // Advance every 1 second of real time
            {
                var hours = _autoAdvanceTimer;
                _autoAdvanceTimer = 0;
                var command = new AdvanceTime(hours);
                _stateStore.ApplyCommand(command);
            }
        }
    }

    private void UpdateMouseCoordinates()
    {
        // Get mouse position in viewport coordinates
        var mousePos = GetViewportMousePosition();

        // Check if mouse is within viewport bounds
        _viewportSize = _subViewport.Size;
        var viewportRect = new Rect2(Vector2.Zero, _viewportSize);
        if (!viewportRect.HasPoint(mousePos))
        {
            _screenCoordLabel.Text = "Screen: N/A";
            _mapCoordLabel.Text = "Map: N/A";
            return;
        }

        // Update screen coordinates (viewport pixels)
        _screenCoordLabel.Text = $"Screen: ({mousePos.X:F0}, {mousePos.Y:F0})";

        // Convert to world coordinates using proper Camera2D transformation
        var worldPos = ScreenToWorldPos(mousePos);

        // Convert to light-years relative to Sol (which is at world origin 0,0)
        // Since Sol is at (0,0) in world space, these coordinates are already Sol-relative
        var lightYears = WorldPosToLightYears(worldPos);

        // Update map coordinates (Sol-relative light-years)
        _mapCoordLabel.Text = $"Map: ({lightYears.X:F2} LY, {lightYears.Y:F2} LY)";
    }

    /// <summary>
    /// Converts screen coordinates (viewport pixels) to world coordinates using Camera2D transform.
    /// Uses Godot's proper coordinate transformation system.
    /// </summary>
    private Vector2 ScreenToWorldPos(Vector2 screenPos)
    {
        // Use Camera2D's get_screen_center_position() and zoom to transform
        var screenCenter = _subViewport.Size / 2;

        // Transform from screen space to world space
        // Account for camera position and zoom
        var offsetFromCenter = (screenPos - screenCenter) / _camera.Zoom.X;
        var worldPos = _camera.Position + offsetFromCenter;

        return worldPos;
    }

    /// <summary>
    /// Converts world coordinates to screen coordinates using Camera2D transform.
    /// </summary>
    private Vector2 WorldPosToScreen(Vector2 worldPos)
    {
        var screenCenter = _subViewport.Size / 2;

        // Transform from world space to screen space
        var offsetFromCamera = (worldPos - _camera.Position) * _camera.Zoom.X;
        var screenPos = screenCenter + offsetFromCamera;

        return screenPos;
    }

    /// <summary>
    /// Gets the mouse position in SubViewport pixel coordinates, accounting for SubViewportContainer stretch.
    /// </summary>
    private Vector2 GetViewportMousePosition()
    {
        var containerMousePos = _viewportContainer.GetLocalMousePosition();
        var containerSize = _viewportContainer.Size;
        var viewportSize = _subViewport.Size;

        if (containerSize.X <= 0 || containerSize.Y <= 0)
        {
            return Vector2.Zero;
        }

        var scaleX = viewportSize.X / containerSize.X;
        var scaleY = viewportSize.Y / containerSize.Y;
        var translated = new Vector2(containerMousePos.X * scaleX, containerMousePos.Y * scaleY);

        // Clamp to viewport bounds to avoid precision drift near edges
        translated.X = Mathf.Clamp(translated.X, 0, viewportSize.X);
        translated.Y = Mathf.Clamp(translated.Y, 0, viewportSize.Y);
        return translated;
    }

    /// <summary>
    /// Converts world position (pixels) to light-years using current scaling.
    /// </summary>
    private Vector2 WorldPosToLightYears(Vector2 worldPos)
    {
        // World coordinates are in pixels at BASE_PIXELS_PER_LY scale
        return worldPos / BASE_PIXELS_PER_LY;
    }

    /// <summary>
    /// Converts light-years to world position (pixels) using current scaling.
    /// </summary>
    private Vector2 LightYearsToWorldPos(Vector2 lightYears)
    {
        return lightYears * BASE_PIXELS_PER_LY;
    }

    /// <summary>
    /// Gets the current pixels-per-light-year scale based on zoom level.
    /// </summary>
    private float GetCurrentPixelsPerLY()
    {
        return BASE_PIXELS_PER_LY * _camera.Zoom.X;
    }

    public override void _UnhandledInput(InputEvent @event)
    {
        // NOTE: Using _UnhandledInput instead of _Input so that UI buttons can handle clicks first.
        // This prevents the star selection from intercepting button clicks.

        // Handle mouse wheel zoom
        if (@event is InputEventMouseButton mouseButton)
        {
            if (mouseButton.ButtonIndex == MouseButton.WheelUp)
            {
                ZoomTowardsCursor(ZOOM_STEP, mouseButton.Position);
                GetViewport()!.SetInputAsHandled();
            }
            else if (mouseButton.ButtonIndex == MouseButton.WheelDown)
            {
                ZoomTowardsCursor(-ZOOM_STEP, mouseButton.Position);
                GetViewport()!.SetInputAsHandled();
            }
            else if (mouseButton.ButtonIndex == MouseButton.Right)
            {
                if (mouseButton.Pressed)
                {
                    _isPanning = true;
                    _panStartMousePos = mouseButton.Position;
                    _panStartCameraPos = _camera.Position;
                    if (_stateStore != null && _stateStore.DebugSettings.DebugPanning)
                    {
                        GD.Print($"=== PANNING START ===");
                        GD.Print($"  Start mouse pos: {_panStartMousePos}");
                        GD.Print($"  Start camera pos: {_panStartCameraPos}");
                        GD.Print($"  Camera zoom: {_camera.Zoom.X}");
                    }
                }
                else
                {
                    if (_stateStore != null && _stateStore.DebugSettings.DebugPanning)
                    {
                        GD.Print($"=== PANNING END ===");
                        GD.Print($"  Final camera pos: {_camera.Position}");
                    }
                    _isPanning = false;
                }
                GetViewport()!.SetInputAsHandled();
            }
            else if (mouseButton.ButtonIndex == MouseButton.Left && mouseButton.Pressed)
            {
                HandleStarClick(mouseButton.Position);
                GetViewport()!.SetInputAsHandled();
            }
        }

        // Handle panning drag
        if (@event is InputEventMouseMotion mouseMotion && _isPanning)
        {
            var currentMousePos = mouseMotion.Position;
            var mouseDelta = currentMousePos - _panStartMousePos;

            // Convert screen delta to world delta (accounting for zoom)
            var worldDelta = mouseDelta / _camera.Zoom.X;
            var newCameraPos = _panStartCameraPos - worldDelta;

            _camera.Position = newCameraPos;

            // Debug every 10th motion event to avoid spam
            if (_stateStore != null && _stateStore.DebugSettings.DebugPanning && Engine.GetProcessFrames() % 10 == 0)
            {
                GD.Print($"PANNING: mouse delta {mouseDelta}, world delta {worldDelta}, camera now at {newCameraPos}");
            }

            GetViewport()!.SetInputAsHandled();
        }

        // Reset view with Space
        if (@event is InputEventKey keyEvent && keyEvent.Keycode == Key.Space && keyEvent.Pressed)
        {
            ResetView();
            GetViewport()!.SetInputAsHandled();
        }
    }

    /// <summary>
    /// Handles input events from the ViewportContainer to detect star map interactions.
    /// This is called via GuiInput signal, which fires AFTER UI buttons have processed input.
    /// </summary>
    private void OnViewportContainerInput(InputEvent @event)
    {
        if (_stateStore != null && _stateStore.DebugSettings.DebugSelection) GD.Print($"OnViewportContainerInput: {@event.GetType().Name}");

        // Handle mouse wheel zoom
        if (@event is InputEventMouseButton mouseButton)
        {
            if (mouseButton.ButtonIndex == MouseButton.WheelUp)
            {
                ZoomTowardsCursor(ZOOM_STEP, mouseButton.Position);
                _viewportContainer.AcceptEvent();
            }
            else if (mouseButton.ButtonIndex == MouseButton.WheelDown)
            {
                ZoomTowardsCursor(-ZOOM_STEP, mouseButton.Position);
                _viewportContainer.AcceptEvent();
            }
            else if (mouseButton.ButtonIndex == MouseButton.Right)
            {
                if (mouseButton.Pressed)
                {
                    _isPanning = true;
                    _panStartMousePos = mouseButton.Position;
                    _panStartCameraPos = _camera.Position;
                    if (_stateStore != null && _stateStore.DebugSettings.DebugPanning)
                    {
                        GD.Print($"=== PANNING START ===");
                        GD.Print($"  Start mouse pos: {_panStartMousePos}");
                        GD.Print($"  Start camera pos: {_panStartCameraPos}");
                        GD.Print($"  Camera zoom: {_camera.Zoom.X}");
                    }
                }
                else
                {
                    if (_stateStore != null && _stateStore.DebugSettings.DebugPanning)
                    {
                        GD.Print($"=== PANNING END ===");
                        GD.Print($"  Final camera pos: {_camera.Position}");
                    }
                    _isPanning = false;
                }
                _viewportContainer.AcceptEvent();
            }
            else if (mouseButton.ButtonIndex == MouseButton.Left && mouseButton.Pressed)
            {
                HandleStarClick(mouseButton.Position);
                _viewportContainer.AcceptEvent();
            }
        }

        // Handle panning drag
        if (@event is InputEventMouseMotion mouseMotion && _isPanning)
        {
            var currentMousePos = mouseMotion.Position;
            var mouseDelta = currentMousePos - _panStartMousePos;

            // Convert screen delta to world delta (accounting for zoom)
            var worldDelta = mouseDelta / _camera.Zoom.X;
            var newCameraPos = _panStartCameraPos - worldDelta;

            _camera.Position = newCameraPos;

            // Debug every 10th motion event to avoid spam
            if (_stateStore != null && _stateStore.DebugSettings.DebugPanning && Engine.GetProcessFrames() % 10 == 0)
            {
                GD.Print($"PANNING: mouse delta {mouseDelta}, world delta {worldDelta}, camera now at {newCameraPos}");
            }

            _viewportContainer.AcceptEvent();
        }

        // Reset view with Space
        if (@event is InputEventKey keyEvent && keyEvent.Keycode == Key.Space && keyEvent.Pressed)
        {
            ResetView();
            _viewportContainer.AcceptEvent();
        }
    }

    /// <summary>
    /// Zooms the camera towards the cursor position, keeping the point under the cursor stationary.
    /// </summary>
    private void ZoomTowardsCursor(float zoomDelta, Vector2 screenMousePos)
    {
        var mousePos = GetViewportMousePosition();

        var oldZoom = _camera.Zoom.X;
        var newZoom = Mathf.Clamp(oldZoom + zoomDelta, MIN_ZOOM, MAX_ZOOM);

        if (Mathf.Abs(newZoom - oldZoom) < 0.001f)
            return; // No zoom change

        // Get world position under cursor before zoom
        var worldPosBeforeZoom = ScreenToWorldPos(mousePos);

        // Apply new zoom
        _camera.Zoom = new Vector2(newZoom, newZoom);

        // Get world position under cursor after zoom
        var worldPosAfterZoom = ScreenToWorldPos(mousePos);

        // Adjust camera to keep world position under cursor the same
        var worldPosDrift = worldPosBeforeZoom - worldPosAfterZoom;
        _camera.Position += worldPosDrift;

        // Update all star nodes with new zoom for correct label/dot scaling
        UpdateStarZoomLevels(newZoom);

        // Update UI
        _zoomLabel.Text = $"Zoom: {newZoom:F1}x ({GetCurrentPixelsPerLY():F1} px/LY)";

        GD.Print($"Zoom: {oldZoom:F2} -> {newZoom:F2}, pixels/LY: {GetCurrentPixelsPerLY():F1}");
    }

    /// <summary>
    /// Handles left-click star selection using proper coordinate transformation.
    /// </summary>
    private void HandleStarClick(Vector2 screenMousePos)
    {
        var mousePos = GetViewportMousePosition();

        // Convert to world position
        var worldPos = ScreenToWorldPos(mousePos);
        var lightYearPos = WorldPosToLightYears(worldPos);

        GD.Print($"=== Star Click Debug ===");
        GD.Print($"  Container mouse: ({screenMousePos.X:F1}, {screenMousePos.Y:F1})");
        GD.Print($"  Viewport mouse: ({mousePos.X:F1}, {mousePos.Y:F1})");
        GD.Print($"  World pos: ({worldPos.X:F1}, {worldPos.Y:F1})");
        GD.Print($"  LY pos: ({lightYearPos.X:F2}, {lightYearPos.Y:F2})");
        GD.Print($"  Camera pos: ({_camera.Position.X:F1}, {_camera.Position.Y:F1})");
        GD.Print($"  Camera zoom: {_camera.Zoom.X:F2}");

        // Find nearest star within reasonable click distance
        const float clickRadiusLY = 2.0f;
        StarNode? clickedStar = null;
        float closestDistance = float.MaxValue;

        foreach (var starNode in _starNodes)
        {
            var starWorldPos = starNode.Position;
            var distance = worldPos.DistanceTo(starWorldPos);
            var distanceLY = distance / BASE_PIXELS_PER_LY;

            // Debug: print distance to Sol
            if (starNode.System.Name == "Sol")
            {
                GD.Print($"  Distance to Sol: {distanceLY:F2} LY (world: {distance:F1} px)");
            }

            if (distanceLY <= clickRadiusLY && distance < closestDistance)
            {
                closestDistance = distance;
                clickedStar = starNode;
            }
        }

        if (clickedStar != null)
        {
            GD.Print($"✓ Clicked: {clickedStar.System.Name} (dist: {closestDistance / BASE_PIXELS_PER_LY:F2} LY)");
            OnStarClicked(clickedStar);
        }
        else
        {
            GD.Print($"✗ No star within {clickRadiusLY} LY radius");
        }
    }

    /// <summary>
    /// Updates all star nodes with the current zoom level for label rendering.
    /// </summary>
    private void UpdateStarZoomLevels(float zoomLevel)
    {
        foreach (var starNode in _starNodes)
        {
            starNode.UpdateZoomLevel(zoomLevel);
        }
    }

    private void ResetView()
    {
        GD.Print("ResetView: Centering on Sol with all stars visible");

        if (_starNodes.Count == 0)
        {
            // No stars - just center on origin (Sol position)
            _camera.Position = Vector2.Zero;
            _camera.Zoom = new Vector2(1.0f, 1.0f);
            _zoomLabel.Text = "Zoom: 1.0x (2.0 px/LY)";
            UpdateStarZoomLevels(_camera.Zoom.X);
            return;
        }

        // Camera should always center on Sol (which is at origin 0,0 in our coordinate system)
        _camera.Position = Vector2.Zero;

        // Calculate bounding box of all stars to determine zoom level
        var minPos = new Vector2(float.MaxValue, float.MaxValue);
        var maxPos = new Vector2(float.MinValue, float.MinValue);

        foreach (var starNode in _starNodes)
        {
            var pos = starNode.Position; // Already in world coordinates
            minPos.X = Mathf.Min(minPos.X, pos.X);
            minPos.Y = Mathf.Min(minPos.Y, pos.Y);
            maxPos.X = Mathf.Max(maxPos.X, pos.X);
            maxPos.Y = Mathf.Max(maxPos.Y, pos.Y);
        }

        var size = maxPos - minPos;

        // Calculate zoom to fit all stars with padding (20% on each side)
        var viewportSize = _subViewport.Size;

        // Add 40% padding to ensure stars aren't at screen edges
        var zoomX = viewportSize.X / (size.X * 1.4f);
        var zoomY = viewportSize.Y / (size.Y * 1.4f);
        var zoom = Mathf.Min(zoomX, zoomY);
        zoom = Mathf.Clamp(zoom, MIN_ZOOM, MAX_ZOOM);

        _camera.Zoom = new Vector2(zoom, zoom);
        _zoomLabel.Text = $"Zoom: {zoom:F1}x ({GetCurrentPixelsPerLY():F1} px/LY)";
        UpdateStarZoomLevels(_camera.Zoom.X);

        GD.Print($"ResetView: Camera centered on Sol at (0, 0), zoom {zoom:F2}");
        GD.Print($"ResetView: Star field bounds: ({minPos.X:F1}, {minPos.Y:F1}) to ({maxPos.X:F1}, {maxPos.Y:F1})");
        GD.Print($"ResetView: Field size: {size.X:F1} x {size.Y:F1} world pixels ({size.X / BASE_PIXELS_PER_LY:F1} x {size.Y / BASE_PIXELS_PER_LY:F1} LY)");
    }

    private void OnStateChanged()
    {
        RenderGalaxy();
        RenderProbes();
        UpdateProbePositions();
    }

    /// <summary>
    /// Renders all stars in the galaxy as StarNode instances.
    /// </summary>
    private void RenderGalaxy()
    {
        if (_stateStore.DebugSettings.DebugRendering) GD.Print("RenderGalaxy: Called");

        var state = _stateStore.State;

        if (state == null)
        {
            GD.PrintErr("RenderGalaxy: State is null!");
            return;
        }

        var allSystems = state.Systems;

        if (allSystems == null || allSystems.Count == 0)
        {
            GD.PrintErr($"RenderGalaxy: No systems found! Systems list is {(allSystems == null ? "null" : "empty")}");
            return;
        }

        // For debugging, limit to DEBUG_MAX_STARS systems
        // Prioritize Sol first, then take others
        var systems = new List<StarSystem>();
        var sol = allSystems.Find(s => s.Name == "Sol" || Mathf.Abs(s.DistanceFromSol) < 0.01f);
        if (sol != null)
        {
            systems.Add(sol);
        }

        // Add other systems up to the limit
        var otherSystems = allSystems.Where(s => s != sol).Take(DEBUG_MAX_STARS - (sol != null ? 1 : 0));
        systems.AddRange(otherSystems);

        if (_stateStore != null && _stateStore.DebugSettings.DebugRendering)
        {
            GD.Print($"RenderGalaxy: Limiting to {DEBUG_MAX_STARS} stars for debugging (found {allSystems.Count} total systems)");
            GD.Print($"RenderGalaxy: Selected {systems.Count} systems to render:");
        }

        // Clear existing star nodes
        foreach (var node in _starNodes)
        {
            node.QueueFree();
        }
        _starNodes.Clear();

        // Add an origin marker to visualize Sol's position (world 0,0)
        var originMarker = new OriginMarker();
        _starsContainer.AddChild(originMarker);

        // Create new star nodes with debug output
        int index = 0;
        foreach (var system in systems)
        {
            var starNode = CreateStarNode(system);
            starNode.SetShowLabels(_showLabels); // Apply current label visibility setting
            _starsContainer.AddChild(starNode);
            _starNodes.Add(starNode);

            if (_stateStore != null && _stateStore.DebugSettings.DebugRendering)
            {
                // Debug output for each generated star
                var worldPos = starNode.Position;
                var mapPos = WorldPosToLightYears(worldPos);
                GD.Print($"  {index + 1:D2}. {system.Name}:");
                GD.Print($"      Raw pos: ({system.Position.X:F2}, {system.Position.Y:F2}, {system.Position.Z:F2})");
                GD.Print($"      World pos: ({worldPos.X:F1}, {worldPos.Y:F1}) px");
                GD.Print($"      Map pos: ({mapPos.X:F1}, {mapPos.Y:F1}) LY");
                GD.Print($"      Distance from Sol: {system.DistanceFromSol:F2} LY");
                GD.Print($"      Spectral class: {system.SpectralClass}");
            }

            index++;
        }

        _titleLabel.Text = $"Galaxy Map - {systems.Count} Stars (Debug Mode)";

        // Sync drawables with the current zoom while we await any deferred camera adjustments.
        UpdateStarZoomLevels(_camera.Zoom.X);

        ApplySelectionFromState(state);

        var shouldAutoFit = !_hasAutoFitView || systems.Count != _lastRenderedStarCount;
        _lastRenderedStarCount = systems.Count;

        if (shouldAutoFit)
        {
            if (_stateStore != null && _stateStore.DebugSettings.DebugRendering) GD.Print("RenderGalaxy: Queueing ResetView() call...");
            CallDeferred(MethodName.ResetView);
            _hasAutoFitView = true;
        }

        if (_stateStore != null && _stateStore.DebugSettings.DebugRendering) GD.Print($"RenderGalaxy: Completed. Stars in container: {_starsContainer.GetChildCount()}, Stars in list: {_starNodes.Count}");
    }

    /// <summary>
    /// Creates a StarNode for a given star system.
    /// Positions stars RELATIVE to Sol (Sol is at origin 0,0, all others are relative offsets).
    /// </summary>
    private StarNode CreateStarNode(StarSystem system)
    {
        var starNode = new StarNode();
        starNode.Initialize(system);

        // Sol should ALWAYS be at Vector3.Zero from galaxy generation
        // Therefore, Sol's world position in 2D should always be (0, 0)
        // All other stars are positioned relative to Sol

        // Convert 3D position to 2D (just take X,Y, ignore Z)
        var systemPos = new Vector2(system.Position.X, system.Position.Y);

        // Scale by BASE_PIXELS_PER_LY to convert light-years to pixels
        var worldPos = systemPos * BASE_PIXELS_PER_LY;

        starNode.Position = worldPos;

        // Debug output for coordinate verification
        if (_stateStore != null && _stateStore.DebugSettings.DebugStarCreation && (system.Name == "Sol" || system.Name.Contains("Gliese") || system.Name.Contains("2MASS")))
        {
            GD.Print($"CreateStarNode: {system.Name}:");
            GD.Print($"  3D Position (LY): ({system.Position.X:F4}, {system.Position.Y:F4}, {system.Position.Z:F4})");
            GD.Print($"  2D Position (LY): ({systemPos.X:F4}, {systemPos.Y:F4})");
            GD.Print($"  World pos (px): ({worldPos.X:F1}, {worldPos.Y:F1})");
            GD.Print($"  Distance from Sol: {system.DistanceFromSol:F2} LY");

            if (system.Name == "Sol")
            {
                if (Mathf.Abs(worldPos.X) > 0.01f || Mathf.Abs(worldPos.Y) > 0.01f)
                {
                    GD.PrintErr($"ERROR: Sol is not at world origin! Position: ({worldPos.X}, {worldPos.Y})");
                }
                else
                {
                    GD.Print($"✓ Sol correctly positioned at world origin (0, 0)");
                }
            }
        }

        return starNode;
    }

    /// <summary>
    /// Called when a star is clicked.
    /// </summary>
    public void OnStarClicked(StarNode starNode)
    {
        // Deselect previous star
        if (_selectedStar != null)
        {
            _selectedStar.SetSelected(false);
        }

        // Select new star
        _selectedStar = starNode;
        _selectedStar.SetSelected(true);

        // Update UI
        UpdateSelectionInfo();

        // Send SelectSystemCommand
        var command = new SelectSystemCommand(starNode.SystemId);
        _stateStore.ApplyCommand(command);
    }

    private void UpdateSelectionInfo()
    {
        if (_selectedStar == null)
        {
            _systemInfoLabel.Text = "No system selected";
            _distanceLabel.Text = "Distance: N/A";
            _spectralClassLabel.Text = "Spectral Class: N/A";
            _launchProbeButton.Disabled = true;
            _viewSystemButton.Disabled = true;
            _launchColonyButton.Disabled = true;
            return;
        }

        var system = _selectedStar.System;
        _systemInfoLabel.Text = $"Selected: {system.Name}";
        _distanceLabel.Text = $"Distance: {system.DistanceFromSol:F2} LY";
        _spectralClassLabel.Text = $"Spectral Class: {system.SpectralClass} | Luminosity: {system.Luminosity:F2} L☉";

        // Enable/disable buttons based on discovery level
        _launchProbeButton.Disabled = system.DiscoveryLevel == DiscoveryLevel.Explored;
        _viewSystemButton.Disabled = system.DiscoveryLevel == DiscoveryLevel.Unknown;
        _launchColonyButton.Disabled = system.DiscoveryLevel != DiscoveryLevel.Explored;
    }

    private void OnLaunchProbePressed()
    {
        if (_stateStore.DebugSettings.DebugActions) GD.Print($"=== LAUNCH PROBE BUTTON PRESSED ===");

        if (_selectedStar == null)
        {
            if (_stateStore.DebugSettings.DebugActions) GD.Print("  ✗ No star selected!");
            return;
        }


        if (_stateStore.DebugSettings.DebugActions)
        {
            GD.Print($"  ✓ Selected star: {_selectedStar.System.Name}");
            GD.Print($"  ✓ System ID: {_selectedStar.SystemId}");
            GD.Print($"  Creating LaunchProbe command...");
        }

        var command = new LaunchProbe(_selectedStar.SystemId);

        if (_stateStore.DebugSettings.DebugActions) GD.Print($"  Applying command to StateStore...");
        _stateStore.ApplyCommand(command);

        if (_stateStore.DebugSettings.DebugActions)
        {
            GD.Print($"  ✓ Probe launched to {_selectedStar.System.Name}");
            GD.Print($"  Current probes in flight: {_stateStore.State.ProbesInFlight.Count}");
        }

        // Render probes to show the newly launched probe
        if (_stateStore.DebugSettings.DebugRendering) GD.Print($"  Calling RenderProbes()...");
        RenderProbes();
        if (_stateStore.DebugSettings.DebugRendering) GD.Print($"  ✓ Probes rendered");
    }

    private void OnViewSystemPressed()
    {
        if (_stateStore.DebugSettings.DebugActions) GD.Print($"=== VIEW SYSTEM DETAILS BUTTON PRESSED ===");

        if (_selectedStar == null)
        {
            if (_stateStore.DebugSettings.DebugActions) GD.Print("  ✗ No star selected!");
            return;
        }

        if (_stateStore.DebugSettings.DebugActions) GD.Print($"  ✓ Selected star: {_selectedStar.System.Name}");
        if (_stateStore.DebugSettings.DebugActions) GD.Print($"  ✓ Calling ShowSystem() on modal...");

        _systemDetailsModal.ShowSystem(_selectedStar.System);

        if (_stateStore.DebugSettings.DebugActions) GD.Print($"  ✓ Modal should be visible now");
    }

    private void OnLaunchColonyPressed()
    {
        if (_selectedStar == null) return;

        GD.Print($"Launching colony mission to {_selectedStar.System.Name}");
        GetTree().ChangeSceneToFile("res://Scenes/ShipJourneyLog.tscn");
    }

    private void OnBackPressed()
    {
        GD.Print("Back button pressed - returning to New Game Config");
        GetTree().ChangeSceneToFile("res://Scenes/NewGameConfigScreen.tscn");
    }

    private void OnClosePressed()
    {
        GD.Print("Star map closed");

        // Check if we're in standalone mode (scene was loaded directly)
        // or embedded mode (part of Main scene)
        var mainNode = GetNodeOrNull("/root/Main");
        if (mainNode == null)
        {
            // Standalone mode - return to New Game Config
            GetTree().ChangeSceneToFile("res://Scenes/NewGameConfigScreen.tscn");
        }
        else
        {
            // Embedded mode - just hide
            Hide();
        }
    }

    private void ApplySelectionFromState(GameState state)
    {
        foreach (var starNode in _starNodes)
        {
            starNode.SetSelected(false);
        }

        _selectedStar = null;

        if (state.SelectedSystemId is { } selectedId)
        {
            var matchingStar = _starNodes.FirstOrDefault(node => node.SystemId == selectedId);
            if (matchingStar != null)
            {
                _selectedStar = matchingStar;
                matchingStar.SetSelected(true);
            }
        }

        UpdateSelectionInfo();
    }

    // private void CreateTimeControlsUI()
    // {
    //     // Create a simple inline panel for time controls
    //     var timeControlsPanel = new PanelContainer();
    //     timeControlsPanel.Name = "TimeControls";
    //     AddChild(timeControlsPanel);

    //     // Position in top-right corner - wider to accommodate game time
    //     timeControlsPanel.SetAnchorsPreset(Control.LayoutPreset.TopRight);
    //     timeControlsPanel.OffsetLeft = -380;
    //     timeControlsPanel.OffsetTop = 80;
    //     timeControlsPanel.OffsetRight = -10;
    //     timeControlsPanel.OffsetBottom = 140;

    //     var margin = new MarginContainer();
    //     margin.AddThemeConstantOverride("margin_left", 10);
    //     margin.AddThemeConstantOverride("margin_top", 10);
    //     margin.AddThemeConstantOverride("margin_right", 10);
    //     margin.AddThemeConstantOverride("margin_bottom", 10);
    //     timeControlsPanel.AddChild(margin);

    //     var hbox = new HBoxContainer();
    //     hbox.AddThemeConstantOverride("separation", 15);
    //     margin.AddChild(hbox);

    //     // Left side: time controls
    //     var leftVBox = new VBoxContainer();
    //     leftVBox.AddThemeConstantOverride("separation", 5);
    //     hbox.AddChild(leftVBox);

    //     _timeSpeedLabel = new Label();
    //     _timeSpeedLabel.Text = "Time: 1.0x (Running)";
    //     _timeSpeedLabel.HorizontalAlignment = HorizontalAlignment.Center;
    //     leftVBox.AddChild(_timeSpeedLabel);

    //     var buttonBox = new HBoxContainer();
    //     buttonBox.AddThemeConstantOverride("separation", 5);
    //     leftVBox.AddChild(buttonBox);

    //     _pauseButton = new Button();
    //     _pauseButton.Text = "⏸";
    //     _pauseButton.TooltipText = "Pause/Resume";
    //     _pauseButton.CustomMinimumSize = new Vector2(50, 0);
    //     buttonBox.AddChild(_pauseButton);

    //     _slowDownButton = new Button();
    //     _slowDownButton.Text = "◀";
    //     _slowDownButton.TooltipText = "Slow Down";
    //     _slowDownButton.CustomMinimumSize = new Vector2(50, 0);
    //     buttonBox.AddChild(_slowDownButton);

    //     _speedUpButton = new Button();
    //     _speedUpButton.Text = "▶";
    //     _speedUpButton.TooltipText = "Speed Up";
    //     _speedUpButton.CustomMinimumSize = new Vector2(50, 0);
    //     buttonBox.AddChild(_speedUpButton);

    //     // Separator
    //     var separator = new VSeparator();
    //     hbox.AddChild(separator);

    //     // Right side: game time display
    //     var rightVBox = new VBoxContainer();
    //     rightVBox.AddThemeConstantOverride("separation", 3);
    //     rightVBox.SizeFlagsVertical = Control.SizeFlags.ExpandFill;
    //     hbox.AddChild(rightVBox);

    //     var gameTimeHeaderLabel = new Label();
    //     gameTimeHeaderLabel.Text = "Game Time";
    //     gameTimeHeaderLabel.HorizontalAlignment = HorizontalAlignment.Center;
    //     gameTimeHeaderLabel.AddThemeColorOverride("font_color", new Color(0.7f, 0.7f, 0.7f));
    //     rightVBox.AddChild(gameTimeHeaderLabel);

    //     _gameTimeLabel = new Label();
    //     _gameTimeLabel.Name = "GameTimeLabel";
    //     _gameTimeLabel.Text = "Year 0, Day 0";
    //     _gameTimeLabel.HorizontalAlignment = HorizontalAlignment.Center;
    //     _gameTimeLabel.SizeFlagsVertical = Control.SizeFlags.ExpandFill;
    //     rightVBox.AddChild(_gameTimeLabel);
    // }

    private void OnPausePressed()
    {
        if (_stateStore.DebugSettings.DebugTimeControls) GD.Print($"=== PAUSE BUTTON PRESSED ===");

        _isPaused = !_isPaused;

        if (_stateStore.DebugSettings.DebugTimeControls) GD.Print($"  Time is now: {(_isPaused ? "PAUSED" : "RUNNING")}");
        if (_stateStore.DebugSettings.DebugTimeControls) GD.Print($"  Current time scale: {_timeScale}x");

        UpdateTimeControlUI();
    }

    private void OnSlowDownPressed()
    {
        if (_stateStore.DebugSettings.DebugTimeControls) GD.Print($"=== SLOW DOWN BUTTON PRESSED ===");

        if (_isPaused)
        {
            if (_stateStore.DebugSettings.DebugTimeControls) GD.Print($"  ✗ Time is paused, ignoring");
            return;
        }

        var oldScale = _timeScale;
        _timeScale = Mathf.Max(0.25, _timeScale * 0.5);

        if (_stateStore.DebugSettings.DebugTimeControls) GD.Print($"  Time scale: {oldScale}x -> {_timeScale}x");

        UpdateTimeControlUI();
    }

    private void OnSpeedUpPressed()
    {
        if (_stateStore.DebugSettings.DebugTimeControls) GD.Print($"=== SPEED UP BUTTON PRESSED ===");

        if (_isPaused)
        {
            if (_stateStore.DebugSettings.DebugTimeControls) GD.Print($"  ✗ Time is paused, ignoring");
            return;
        }

        var oldScale = _timeScale;
        _timeScale = Mathf.Min(32.0, _timeScale * 2.0);

        if (_stateStore.DebugSettings.DebugTimeControls) GD.Print($"  Time scale: {oldScale}x -> {_timeScale}x");

        UpdateTimeControlUI();
    }

    private void UpdateTimeControlUI()
    {
        if (_timeSpeedLabel == null)
        {
            if (_stateStore.DebugSettings.DebugTimeControls) GD.Print($"  ✗ _timeSpeedLabel is null, cannot update UI");
            return;
        }

        if (_isPaused)
        {
            _timeSpeedLabel.Text = "Time: PAUSED";
        }
        else
        {
            _timeSpeedLabel.Text = $"Time: {_timeScale:F2}x (Running)";
        }

        if (_stateStore.DebugSettings.DebugTimeControls) GD.Print($"  ✓ Time label updated: {_timeSpeedLabel.Text}");
    }

    private void UpdateGameTimeDisplay()
    {
        if (_gameTimeLabel == null) return;

        var gameTime = _stateStore.State.GameTime;

        // Convert hours to years and days
        const double hoursPerDay = 24.0;
        const double daysPerYear = 365.25;
        const double hoursPerYear = hoursPerDay * daysPerYear;

        var years = (int)(gameTime / hoursPerYear);
        var remainingHours = gameTime % hoursPerYear;
        var days = (int)(remainingHours / hoursPerDay);
        var hours = (int)(remainingHours % hoursPerDay);

        _gameTimeLabel.Text = $"Year {years}, Day {days}\n{hours:D2}:00";
    }

    // private void CreateDisplayOptionsUI()
    // {
    //     // Create a panel for display options
    //     var displayOptionsPanel = new PanelContainer();
    //     displayOptionsPanel.Name = "DisplayOptions";
    //     AddChild(displayOptionsPanel);

    //     // Position in top-right corner, below time controls
    //     displayOptionsPanel.SetAnchorsPreset(Control.LayoutPreset.TopRight);
    //     displayOptionsPanel.OffsetLeft = -380;
    //     displayOptionsPanel.OffsetTop = 150;
    //     displayOptionsPanel.OffsetRight = -10;
    //     displayOptionsPanel.OffsetBottom = 190;

    //     var margin = new MarginContainer();
    //     margin.AddThemeConstantOverride("margin_left", 10);
    //     margin.AddThemeConstantOverride("margin_top", 5);
    //     margin.AddThemeConstantOverride("margin_right", 10);
    //     margin.AddThemeConstantOverride("margin_bottom", 5);
    //     displayOptionsPanel.AddChild(margin);

    //     var vbox = new VBoxContainer();
    //     vbox.AddThemeConstantOverride("separation", 5);
    //     margin.AddChild(vbox);

    //     var label = new Label();
    //     label.Text = "Display Options";
    //     label.HorizontalAlignment = HorizontalAlignment.Center;
    //     vbox.AddChild(label);

    //     _showLabelsCheckbox = new CheckBox();
    //     _showLabelsCheckbox.Name = "ShowLabelsCheckbox";
    //     _showLabelsCheckbox.Text = "Show Star Labels";
    //     _showLabelsCheckbox.ButtonPressed = _showLabels;
    //     _showLabelsCheckbox.Toggled += OnShowLabelsToggled;
    //     vbox.AddChild(_showLabelsCheckbox);
    // }

    private void OnShowLabelsToggled(bool pressed)
    {
        _showLabels = pressed;

        // Update all star nodes
        foreach (var starNode in _starNodes)
        {
            starNode.SetShowLabels(_showLabels);
        }

        GD.Print($"Star labels: {(_showLabels ? "ON" : "OFF")}");
    }

    private void RenderProbes()
    {
        if (_stateStore.DebugSettings.DebugRendering) GD.Print($"=== RENDER PROBES ===");

        var state = _stateStore.State;

        if (_stateStore.DebugSettings.DebugRendering) GD.Print($"  Probes in flight: {state.ProbesInFlight.Count}");

        // Clear existing probe nodes
        foreach (var probeNode in _probeNodes)
        {
            probeNode.QueueFree();
        }
        _probeNodes.Clear();

        // Create probe nodes for all probes in flight
        int count = 0;
        foreach (var probe in state.ProbesInFlight)
        {
            // Find target system
            var targetSystem = state.Systems.FirstOrDefault(s => s.Id == probe.TargetSystemId);
            if (targetSystem == null)
            {
                if (_stateStore.DebugSettings.DebugRendering) GD.Print($"  ✗ Probe {probe.Id}: Target system {probe.TargetSystemId} not found!");
                continue;
            }

            var probeNode = new ProbeNode();
            probeNode.Initialize(probe, targetSystem, state.GameTime);
            probeNode.ZIndex = 10; // Render above stars
            _probesContainer.AddChild(probeNode);
            _probeNodes.Add(probeNode);
            count++;

            if (_stateStore.DebugSettings.DebugRendering)
            {
                GD.Print($"  ✓ Probe {count}: {probe.Id}");
                GD.Print($"    Target: {targetSystem.Name}");
                GD.Print($"    Launched at: {probe.LaunchedAt:F2}");
                GD.Print($"    Arrival time: {probe.ArrivalTime:F2}");
                GD.Print($"    Current game time: {state.GameTime:F2}");
            }
        }

        if (_stateStore.DebugSettings.DebugRendering) GD.Print($"  ✓ Rendered {count} probe(s)");
    }

    private void UpdateProbePositions()
    {
        var state = _stateStore.State;

        foreach (var probeNode in _probeNodes)
        {
            probeNode.UpdatePosition(state.GameTime);
        }
    }
}

/// <summary>
/// Visual representation of a single star in the galaxy map.
/// Simplified Node2D-based approach without collision detection for better performance.
/// </summary>
public partial class StarNode : Node2D
{
    public StarSystem System { get; private set; } = null!;
    public Ulid SystemId => System.Id;

    private bool _isSelected = false;
    private const float BASE_STAR_SIZE = 6f;
    private const float SELECTION_RING_SIZE = 12f;
    private float _currentZoom = 1.0f;
    private bool _showLabels = true;

    public void Initialize(StarSystem system)
    {
        System = system;
    }

    /// <summary>
    /// Updates the zoom level and triggers a redraw to show/hide labels appropriately.
    /// </summary>
    public void UpdateZoomLevel(float zoomLevel)
    {
        if (Mathf.Abs(_currentZoom - zoomLevel) > 0.01f)
        {
            _currentZoom = zoomLevel;
            QueueRedraw();
        }
    }

    /// <summary>
    /// Sets whether labels should be displayed.
    /// </summary>
    public void SetShowLabels(bool showLabels)
    {
        if (_showLabels != showLabels)
        {
            _showLabels = showLabels;
            QueueRedraw();
        }
    }

    public override void _Draw()
    {
        if (System == null) return;

        // Calculate inverse zoom to keep visual elements constant screen size
        float inverseZoom = 1.0f / Mathf.Max(_currentZoom, 0.1f);

        // Special rendering for Sol (home system) - add a distinctive ring
        bool isSol = System.Name == "Sol" || Mathf.Abs(System.DistanceFromSol) < 0.01f;
        if (isSol)
        {
            // Draw a golden ring around Sol to make it distinctive
            DrawArc(Vector2.Zero, (SELECTION_RING_SIZE + 4f) * inverseZoom, 0, Mathf.Tau, 32, new Color(1.0f, 0.84f, 0.0f, 0.9f), 2.0f * inverseZoom);
        }

        // Draw selection ring if selected (constant screen size)
        if (_isSelected)
        {
            DrawCircle(Vector2.Zero, SELECTION_RING_SIZE * inverseZoom, new Color(0.2f, 0.8f, 1.0f, 0.8f));
        }

        // Calculate star size based on luminosity
        var baseLuminositySize = Mathf.Log(System.Luminosity + 1.0f) * 2.0f;
        baseLuminositySize = Mathf.Clamp(baseLuminositySize, 3f, 15f);
        var screenStarSize = BASE_STAR_SIZE + baseLuminositySize;
        var worldStarSize = screenStarSize * inverseZoom;

        var color = GetStarColor();
        DrawCircle(Vector2.Zero, worldStarSize, color);

        // Show labels if enabled, or always show for selected stars or Sol
        bool shouldShowLabel = _showLabels || _isSelected || isSol;

        if (shouldShowLabel)
        {
            // For crisp text rendering, we need to:
            // 1. Use a fixed screen-space font size
            // 2. Counter-scale the canvas transform to render at screen resolution

            var font = ThemeDB.FallbackFont;
            var fontSize = 14; // Fixed screen-space font size for crisp rendering

            var labelOffset = GetSmartLabelPosition(screenStarSize);
            var worldLabelOffset = labelOffset * inverseZoom;

            // Debug mode: show coordinates
            var ly = Position / StarMapPresenter.BASE_PIXELS_PER_LY;
            var labelText = $"{System.Name} ({ly.X:F1}, {ly.Y:F1})";

            // Save current transform
            var originalTransform = GetCanvasTransform();

            // Counter-scale to render text at screen resolution
            // This prevents blurry text when zoomed
            DrawSetTransform(worldLabelOffset, 0.0f, new Vector2(inverseZoom, inverseZoom));

            var fontColor = isSol ? new Color(1.0f, 0.84f, 0.0f, 1.0f) : new Color(1, 1, 1, 0.95f);
            DrawString(font, Vector2.Zero, labelText, HorizontalAlignment.Left, -1, fontSize, fontColor);

            // Restore transform
            DrawSetTransform(Vector2.Zero, 0.0f, Vector2.One);
        }
    }

    /// <summary>
    /// Calculates smart label position based on system ID to reduce overlap.
    /// </summary>
    private Vector2 GetSmartLabelPosition(float starSize)
    {
        int hash = System.Id.GetHashCode();
        int position = Math.Abs(hash) % 8;
        float offset = starSize + 8f;

        return position switch
        {
            0 => new Vector2(offset, 0),              // East
            1 => new Vector2(offset, offset),         // Southeast
            2 => new Vector2(0, offset),              // South
            3 => new Vector2(-offset, offset),        // Southwest
            4 => new Vector2(-offset, 0),             // West
            5 => new Vector2(-offset, -offset),       // Northwest
            6 => new Vector2(0, -offset),             // North
            7 => new Vector2(offset, -offset),        // Northeast
            _ => new Vector2(offset, offset)          // Default
        };
    }

    private Color GetStarColor()
    {
        // Grey out undiscovered stars
        if (System.DiscoveryLevel == DiscoveryLevel.Unknown || System.DiscoveryLevel == DiscoveryLevel.Detected)
        {
            return new Color(0.5f, 0.5f, 0.5f, 0.8f);
        }

        // Color based on spectral class
        var spectralClass = System.SpectralClass.Length > 0 ? System.SpectralClass[0] : 'G';
        return spectralClass switch
        {
            'O' => new Color(0.6f, 0.7f, 1.0f, 1.0f),      // Blue
            'B' => new Color(0.7f, 0.8f, 1.0f, 1.0f),      // Blue-white
            'A' => new Color(0.9f, 0.9f, 1.0f, 1.0f),      // White
            'F' => new Color(1.0f, 1.0f, 0.9f, 1.0f),      // Yellow-white
            'G' => new Color(1.0f, 1.0f, 0.6f, 1.0f),      // Yellow (like our Sun)
            'K' => new Color(1.0f, 0.7f, 0.4f, 1.0f),      // Orange
            'M' => new Color(1.0f, 0.5f, 0.3f, 1.0f),      // Red
            _ => new Color(1.0f, 1.0f, 1.0f, 1.0f)         // Default white
        };
    }

    public void SetSelected(bool selected)
    {
        _isSelected = selected;
        QueueRedraw();
    }
}

/// <summary>
/// Visual marker for the origin (Sol's position at 0,0).
/// Helps visualize the coordinate system center.
/// </summary>
public partial class OriginMarker : Node2D
{
    public override void _Process(double delta)
    {
        // Always redraw to stay visible at all zoom levels
        QueueRedraw();
    }

    public override void _Draw()
    {
        // Get the camera to determine proper scaling
        var camera = GetViewport()?.GetCamera2D();
        if (camera == null) return;

        float inverseZoom = 1.0f / Mathf.Max(camera.Zoom.X, 0.1f);

        // Draw subtle crosshairs at origin
        float crosshairSize = 30f * inverseZoom;
        float crosshairThickness = 1.5f * inverseZoom;
        var crosshairColor = new Color(0.5f, 0.5f, 0.5f, 0.4f);

        // Horizontal line
        DrawLine(new Vector2(-crosshairSize, 0), new Vector2(crosshairSize, 0), crosshairColor, crosshairThickness);
        // Vertical line
        DrawLine(new Vector2(0, -crosshairSize), new Vector2(0, crosshairSize), crosshairColor, crosshairThickness);

        // Draw circle at center
        DrawCircle(Vector2.Zero, 3f * inverseZoom, crosshairColor);
    }
}

/// <summary>
/// Visual representation of a probe in flight on the star map.
/// Renders as a moving indicator between Sol and the target system.
/// </summary>
public partial class ProbeNode : Node2D
{
    private ProbeInFlight _probe = null!;
    private StarSystem _targetSystem = null!;
    private Vector2 _targetWorldPos;
    private double _totalTravelTime;

    public void Initialize(ProbeInFlight probe, StarSystem targetSystem, double currentGameTime)
    {
        _probe = probe;
        _targetSystem = targetSystem;

        // Calculate target position in world coordinates
        var targetLYPos = new Vector2(targetSystem.Position.X, targetSystem.Position.Y);
        _targetWorldPos = targetLYPos * StarMapPresenter.BASE_PIXELS_PER_LY;

        _totalTravelTime = probe.ArrivalTime - probe.LaunchedAt;

        UpdatePosition(currentGameTime);
    }

    public void UpdatePosition(double currentGameTime)
    {
        if (_probe == null) return;

        // Calculate progress (0.0 = just launched from Sol, 1.0 = arrived at target)
        var elapsed = currentGameTime - _probe.LaunchedAt;
        var progress = Mathf.Clamp((float)(elapsed / _totalTravelTime), 0f, 1f);

        // Interpolate between Sol (0,0) and target system
        Position = Vector2.Zero.Lerp(_targetWorldPos, progress);

        QueueRedraw();
    }

    public override void _Draw()
    {
        if (_probe == null) return;

        // Get camera for inverse zoom scaling
        var camera = GetViewport()?.GetCamera2D();
        if (camera == null) return;

        float inverseZoom = 1.0f / Mathf.Max(camera.Zoom.X, 0.1f);

        // Draw probe as a small triangle pointing towards target
        var direction = (_targetWorldPos - Position).Normalized();
        var angle = direction.Angle();

        // Create triangle vertices (pointing right by default)
        var size = 8f * inverseZoom;
        Vector2[] vertices = new[]
        {
            new Vector2(size, 0),              // Front point
            new Vector2(-size * 0.5f, size * 0.5f),   // Back top
            new Vector2(-size * 0.5f, -size * 0.5f)   // Back bottom
        };

        // Rotate vertices
        for (int i = 0; i < vertices.Length; i++)
        {
            vertices[i] = vertices[i].Rotated(angle);
        }

        // Draw filled triangle (cyan color for probes)
        DrawColoredPolygon(vertices, new Color(0.2f, 0.8f, 1.0f, 0.9f));

        // Draw outline
        DrawPolyline(new[] { vertices[0], vertices[1], vertices[2], vertices[0] },
                     new Color(1f, 1f, 1f, 1f), 1f * inverseZoom);

        // Draw progress trail (line from Sol to current position)
        if (Position.LengthSquared() > 1f)
        {
            DrawLine(Vector2.Zero, Vector2.Zero.DirectionTo(Position) * Position.Length(),
                     new Color(0.2f, 0.8f, 1.0f, 0.3f), 1.5f * inverseZoom);
        }
    }
}

