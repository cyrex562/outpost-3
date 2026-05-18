using Godot;
using OutpostGame.Core.Colony;
using OutpostGame.Core.Simulation;
using OutpostGame.Core.World;

namespace OutpostGame.Rendering;

/// <summary>
/// Mouse-driven placement of real (registry-backed) buildings. Phase 2 routes the
/// commit through <see cref="ColonySession.QueueConstruction"/>: the placer no longer
/// touches the grid directly, and the placed slot enters the <c>UnderConstruction</c>
/// lifecycle so it must tick to completion before becoming operational.
///
/// Hotkeys (also exposed via the HUD selector):
///   1 → Solar Array Mk1   (2x2 power producer)
///   2 → Basic Habitat     (3x3 essential housing)
///   3 → Iron Mine         (3x3 ore extractor)
///   ESC / right-click     → exit placement mode
/// </summary>
public partial class BuildingPlacer : Node2D
{
    public ColonyGridView? GridView { get; set; }
    public IsometricCamera? Camera { get; set; }
    public ColonySession? Session { get; set; }

    /// <summary>Fires when the active building changes (incl. cleared). HUD listens
    /// to keep its selector in sync with hotkey changes.</summary>
    public event Action<string?>? ActiveBuildingChanged;

    /// <summary>Fires when a placement is rejected (resources, bounds, occupancy).
    /// HUD listens so it can flash a brief banner.</summary>
    public event Action<string>? PlacementRejected;

    /// <summary>Fires when the player left-clicks a placed building while not in
    /// placement mode. Passes the clicked slot id, or null when the click missed
    /// every footprint. HUD listens to open / close its details panel.</summary>
    public event Action<Guid?>? BuildingSelected;

    private string? _activeBuildingId;
    private GridSize _activeSize;
    private GridPosition? _hoverCell;
    private bool _validHover;

    private Font? _font;

    public override void _Ready()
    {
        ZIndex = 100;
        _font = ThemeDB.FallbackFont;
        SetProcess(true);
    }

    public bool IsActive => _activeBuildingId != null;
    public string? ActiveBuildingId => _activeBuildingId;

    public void SetActiveBuilding(string? buildingId)
    {
        if (buildingId == null)
        {
            if (_activeBuildingId != null)
            {
                _activeBuildingId = null;
                ActiveBuildingChanged?.Invoke(null);
                QueueRedraw();
            }
            return;
        }

        if (!BuildingRegistry.TryGet(buildingId, out var def) || def is null) return;
        _activeBuildingId = buildingId;
        _activeSize = def.Size;
        ActiveBuildingChanged?.Invoke(buildingId);
        QueueRedraw();
    }

    public override void _UnhandledInput(InputEvent @event)
    {
        switch (@event)
        {
            case InputEventKey k when k.Pressed && !k.Echo:
                HandleKey(k);
                break;
            case InputEventMouseButton mb when mb.Pressed:
                HandleMouseButton(mb);
                break;
        }
    }

    private void HandleKey(InputEventKey k)
    {
        if (Session == null) return;
        var list = Session.BuildableBuildings;
        switch (k.Keycode)
        {
            case Key.Key1: if (list.Count > 0) SetActiveBuilding(list[0].Id); break;
            case Key.Key2: if (list.Count > 1) SetActiveBuilding(list[1].Id); break;
            case Key.Key3: if (list.Count > 2) SetActiveBuilding(list[2].Id); break;
            case Key.Escape: SetActiveBuilding(null); break;
        }
    }

    private void HandleMouseButton(InputEventMouseButton mb)
    {
        if (GridView == null || Session == null) return;

        if (mb.ButtonIndex == MouseButton.Right)
        {
            if (IsActive) SetActiveBuilding(null);
            return;
        }

        if (mb.ButtonIndex != MouseButton.Left) return;

        if (IsActive)
        {
            // Placement mode — try to queue construction at the hovered cell.
            if (_hoverCell.HasValue && _validHover)
            {
                var result = Session.QueueConstruction(_activeBuildingId!, _hoverCell.Value);
                if (!result.Success)
                    PlacementRejected?.Invoke(result.FailureReason ?? "Placement failed");
                QueueRedraw();
            }
            return;
        }

        // Selection mode — identify the slot under the cursor, if any.
        Vector2 mouseWorld = GetGlobalMousePosition();
        Vector2 local = GridView.ToLocal(mouseWorld);
        var cell = GridView.ScreenToGrid(local);
        if (!GridView.InBounds(cell))
        {
            BuildingSelected?.Invoke(null);
            return;
        }
        var slot = Session.State.Grid.GetSlotAtCell(cell);
        BuildingSelected?.Invoke(slot?.Id);
    }

