using Godot;
using OutpostGame.Core.Colony;
using OutpostGame.Core.Simulation;
using OutpostGame.Rendering;

namespace OutpostGame.Game;

/// <summary>
/// Phase-2 in-game HUD: turn counter, end-turn / skip controls, building selector,
/// and a live readout of resources, power, and population. Built entirely from code
/// so the .tscn stays small and the wiring is easy to inspect.
/// </summary>
public partial class ColonyHud : CanvasLayer
{
    private ColonySession? _session;
    private BuildingPlacer? _placer;

    private Label _solLabel = null!;
    private Label _resourceLabel = null!;
    private Label _powerLabel = null!;
    private Label _populationLabel = null!;
    private Label _statusLabel = null!;
    private OptionButton _buildingSelector = null!;
    private double _statusFadeTimer;

    public override void _Ready()
    {
        BuildUi();
    }

    public void Bind(ColonySession session, BuildingPlacer placer)
    {
        _session = session;
        _placer = placer;

        _buildingSelector.Clear();
        for (int i = 0; i < session.BuildableBuildings.Count; i++)
        {
            var def = session.BuildableBuildings[i];
            string costSummary = string.Join(", ",
                def.ConstructionCost.Select(kv => $"{kv.Value} {kv.Key}"));
            _buildingSelector.AddItem(
                $"[{i + 1}] {def.DisplayName}  ({def.Size.Width}x{def.Size.Height}, {costSummary})", i);
        }

        session.StateChanged += Refresh;
        placer.ActiveBuildingChanged += OnPlacerSelectionChanged;
        placer.PlacementRejected += FlashStatus;
        Refresh();
    }

    public override void _Process(double delta)
    {
        if (_statusFadeTimer > 0)
        {
            _statusFadeTimer -= delta;
            if (_statusFadeTimer <= 0)
            {
                _statusLabel.Text = "";
                _statusLabel.Modulate = new Color(1, 1, 1, 1);
            }
            else
            {
                _statusLabel.Modulate = new Color(1, 0.4f, 0.4f, (float)Math.Min(1.0, _statusFadeTimer));
            }
        }
    }

    private void OnPlacerSelectionChanged(string? buildingId)
    {
        if (_session == null) return;
        if (buildingId == null)
        {
            _buildingSelector.Selected = -1;
            return;
        }
        for (int i = 0; i < _session.BuildableBuildings.Count; i++)
        {
            if (_session.BuildableBuildings[i].Id == buildingId)
            {
                _buildingSelector.Selected = i;
                return;
            }
        }
    }

    private void FlashStatus(string message)
    {
        _statusLabel.Text = message;
        _statusFadeTimer = 3.0;
    }

    private void Refresh()
    {
        if (_session == null) return;
        var s = _session.State;

        _solLabel.Text = $"Sol {_session.CurrentSol}";
        _resourceLabel.Text = FormatResources(s.Resources);
        _powerLabel.Text =
            $"Power: {s.Power.TotalCapacity:F1} MW gen / {s.Power.TotalConsumption:F1} MW used"
            + (s.Power.Deficit > 0 ? $"  (deficit {s.Power.Deficit:F1} MW)" : "");
        int building = s.Grid.AllSlots.Count(b => b.State == BuildingState.UnderConstruction);
        int operational = s.Grid.AllSlots.Count(b => b.State == BuildingState.Operational);
        _populationLabel.Text =
            $"Pop {s.Population.Count}  •  Workers {s.Labor.AllocatedWorkers}/{s.Labor.TotalWorkers}"
            + $"  •  Morale {s.Population.Morale:F0}  •  Health {s.Population.Health:F0}"
            + $"  •  Buildings: {operational} ops / {building} building";
    }

    private static string FormatResources(ResourceStore store)
    {
        var snap = store.Snapshot()
            .Where(kv => kv.Value > 0.01f)
            .OrderBy(kv => kv.Key)
            .Select(kv => $"{kv.Key} {kv.Value:F0}");
        var joined = string.Join("   ", snap);
        return string.IsNullOrEmpty(joined) ? "Resources: (empty)" : "Resources: " + joined;
    }

    private void BuildUi()
    {
        // Top-left turn / building controls panel.
        var topLeft = new PanelContainer
        {
            AnchorLeft = 0, AnchorTop = 0,
            OffsetLeft = 12, OffsetTop = 12,
        };
        var topVbox = new VBoxContainer { CustomMinimumSize = new Vector2(420, 0) };
        topLeft.AddChild(topVbox);

        _solLabel = new Label { Text = "Sol 0" };
        _solLabel.AddThemeFontSizeOverride("font_size", 22);
        topVbox.AddChild(_solLabel);

        var turnButtons = new HBoxContainer();
        topVbox.AddChild(turnButtons);

        var endTurnBtn = new Button { Text = "End Turn (1 sol)" };
        endTurnBtn.Pressed += () => _session?.EndTurn(1);
        turnButtons.AddChild(endTurnBtn);

        var skip10Btn = new Button { Text = "Skip 10" };
        skip10Btn.Pressed += () => _session?.EndTurn(10);
        turnButtons.AddChild(skip10Btn);

        var skipMonthBtn = new Button { Text = "Skip Month (30)" };
        skipMonthBtn.Pressed += () => _session?.EndTurn(30);
        turnButtons.AddChild(skipMonthBtn);

        topVbox.AddChild(new HSeparator());

        topVbox.AddChild(new Label { Text = "Build" });
        _buildingSelector = new OptionButton();
        _buildingSelector.ItemSelected += idx =>
        {
            if (_session == null) return;
            int i = (int)idx;
            if (i < 0 || i >= _session.BuildableBuildings.Count) return;
            _placer?.SetActiveBuilding(_session.BuildableBuildings[i].Id);
        };
        topVbox.AddChild(_buildingSelector);

        var clearBtn = new Button { Text = "Cancel placement (Esc)" };
        clearBtn.Pressed += () => _placer?.SetActiveBuilding(null);
        topVbox.AddChild(clearBtn);

        topVbox.AddChild(new HSeparator());

        _populationLabel = new Label { AutowrapMode = TextServer.AutowrapMode.WordSmart };
        topVbox.AddChild(_populationLabel);

        _powerLabel = new Label { AutowrapMode = TextServer.AutowrapMode.WordSmart };
        topVbox.AddChild(_powerLabel);

        AddChild(topLeft);

        // Bottom resource ticker.
        var bottom = new PanelContainer
        {
            AnchorLeft = 0, AnchorRight = 1,
            AnchorTop = 1, AnchorBottom = 1,
            OffsetTop = -56, OffsetBottom = -8,
            OffsetLeft = 12, OffsetRight = -12,
        };
        var bottomVbox = new VBoxContainer();
        bottom.AddChild(bottomVbox);

        _resourceLabel = new Label
        {
            AutowrapMode = TextServer.AutowrapMode.WordSmart,
            HorizontalAlignment = HorizontalAlignment.Left,
        };
        bottomVbox.AddChild(_resourceLabel);

        _statusLabel = new Label
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            Modulate = new Color(1, 0.4f, 0.4f, 1),
        };
        bottomVbox.AddChild(_statusLabel);

        AddChild(bottom);
    }
}
