using Godot;
using OutpostGame.Core.Colony;
using OutpostGame.Core.World;

namespace OutpostGame.Rendering;

/// <summary>
/// Mouse-driven placement of placeholder buildings.
///
/// Phase 1 has no build menu — switch sizes with 1 / 2 / 3:
///   1 → 1x1 "ph_1x1"
///   2 → 2x2 "ph_2x2"
///   3 → 2x3 "ph_2x3"
///
/// Left-click commits a Place() through ColonyGrid; right-click clears the
/// active size (exits placement mode). The ghost footprint rotates with view
/// facing — the model footprint is constant but the *displayed* rectangle is
/// derived from the four iso-projected corners of the rotated cells.
/// </summary>
public partial class BuildingPlacer : Node2D
{
    private readonly record struct Placeholder(string Id, GridSize Size);

    private static readonly Placeholder[] Placeholders = new[]
    {
        new Placeholder("ph_1x1", new GridSize(1, 1)),
        new Placeholder("ph_2x2", new GridSize(2, 2)),
        new Placeholder("ph_2x3", new GridSize(2, 3)),
    };

    public ColonyGridView? GridView { get; set; }
    public IsometricCamera? Camera { get; set; }

    private int _placeholderIndex; // 0 = inactive, 1..3 = Placeholders[idx-1]
    private GridPosition? _hoverCell;
    private bool _validHover;

    private Font? _font;

    public override void _Ready()
    {
        ZIndex = 100; // draw on top
        _font = ThemeDB.FallbackFont;
        SetProcess(true);
    }

    public bool IsActive => _placeholderIndex > 0;
    private Placeholder? ActivePlaceholder =>
        IsActive ? Placeholders[_placeholderIndex - 1] : null;

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
        switch (k.Keycode)
        {
            case Key.Key1: _placeholderIndex = 1; QueueRedraw(); break;
            case Key.Key2: _placeholderIndex = 2; QueueRedraw(); break;
            case Key.Key3: _placeholderIndex = 3; QueueRedraw(); break;
            case Key.R: CyclePlaceholder(); break;
            case Key.Escape: _placeholderIndex = 0; QueueRedraw(); break;
        }
    }

    private void CyclePlaceholder()
    {
        if (!IsActive) { _placeholderIndex = 1; }
        else { _placeholderIndex = (_placeholderIndex % Placeholders.Length) + 1; }
        QueueRedraw();
    }

    private void HandleMouseButton(InputEventMouseButton mb)
    {
        if (!IsActive || GridView == null) return;

        if (mb.ButtonIndex == MouseButton.Right)
        {
            _placeholderIndex = 0;
            QueueRedraw();
            return;
        }

        if (mb.ButtonIndex == MouseButton.Left && _hoverCell.HasValue && _validHover)
        {
            var ph = ActivePlaceholder!.Value;
            var slot = new BuildingSlot
            {
                Origin = _hoverCell.Value,
                Size = ph.Size,
                BuildingDefinitionId = ph.Id,
                State = BuildingState.Planned,
            };
            var result = GridView.Grid.Place(slot);
            if (result.Success)
            {
                GridView.QueueRedraw();
            }
            QueueRedraw();
        }
    }

    public override void _Process(double delta)
    {
        if (!IsActive || GridView == null) return;

        // Translate the mouse from screen → world → GridView local space.
        Vector2 mouseWorld = GetGlobalMousePosition();
        Vector2 local = GridView.ToLocal(mouseWorld);
        var gp = GridView.ScreenToGrid(local);

        var ph = ActivePlaceholder!.Value;
        _hoverCell = gp;
        _validHover = IsFootprintValid(gp, ph.Size);
        QueueRedraw();
    }

    private bool IsFootprintValid(GridPosition origin, GridSize size)
    {
        if (GridView == null) return false;

        // Bounds + occupancy via ColonyGrid.Validate.
        var validation = GridView.Grid.Validate(origin, size);
        if (!validation.Success) return false;

        // Also forbid impassable terrain underneath any footprint cell.
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

        var ph = ActivePlaceholder!.Value;
        var origin = _hoverCell.Value;

        // Collect the four screen corners of the rotated bounding box.
        Vector2[]? corners = ComputeFootprintCorners(origin, ph.Size);
        if (corners == null) return;

        // BuildingPlacer is a sibling of GridView under the same parent — both share
        // the same parent transform. So we draw in the parent's local space using the
        // *GridView-local* coords directly (BuildingPlacer's own transform is identity).
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
            string text = $"{ph.Size.Width}x{ph.Size.Height} {ph.Id}";
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