    public override void _Process(double delta)
    {
        if (!IsActive || GridView == null) return;

        Vector2 mouseWorld = GetGlobalMousePosition();
        Vector2 local = GridView.ToLocal(mouseWorld);
        _hoverCell = GridView.ScreenToGrid(local);
        _validHover = IsFootprintValid(_hoverCell.Value, _activeSize);
        QueueRedraw();
    }

    private bool IsFootprintValid(GridPosition origin, GridSize size)
    {
        if (GridView == null) return false;

        var validation = GridView.Grid.Validate(origin, size);
        if (!validation.Success) return false;

        for (int dx = 0; dx < size.Width; dx++)
        for (int dy = 0; dy < size.Height; dy++)
        {
            var cell = new GridPosition(origin.X + dx, origin.Y + dy);
            if (GridView.TerrainAt(cell) == TerrainType.Impassable)
                return false;
        }
        return true;
    }

    public override void _Draw()
    {
        if (!IsActive || GridView == null || !_hoverCell.HasValue) return;

        var origin = _hoverCell.Value;
        Vector2[]? corners = ComputeFootprintCorners(origin, _activeSize);
        if (corners == null) return;

        Color fill = _validHover
            ? new Color(0.3f, 0.9f, 0.3f, 0.45f)
            : new Color(0.9f, 0.25f, 0.25f, 0.45f);
        Color outline = _validHover
            ? new Color(0.1f, 0.6f, 0.1f, 0.9f)
            : new Color(0.7f, 0.1f, 0.1f, 0.9f);

        DrawColoredPolygon(corners, fill);
        DrawPolyline(new[] { corners[0], corners[1], corners[2], corners[3], corners[0] }, outline, 2f);

        if (_font != null)
        {
            Vector2 center = (corners[0] + corners[2]) * 0.5f;
            string name = BuildingRegistry.TryGet(_activeBuildingId!, out var def) && def is not null
                ? def.DisplayName
                : _activeBuildingId!;
            string text = $"{_activeSize.Width}x{_activeSize.Height} {name}";
            var size = _font.GetStringSize(text);
            DrawString(_font, center - size * 0.5f + new Vector2(0, size.Y * 0.25f),
                text, HorizontalAlignment.Left, -1, 12, new Color(1, 1, 1));
        }
    }

    private Vector2[]? ComputeFootprintCorners(GridPosition origin, GridSize size)
    {
        if (GridView == null) return null;

        int w = GridView.Width;
        int h = GridView.Height;
        int facing = GridView.Facing;

        int minRx = int.MaxValue, minRy = int.MaxValue, maxRx = int.MinValue, maxRy = int.MinValue;
        for (int dx = 0; dx < size.Width; dx++)
        for (int dy = 0; dy < size.Height; dy++)
        {
            var (rx, ry) = IsoProjection.ApplyFacing(origin.X + dx, origin.Y + dy, w, h, facing);
            if (rx < minRx) minRx = rx;
            if (ry < minRy) minRy = ry;
            if (rx > maxRx) maxRx = rx;
            if (ry > maxRy) maxRy = ry;
        }

        Vector2 top = IsoProjection.GridToScreen(minRx, minRy) + new Vector2(0, -IsoProjection.TileHeight * 0.5f);
        Vector2 right = IsoProjection.GridToScreen(maxRx, minRy) + new Vector2(IsoProjection.TileWidth * 0.5f, 0);
        Vector2 bottom = IsoProjection.GridToScreen(maxRx, maxRy) + new Vector2(0, IsoProjection.TileHeight * 0.5f);
        Vector2 left = IsoProjection.GridToScreen(minRx, maxRy) + new Vector2(-IsoProjection.TileWidth * 0.5f, 0);
        return new[] { top, right, bottom, left };
    }
}
