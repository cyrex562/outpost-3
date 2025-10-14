using System;
using Godot;
using Outpost3.Core;
using Outpost3.Core.Commands;
using Outpost3.Core.Domain;

namespace Outpost3.UI.Common;

/// <summary>
/// Reusable camera controller for 2D viewport navigation.
/// Handles panning, zooming, and coordinate conversions for both galaxy and system maps.
/// </summary>
public class CameraController
{
    private Camera2D _camera;
    private SubViewport _viewport;
    private StateStore _stateStore;

    // Panning state
    private bool _isPanning = false;
    private Vector2 _panStartMousePos;
    private Vector2 _panStartCameraPos;

    // Zoom constraints
    public float MinZoom { get; set; } = 0.1f;
    public float MaxZoom { get; set; } = 20.0f;
    public float ZoomStep { get; set; } = 0.1f;

    public CameraController(Camera2D camera, SubViewport viewport, StateStore stateStore)
    {
        _camera = camera ?? throw new ArgumentNullException(nameof(camera));
        _viewport = viewport ?? throw new ArgumentNullException(nameof(viewport));
        _stateStore = stateStore ?? throw new ArgumentNullException(nameof(stateStore));

        // Configure Camera2D for proper centering
        _camera.AnchorMode = Camera2D.AnchorModeEnum.DragCenter;
    }

    /// <summary>
    /// Current zoom level of the camera.
    /// </summary>
    public float CurrentZoom => _camera.Zoom.X;

    /// <summary>
    /// Current position of the camera in world coordinates.
    /// </summary>
    public Vector2 CurrentPosition => _camera.Position;

    /// <summary>
    /// Whether the camera is currently being panned.
    /// </summary>
    public bool IsPanning => _isPanning;

    /// <summary>
    /// Handle mouse button input for camera controls.
    /// Returns true if the input was handled.
    /// </summary>
    public bool HandleMouseButton(InputEventMouseButton mouseButton)
    {
        if (mouseButton.ButtonIndex == MouseButton.WheelUp)
        {
            ZoomIn();
            return true;
        }
        else if (mouseButton.ButtonIndex == MouseButton.WheelDown)
        {
            ZoomOut();
            return true;
        }
        else if (mouseButton.ButtonIndex == MouseButton.Right)
        {
            if (mouseButton.Pressed)
            {
                StartPanning(mouseButton.Position);
            }
            else
            {
                StopPanning();
            }
            return true;
        }

        return false;
    }

    /// <summary>
    /// Handle mouse motion input for camera panning.
    /// Returns true if the input was handled.
    /// </summary>
    public bool HandleMouseMotion(InputEventMouseMotion mouseMotion)
    {
        if (_isPanning)
        {
            UpdatePanning(mouseMotion.Position);
            return true;
        }

        return false;
    }

    /// <summary>
    /// Start panning from the given screen position.
    /// </summary>
    public void StartPanning(Vector2 screenPosition)
    {
        _isPanning = true;
        _panStartMousePos = screenPosition;
        _panStartCameraPos = _camera.Position;
    }

    /// <summary>
    /// Stop panning.
    /// </summary>
    public void StopPanning()
    {
        _isPanning = false;
    }

    /// <summary>
    /// Update panning to the given screen position.
    /// </summary>
    public void UpdatePanning(Vector2 currentScreenPosition)
    {
        if (!_isPanning) return;

        var mouseDelta = currentScreenPosition - _panStartMousePos;
        var worldDelta = mouseDelta / _camera.Zoom.X;
        var newCameraPos = _panStartCameraPos - worldDelta;

        _camera.Position = newCameraPos;
    }

    /// <summary>
    /// Zoom in by the configured zoom step.
    /// </summary>
    public void ZoomIn()
    {
        var newZoom = Math.Min(CurrentZoom + ZoomStep, MaxZoom);
        SetZoom(newZoom);
    }

    /// <summary>
    /// Zoom out by the configured zoom step.
    /// </summary>
    public void ZoomOut()
    {
        var newZoom = Math.Max(CurrentZoom - ZoomStep, MinZoom);
        SetZoom(newZoom);
    }

    /// <summary>
    /// Set the zoom level directly.
    /// </summary>
    public void SetZoom(float zoom)
    {
        var clampedZoom = Math.Max(MinZoom, Math.Min(zoom, MaxZoom));
        _camera.Zoom = new Vector2(clampedZoom, clampedZoom);
    }

    /// <summary>
    /// Set the camera position and zoom.
    /// </summary>
    public void SetPositionAndZoom(Vector2 position, float zoom)
    {
        _camera.Position = position;
        SetZoom(zoom);
    }

    /// <summary>
    /// Reset camera to origin with default zoom.
    /// </summary>
    public void ResetToOrigin(float defaultZoom = 1.0f)
    {
        _camera.Position = Vector2.Zero;
        SetZoom(defaultZoom);
    }

    /// <summary>
    /// Convert screen coordinates to world coordinates.
    /// </summary>
    public Vector2 ScreenToWorld(Vector2 screenPosition)
    {
        var viewportSize = _viewport.GetVisibleRect().Size;
        var centeredPos = screenPosition - viewportSize / 2.0f;
        return (centeredPos / _camera.Zoom.X) + _camera.Position;
    }

    /// <summary>
    /// Convert world coordinates to screen coordinates.
    /// </summary>
    public Vector2 WorldToScreen(Vector2 worldPosition)
    {
        var viewportSize = _viewport.GetVisibleRect().Size;
        var relativePos = (worldPosition - _camera.Position) * _camera.Zoom.X;
        return relativePos + viewportSize / 2.0f;
    }

    /// <summary>
    /// Get a formatted string describing the current camera state for UI display.
    /// </summary>
    public string GetCameraInfoText(float pixelsPerUnit = 1.0f)
    {
        var positionInUnits = _camera.Position / pixelsPerUnit;
        return $"Zoom: {CurrentZoom:F2}x | Center: ({positionInUnits.X:F1}, {positionInUnits.Y:F1})";
    }
}