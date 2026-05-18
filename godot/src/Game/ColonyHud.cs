using Godot;
using OutpostGame.Core.Colony;
using OutpostGame.Core.Simulation;
using OutpostGame.Rendering;

namespace OutpostGame.Game;

/// <summary>
/// Phase-3 in-game HUD: turn counter, end-turn / skip controls, building selector,
/// auto-assign labor, live resource / power / population readout, and an event log panel.
/// Built entirely from code so the .tscn stays small and the wiring is easy to inspect.
/// </summary>
public partial class ColonyHud : CanvasLayer
{
    private ColonySession? _session;
    private BuildingPlacer? _placer;

    private Label _solLabel = null!;
    private HBoxContainer _resourceFlow = null!;
    private HBoxContainer _powerTile = null!;
    private Label _powerTileText = null!;
    private Label _populationHeaderLabel = null!;
    private ProgressBar _healthBar = null!;
    private ProgressBar _moraleBar = null!;
    private Label _healthValueLabel = null!;
    private Label _moraleValueLabel = null!;
    private readonly Dictionary<string, ProgressBar> _needsBars = new();
    private readonly Dictionary<string, Label> _needsValueLabels = new();
    private Label _skillsLabel = null!;
    private Label _statusLabel = null!;
    private Label _stormLabel = null!;
    private Label _colonyStatusLabel = null!;
    private Control _terminalOverlay = null!;
    private VBoxContainer _underConstructionContainer = null!;
    private Label _underConstructionHeader = null!;
    private VBoxContainer _eventLogContainer = null!;
    private VBoxContainer _damagedContainer = null!;
    private readonly Dictionary<string, Button> _buildButtons = new();
    private double _statusFadeTimer;

    // Decision modal
    private Control _decisionOverlay = null!;
    private Label _decisionTitle = null!;
    private Label _decisionDesc = null!;
    private VBoxContainer _decisionChoices = null!;

    // Critical event alert
    private Label _criticalBadgeLabel = null!;

    // Save/load modal
    private Control _saveModalOverlay = null!;
    private VBoxContainer _saveModalSlots = null!;
    private Label _saveModalTitle = null!;

    // In-game (Esc) menu
    private Control _escMenuOverlay = null!;

    // Top-bar compact stat buttons
    private Button _popStatBtn = null!;
    private Button _laborStatBtn = null!;
    private Button _buildingsStatBtn = null!;
    private Button _fleetStatBtn = null!;

    // Buildings panel (opens from the 🏗 stat button)
    private Control _buildingsPanel = null!;
    private Label _buildingsPanelSummary = null!;

    // Fleet panel (opens from the 🚜 stat button)
    private Control _fleetPanel = null!;
    private Label _fleetPanelSummary = null!;
    private VBoxContainer _fleetFactoryList = null!;

    // Operators section inside Labor panel
    private Label _operatorsSummaryLabel = null!;

    // Resource details panel
    private Control _resourceDetailsPanel = null!;
    private Label _resourceDetailsTitle = null!;
    private VBoxContainer _resourceDetailsBody = null!;
    private string? _selectedResourceId;

    // Tech tree panel
    private Button _techStatBtn = null!;
    private Control _techPanel = null!;
    private VBoxContainer _techList = null!;

    // Side panels (left-anchored, toggle from top-bar stats)
    private Control _popPanel = null!;
    private Control _laborPanel = null!;
    private VBoxContainer _laborPerBuildingList = null!;
    private Label _laborSummaryLabel = null!;

    // Building details panel (opens on building click)
    private Control _buildingDetailsPanel = null!;
    private Label _buildingDetailsTitle = null!;
    private VBoxContainer _buildingDetailsBody = null!;
    private Guid? _selectedSlotId;

    // Bottom build bar + tile popup
    private Control _bottomBuildBar = null!;
    private HBoxContainer _buildCategoryRow = null!;
    private Control _buildTilePopup = null!;
    private HBoxContainer _buildTileRow = null!;
    private BuildingCategory? _openBuildCategory;

    public override void _Ready()
    {
        BuildUi();
    }

    public void Bind(ColonySession session, BuildingPlacer placer)
    {
        _session = session;
        _placer = placer;

        session.StateChanged += Refresh;
        placer.ActiveBuildingChanged += OnPlacerSelectionChanged;
        placer.PlacementRejected += FlashStatus;
        placer.BuildingSelected += OnBuildingSelected;
        Refresh();
    }

    private void OnBuildingSelected(Guid? slotId)
    {
        // Click landed on a placed building (not in placement mode).
        _selectedSlotId = slotId;
        if (slotId == null)
        {
            _buildingDetailsPanel.Visible = false;
            return;
        }
        // Close any other side panels first; show building details.
        _popPanel.Visible = false;
        _laborPanel.Visible = false;
        _buildingsPanel.Visible = false;
        _fleetPanel.Visible = false;
        if (_resourceDetailsPanel != null) _resourceDetailsPanel.Visible = false;
        if (_techPanel != null) _techPanel.Visible = false;
        _buildingDetailsPanel.Visible = true;
        RefreshBuildingDetailsPanel();
    }

    private static string CategoryName(BuildingCategory c) => c switch
    {
        BuildingCategory.Power       => "Power",
        BuildingCategory.LifeSupport => "Life Support",
        BuildingCategory.Habitat     => "Habitat",
        BuildingCategory.Production  => "Production",
        BuildingCategory.Storage     => "Storage",
        _                            => "Other",
    };

    private void BuildTopBar()
    {
        // Full-width horizontal bar at the top of the screen carrying the turn
        // controls on the left and the resource tiles on the right.
        var top = new PanelContainer
        {
            AnchorLeft = 0, AnchorRight = 1,
            AnchorTop  = 0, AnchorBottom = 0,
            OffsetLeft = 8, OffsetRight = -8, OffsetTop = 6,
        };
        var topRow = new HBoxContainer();
        topRow.AddThemeConstantOverride("separation", 12);
        top.AddChild(topRow);

        // ── Left: turn controls ──────────────────────────────────────────────
        var controls = new HBoxContainer();
        controls.AddThemeConstantOverride("separation", 6);

        _solLabel = new Label { Text = "Sol 0", CustomMinimumSize = new Vector2(90, 0) };
        _solLabel.AddThemeFontSizeOverride("font_size", 18);
        controls.AddChild(_solLabel);

        var endTurnBtn = new Button { Text = "End Turn" };
        endTurnBtn.Pressed += () => _session?.EndTurn(1);
        controls.AddChild(endTurnBtn);

        var skip10Btn = new Button { Text = "Skip 10" };
        skip10Btn.Pressed += () => _session?.EndTurn(10);
        controls.AddChild(skip10Btn);

        var skipMonthBtn = new Button { Text = "Skip 30" };
        skipMonthBtn.Pressed += () => _session?.EndTurn(30);
        controls.AddChild(skipMonthBtn);

        controls.AddChild(new VSeparator());

        var skipNLabel = new Label { Text = "N:" };
        controls.AddChild(skipNLabel);

        var skipNSpin = new SpinBox
        {
            MinValue = 1, MaxValue = 9999, Value = 50, Step = 1,
            CustomMinimumSize = new Vector2(90, 0),
        };
        controls.AddChild(skipNSpin);

        var skipNBtn = new Button { Text = "Skip" };
        skipNBtn.Pressed += () => _session?.EndTurn((int)skipNSpin.Value);
        controls.AddChild(skipNBtn);

        var skipNFastBtn = new Button
        {
            Text = "⚡",
            TooltipText = "Fast skip — advances N sols without rendering intermediates.",
        };
        skipNFastBtn.Pressed += () => _session?.EndTurn((int)skipNSpin.Value, fast: true);
        controls.AddChild(skipNFastBtn);

        controls.AddChild(new VSeparator());

        // Storm / colony status labels live inline so they don't bloat the left panel.
        _stormLabel = new Label
        {
            AutowrapMode = TextServer.AutowrapMode.WordSmart,
            Modulate = new Color(1f, 0.6f, 0.2f),
            Visible = false,
        };
        _stormLabel.AddThemeFontSizeOverride("font_size", 12);
        controls.AddChild(_stormLabel);

        _colonyStatusLabel = new Label
        {
            AutowrapMode = TextServer.AutowrapMode.WordSmart,
            Visible = false,
        };
        _colonyStatusLabel.AddThemeFontSizeOverride("font_size", 13);
        controls.AddChild(_colonyStatusLabel);

        // Flash status (e.g. "Saved.", "Insufficient resources!") in the gap.
        _statusLabel = new Label
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            Modulate = new Color(1, 0.4f, 0.4f, 1),
        };
        controls.AddChild(_statusLabel);

        topRow.AddChild(controls);

        // ── Middle-right: resource tiles + pop/labor stat buttons. ExpandFill
        // gives the HBox the remaining width so tiles lay out horizontally. ─
        _resourceFlow = new HBoxContainer { SizeFlagsHorizontal = Control.SizeFlags.ExpandFill };
        _resourceFlow.AddThemeConstantOverride("separation", 10);
        _resourceFlow.Alignment = BoxContainer.AlignmentMode.End;
        topRow.AddChild(_resourceFlow);

        // Power tile — like a resource tile, but lives outside _resourceFlow so
        // it doesn't get clobbered on every refresh.
        _powerTile = new HBoxContainer();
        _powerTile.AddThemeConstantOverride("separation", 4);
        var powerIcon = new Label { Text = "⚡" };
        powerIcon.AddThemeFontSizeOverride("font_size", 11);
        powerIcon.Modulate = new Color(0.7f, 0.85f, 0.95f);
        _powerTile.AddChild(powerIcon);
        _powerTileText = new Label { Text = "0/0 MW" };
        _powerTileText.AddThemeFontSizeOverride("font_size", 11);
        _powerTile.AddChild(_powerTileText);
        topRow.AddChild(_powerTile);

        // Pop / labor / buildings stats — clickable, each toggles its own panel.
        _popStatBtn = new Button { Text = "👥 0", Flat = true };
        _popStatBtn.AddThemeFontSizeOverride("font_size", 12);
        _popStatBtn.TooltipText = "Population — click for details";
        _popStatBtn.Pressed += () => TogglePanel(_popPanel);
        topRow.AddChild(_popStatBtn);

        _laborStatBtn = new Button { Text = "⚒ 0/0", Flat = true };
        _laborStatBtn.AddThemeFontSizeOverride("font_size", 12);
        _laborStatBtn.TooltipText = "Labor — click for assignments";
        _laborStatBtn.Pressed += () => TogglePanel(_laborPanel);
        topRow.AddChild(_laborStatBtn);

        _buildingsStatBtn = new Button { Text = "🏗 0", Flat = true };
        _buildingsStatBtn.AddThemeFontSizeOverride("font_size", 12);
        _buildingsStatBtn.TooltipText = "Buildings — click for status";
        _buildingsStatBtn.Pressed += () => TogglePanel(_buildingsPanel);
        topRow.AddChild(_buildingsStatBtn);

        _fleetStatBtn = new Button { Text = "🚜 0/10", Flat = true };
        _fleetStatBtn.AddThemeFontSizeOverride("font_size", 12);
        _fleetStatBtn.TooltipText = "Construction fleet — click for details";
        _fleetStatBtn.Pressed += () => TogglePanel(_fleetPanel);
        topRow.AddChild(_fleetStatBtn);

        _techStatBtn = new Button { Text = "🔬", Flat = true };
        _techStatBtn.AddThemeFontSizeOverride("font_size", 12);
        _techStatBtn.TooltipText = "Tech tree";
        _techStatBtn.Pressed += () => TogglePanel(_techPanel);
        topRow.AddChild(_techStatBtn);

        AddChild(top);
    }

    private void TogglePanel(Control panel)
    {
        bool willOpen = !panel.Visible;
        // Close other side panels so only one is visible at a time.
        if (willOpen)
        {
            _popPanel.Visible = false;
            _laborPanel.Visible = false;
            _buildingsPanel.Visible = false;
            _fleetPanel.Visible = false;
            if (_resourceDetailsPanel != null) _resourceDetailsPanel.Visible = false;
            if (_techPanel != null) _techPanel.Visible = false;
            _buildingDetailsPanel.Visible = false;
        }
        panel.Visible = willOpen;
    }

    private void BuildFleetPanel()
    {
        var panel = new PanelContainer
        {
            AnchorLeft = 0, AnchorTop = 0,
            OffsetLeft = 8, OffsetTop = 56,
            Visible = false,
            MouseFilter = Control.MouseFilterEnum.Stop,
        };
        var vbox = new VBoxContainer { CustomMinimumSize = new Vector2(440, 0) };
        vbox.AddThemeConstantOverride("separation", 4);
        panel.AddChild(vbox);

        var headerRow = new HBoxContainer();
        var hdrLbl = new Label
        {
            Text = "Construction Fleet",
            SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
        };
        hdrLbl.AddThemeFontSizeOverride("font_size", 14);
        headerRow.AddChild(hdrLbl);
        var closeBtn = new Button { Text = "✕", Flat = true };
        closeBtn.Pressed += () => panel.Visible = false;
        headerRow.AddChild(closeBtn);
        vbox.AddChild(headerRow);
        vbox.AddChild(new HSeparator());

        _fleetPanelSummary = new Label { AutowrapMode = TextServer.AutowrapMode.WordSmart };
        _fleetPanelSummary.AddThemeFontSizeOverride("font_size", 12);
        vbox.AddChild(_fleetPanelSummary);

        vbox.AddChild(new HSeparator());

        var listHdr = new Label { Text = "Vehicle Factories" };
        listHdr.AddThemeFontSizeOverride("font_size", 12);
        listHdr.Modulate = new Color(0.65f, 0.85f, 1f);
        vbox.AddChild(listHdr);

        _fleetFactoryList = new VBoxContainer();
        vbox.AddChild(_fleetFactoryList);

        _fleetPanel = panel;
        AddChild(panel);
    }

    private void BuildTechPanel()
    {
        var panel = new PanelContainer
        {
            AnchorLeft = 0, AnchorTop = 0,
            OffsetLeft = 8, OffsetTop = 56,
            Visible = false,
            MouseFilter = Control.MouseFilterEnum.Stop,
        };
        var vbox = new VBoxContainer { CustomMinimumSize = new Vector2(480, 0) };
        vbox.AddThemeConstantOverride("separation", 4);
        panel.AddChild(vbox);

        var headerRow = new HBoxContainer();
        var hdr = new Label
        {
            Text = "Tech Tree",
            SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
        };
        hdr.AddThemeFontSizeOverride("font_size", 14);
        headerRow.AddChild(hdr);
        var closeBtn = new Button { Text = "✕", Flat = true };
        closeBtn.Pressed += () => panel.Visible = false;
        headerRow.AddChild(closeBtn);
        vbox.AddChild(headerRow);

        var note = new Label
        {
            Text = "Research is data-only for now — wire-up in a later phase.",
            AutowrapMode = TextServer.AutowrapMode.WordSmart,
        };
        note.AddThemeFontSizeOverride("font_size", 10);
        note.Modulate = new Color(0.65f, 0.65f, 0.7f);
        vbox.AddChild(note);

        vbox.AddChild(new HSeparator());

        _techList = new VBoxContainer();
        _techList.AddThemeConstantOverride("separation", 4);
        vbox.AddChild(_techList);

        _techPanel = panel;
        AddChild(panel);

        PopulateTechList();
    }

    private void PopulateTechList()
    {
        foreach (Node child in _techList.GetChildren()) child.QueueFree();

        var grouped = TechRegistry.All.Values
            .GroupBy(t => t.Tier)
            .OrderBy(g => g.Key);

        foreach (var group in grouped)
        {
            var tierLbl = new Label { Text = $"Tier {group.Key}" };
            tierLbl.AddThemeFontSizeOverride("font_size", 12);
            tierLbl.Modulate = new Color(0.65f, 0.85f, 1f);
            _techList.AddChild(tierLbl);

            foreach (var tech in group.OrderBy(t => t.DisplayName))
            {
                var box = new VBoxContainer();
                box.AddThemeConstantOverride("separation", 1);

                var nameLbl = new Label
                {
                    Text = $"  {tech.DisplayName}   (cost {tech.ResearchCost:F0}, {tech.ResearchTurns} sols)",
                };
                nameLbl.AddThemeFontSizeOverride("font_size", 11);
                box.AddChild(nameLbl);

                if (tech.Prerequisites.Count > 0)
                {
                    var prereqLbl = new Label
                    {
                        Text = "    requires: " + string.Join(", ",
                            tech.Prerequisites.Select(p =>
                                TechRegistry.TryGet(p, out var pd) && pd != null ? pd.DisplayName : p)),
                    };
                    prereqLbl.AddThemeFontSizeOverride("font_size", 10);
                    prereqLbl.Modulate = new Color(0.75f, 0.75f, 0.85f);
                    box.AddChild(prereqLbl);
                }

                var parts = new List<string>();
                if (tech.Unlocks.Buildings.Count > 0)
                    parts.Add("buildings: " + string.Join(", ", tech.Unlocks.Buildings));
                if (tech.Unlocks.Resources.Count > 0)
                    parts.Add("resources: " + string.Join(", ", tech.Unlocks.Resources));
                if (tech.Unlocks.Bonuses.Count > 0)
                    parts.Add("bonuses: " + string.Join(", ", tech.Unlocks.Bonuses));
                if (parts.Count > 0)
                {
                    var unlocksLbl = new Label
                    {
                        Text = "    unlocks: " + string.Join("  •  ", parts),
                        AutowrapMode = TextServer.AutowrapMode.WordSmart,
                    };
                    unlocksLbl.AddThemeFontSizeOverride("font_size", 10);
                    unlocksLbl.Modulate = new Color(0.7f, 1f, 0.85f);
                    box.AddChild(unlocksLbl);
                }

                if (!string.IsNullOrEmpty(tech.Description))
                {
                    var descLbl = new Label
                    {
                        Text = "    " + tech.Description,
                        AutowrapMode = TextServer.AutowrapMode.WordSmart,
                    };
                    descLbl.AddThemeFontSizeOverride("font_size", 10);
                    descLbl.Modulate = new Color(0.65f, 0.65f, 0.7f);
                    box.AddChild(descLbl);
                }

                _techList.AddChild(box);
            }
        }
    }

    private void BuildResourceDetailsPanel()
    {
        var panel = new PanelContainer
        {
            AnchorLeft = 0, AnchorTop = 0,
            OffsetLeft = 8, OffsetTop = 56,
            Visible = false,
            MouseFilter = Control.MouseFilterEnum.Stop,
        };
        var vbox = new VBoxContainer { CustomMinimumSize = new Vector2(440, 0) };
        vbox.AddThemeConstantOverride("separation", 4);
        panel.AddChild(vbox);

        var headerRow = new HBoxContainer();
        _resourceDetailsTitle = new Label
        {
            Text = "Resource",
            SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
        };
        _resourceDetailsTitle.AddThemeFontSizeOverride("font_size", 14);
        headerRow.AddChild(_resourceDetailsTitle);
        var closeBtn = new Button { Text = "✕", Flat = true };
        closeBtn.Pressed += () =>
        {
            _selectedResourceId = null;
            panel.Visible = false;
        };
        headerRow.AddChild(closeBtn);
        vbox.AddChild(headerRow);
        vbox.AddChild(new HSeparator());

        _resourceDetailsBody = new VBoxContainer();
        _resourceDetailsBody.AddThemeConstantOverride("separation", 4);
        vbox.AddChild(_resourceDetailsBody);

        _resourceDetailsPanel = panel;
        AddChild(panel);
    }

    private void RefreshResourceDetails()
    {
        if (_session == null || _selectedResourceId is not string id)
        {
            _resourceDetailsPanel.Visible = false;
            return;
        }
        if (!ResourceRegistry.TryGet(id, out var def) || def == null)
        {
            _resourceDetailsPanel.Visible = false;
            return;
        }
        var s = _session.State;
        float amount = s.Resources.Get(id);
        float cap = s.Resources.Cap(id);
        bool capped = cap < float.MaxValue && cap > 0f;
        float delta = s.ResourceDeltas.TryGetValue(id, out var d) ? d : 0f;

        _resourceDetailsTitle.Text = $"{def.DisplayName} ({def.Tier} • {def.Category})";

        foreach (Node child in _resourceDetailsBody.GetChildren()) child.QueueFree();

        void AddLine(string text, Color? mod = null, int size = 11)
        {
            var lbl = new Label { Text = text, AutowrapMode = TextServer.AutowrapMode.WordSmart };
            lbl.AddThemeFontSizeOverride("font_size", size);
            if (mod.HasValue) lbl.Modulate = mod.Value;
            _resourceDetailsBody.AddChild(lbl);
        }

        AddLine(capped ? $"Stock: {amount:F1} / {cap:F0}" : $"Stock: {amount:F1}");
        string deltaText = $"Δ {delta:+0.0;-0.0;0.0} per sol";
        AddLine(deltaText, TrendColor(delta));
        if (!string.IsNullOrEmpty(def.Description)) AddLine(def.Description, new Color(0.7f, 0.85f, 0.95f), 10);

        _resourceDetailsBody.AddChild(new HSeparator());

        // Producers — operational buildings whose recipe outputs include this id,
        // plus power production (if id is "power"; not stored as a resource today).
        var producers = s.Grid.AllSlots
            .Where(slot => slot.State == BuildingState.Operational)
            .Select(slot => (slot, def: BuildingRegistry.Get(slot.BuildingDefinitionId)))
            .Where(pair => pair.def.Recipe != null
                        && pair.def.Recipe.Outputs.ContainsKey(id))
            .ToList();
        var producerHdr = new Label { Text = $"Producers ({producers.Count})" };
        producerHdr.AddThemeFontSizeOverride("font_size", 12);
        producerHdr.Modulate = new Color(0.5f, 1f, 0.7f);
        _resourceDetailsBody.AddChild(producerHdr);
        if (producers.Count == 0)
        {
            AddLine("  (none — build something with this output to start production)",
                    new Color(0.6f, 0.6f, 0.6f), 10);
        }
        else
        {
            foreach (var (slot, bdef) in producers)
            {
                float per = bdef.Recipe!.Outputs[id];
                int cycle = bdef.Recipe.TurnsPerCycle;
                AddLine($"  • {bdef.DisplayName}: {per:F1}/cycle ({cycle} sols)");
            }
        }

        _resourceDetailsBody.AddChild(new HSeparator());

        // Consumers — recipes whose Inputs include this id, plus maintenance,
        // plus population needs (food/water/oxygen).
        var consumers = s.Grid.AllSlots
            .Where(slot => slot.State == BuildingState.Operational)
            .Select(slot => (slot, def: BuildingRegistry.Get(slot.BuildingDefinitionId)))
            .Where(pair => (pair.def.Recipe != null && pair.def.Recipe.Inputs.ContainsKey(id))
                        || (pair.def.MaintenanceCost != null && pair.def.MaintenanceCost.ContainsKey(id)))
            .ToList();
        var consumerHdr = new Label { Text = $"Consumers ({consumers.Count})" };
        consumerHdr.AddThemeFontSizeOverride("font_size", 12);
        consumerHdr.Modulate = new Color(1f, 0.65f, 0.55f);
        _resourceDetailsBody.AddChild(consumerHdr);
        if (consumers.Count == 0)
            AddLine("  (none)", new Color(0.6f, 0.6f, 0.6f), 10);
        else
            foreach (var (slot, bdef) in consumers)
            {
                var parts = new List<string>();
                if (bdef.Recipe?.Inputs.TryGetValue(id, out var ri) == true)
                    parts.Add($"{ri:F1}/cycle");
                if (bdef.MaintenanceCost?.TryGetValue(id, out var mi) == true)
                    parts.Add($"{mi:F2}/sol maint");
                AddLine($"  • {bdef.DisplayName}: {string.Join(", ", parts)}");
            }

        // Population needs for food/water/oxygen.
        if (id is "nutrients" or "food" or "water" or "oxygen")
        {
            float needsPerSol = id switch
            {
                "nutrients" or "food" => s.Population.Count * s.Population.Needs.FoodPerSol,
                "water"               => s.Population.Count * s.Population.Needs.WaterPerSol,
                "oxygen"              => s.Population.Count * s.Population.Needs.OxygenPerSol,
                _                     => 0f,
            };
            if (needsPerSol > 0f)
                AddLine($"  • Colonist needs: {needsPerSol:F1}/sol", new Color(0.95f, 0.85f, 0.55f));
        }
    }

    private void BuildBuildingsPanel()
    {
        var panel = new PanelContainer
        {
            AnchorLeft = 0, AnchorTop = 0,
            OffsetLeft = 8, OffsetTop = 56,
            Visible = false,
            MouseFilter = Control.MouseFilterEnum.Stop,
        };
        var vbox = new VBoxContainer { CustomMinimumSize = new Vector2(440, 0) };
        vbox.AddThemeConstantOverride("separation", 4);
        panel.AddChild(vbox);

        var headerRow = new HBoxContainer();
        var headerLbl = new Label
        {
            Text = "Buildings",
            SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
        };
        headerLbl.AddThemeFontSizeOverride("font_size", 14);
        headerRow.AddChild(headerLbl);

        var closeBtn = new Button { Text = "✕", Flat = true };
        closeBtn.Pressed += () => panel.Visible = false;
        headerRow.AddChild(closeBtn);
        vbox.AddChild(headerRow);

        vbox.AddChild(new HSeparator());

        _buildingsPanelSummary = new Label { AutowrapMode = TextServer.AutowrapMode.WordSmart };
        _buildingsPanelSummary.AddThemeFontSizeOverride("font_size", 12);
        vbox.AddChild(_buildingsPanelSummary);

        vbox.AddChild(new HSeparator());

        _underConstructionHeader = new Label { Text = "Under Construction", Visible = false };
        _underConstructionHeader.AddThemeFontSizeOverride("font_size", 12);
        _underConstructionHeader.Modulate = new Color(0.95f, 0.75f, 0.35f);
        vbox.AddChild(_underConstructionHeader);

        _underConstructionContainer = new VBoxContainer { Visible = false };
        vbox.AddChild(_underConstructionContainer);

        _damagedContainer = new VBoxContainer();
        vbox.AddChild(_damagedContainer);

        _buildingsPanel = panel;
        AddChild(panel);
    }

    private void BuildRightPanel()
    {
        // Right-side event log panel — unchanged from before, just dropped under
        // the top bar.
        var rightPanel = new PanelContainer
        {
            AnchorLeft = 1, AnchorRight = 1,
            AnchorTop = 0, AnchorBottom = 0,
            OffsetLeft = -340, OffsetRight = -8,
            OffsetTop = 96, OffsetBottom = 12,
            CustomMinimumSize = new Vector2(0, 320),
        };
        var rightVbox = new VBoxContainer();
        rightPanel.AddChild(rightVbox);

        var eventHeaderRow = new HBoxContainer();
        rightVbox.AddChild(eventHeaderRow);

        var eventHeader = new Label
        {
            Text = "Event Log",
            SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
        };
        eventHeader.AddThemeFontSizeOverride("font_size", 14);
        eventHeaderRow.AddChild(eventHeader);

        _criticalBadgeLabel = new Label
        {
            Text = "⚠ 0",
            Visible = false,
            Modulate = new Color(1f, 0.35f, 0.35f),
        };
        _criticalBadgeLabel.AddThemeFontSizeOverride("font_size", 14);
        eventHeaderRow.AddChild(_criticalBadgeLabel);

        rightVbox.AddChild(new HSeparator());

        _eventLogContainer = new VBoxContainer();
        rightVbox.AddChild(_eventLogContainer);

        AddChild(rightPanel);
    }

    private void BuildBottomBuildBar()
    {
        var bottom = new PanelContainer
        {
            AnchorLeft = 0, AnchorRight = 1,
            AnchorTop = 1, AnchorBottom = 1,
            OffsetLeft = 8, OffsetRight = -8,
            OffsetTop = -120, OffsetBottom = -8,
        };
        var bottomVbox = new VBoxContainer
        {
            SizeFlagsVertical = Control.SizeFlags.ShrinkEnd,
        };
        bottomVbox.AddThemeConstantOverride("separation", 4);
        bottom.AddChild(bottomVbox);

        // Tile popup (drops up above the category bar when a category is selected).
        _buildTilePopup = new PanelContainer { Visible = false };
        _buildTileRow = new HBoxContainer();
        _buildTileRow.AddThemeConstantOverride("separation", 6);
        _buildTilePopup.AddChild(_buildTileRow);
        bottomVbox.AddChild(_buildTilePopup);

        // Category row — one toggle button per BuildingCategory.
        _buildCategoryRow = new HBoxContainer();
        _buildCategoryRow.AddThemeConstantOverride("separation", 6);
        bottomVbox.AddChild(_buildCategoryRow);

        foreach (var cat in Enum.GetValues<BuildingCategory>()
                                .OrderBy(CategoryOrder))
        {
            var btn = new Button
            {
                Text = "Build " + CategoryName(cat),
                ToggleMode = true,
            };
            btn.AddThemeFontSizeOverride("font_size", 13);
            BuildingCategory captured = cat;
            btn.Pressed += () => SetOpenBuildCategory(captured);
            _buildCategoryRow.AddChild(btn);
        }

        _bottomBuildBar = bottom;
        AddChild(bottom);
    }

    private void SetOpenBuildCategory(BuildingCategory? cat)
    {
        // Clicking the currently-open category closes it (toggle).
        if (_openBuildCategory == cat) cat = null;
        _openBuildCategory = cat;

        // Sync the toggle state of the category buttons.
        int i = 0;
        foreach (var c in Enum.GetValues<BuildingCategory>().OrderBy(CategoryOrder))
        {
            if (_buildCategoryRow.GetChildOrNull<Button>(i) is { } btn)
                btn.ButtonPressed = (c == cat);
            i++;
        }

        // Rebuild the tile popup contents.
        foreach (Node child in _buildTileRow.GetChildren()) child.QueueFree();
        _buildButtons.Clear();

        if (cat == null || _session == null)
        {
            _buildTilePopup.Visible = false;
            return;
        }

        var matches = _session.BuildableBuildings
            .Where(d => d.Category == cat.Value)
            .ToList();

        foreach (var def in matches)
            _buildTileRow.AddChild(MakeBuildTile(def));

        _buildTilePopup.Visible = matches.Count > 0;
        // Apply current affordability state to the freshly-built tiles.
        if (_session != null) RefreshBuildButtons(_session.State);
    }

    private Control MakeBuildTile(BuildingDefinition def)
    {
        string costSummary = string.Join(", ",
            def.ConstructionCost.Select(kv => $"{kv.Value} {kv.Key}"));
        var btn = new Button
        {
            Text = $"{def.DisplayName}\n{def.Size.Width}×{def.Size.Height}",
            ToggleMode = true,
            CustomMinimumSize = new Vector2(140, 56),
            TooltipText = $"{def.DisplayName} — {costSummary}\n{def.Description}",
        };
        btn.AddThemeFontSizeOverride("font_size", 10);
        string capturedId = def.Id;
        btn.Pressed += () =>
        {
            if (_placer == null) return;
            if (_placer.ActiveBuildingId == capturedId)
                _placer.SetActiveBuilding(null);
            else
                _placer.SetActiveBuilding(capturedId);
        };
        _buildButtons[def.Id] = btn;
        return btn;
    }

    private void BuildPopPanel()
    {
        var panel = new PanelContainer
        {
            AnchorLeft = 0, AnchorTop = 0,
            OffsetLeft = 8, OffsetTop = 56,
            Visible = false,
            MouseFilter = Control.MouseFilterEnum.Stop,
        };
        var vbox = new VBoxContainer { CustomMinimumSize = new Vector2(360, 0) };
        vbox.AddThemeConstantOverride("separation", 4);
        panel.AddChild(vbox);

        var headerRow = new HBoxContainer();
        var headerLbl = new Label
        {
            Text = "Population",
            SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
        };
        headerLbl.AddThemeFontSizeOverride("font_size", 14);
        headerRow.AddChild(headerLbl);

        var closeBtn = new Button { Text = "✕", Flat = true };
        closeBtn.Pressed += () => panel.Visible = false;
        headerRow.AddChild(closeBtn);
        vbox.AddChild(headerRow);

        vbox.AddChild(new HSeparator());

        // Reuse the existing health/morale/needs/skills builder.
        BuildPopulationPanel(vbox);

        _popPanel = panel;
        AddChild(panel);
    }

    private void BuildLaborPanel()
    {
        var panel = new PanelContainer
        {
            AnchorLeft = 0, AnchorTop = 0,
            OffsetLeft = 8, OffsetTop = 56,
            Visible = false,
            MouseFilter = Control.MouseFilterEnum.Stop,
        };
        var vbox = new VBoxContainer { CustomMinimumSize = new Vector2(440, 0) };
        vbox.AddThemeConstantOverride("separation", 4);
        panel.AddChild(vbox);

        var headerRow = new HBoxContainer();
        var headerLbl = new Label
        {
            Text = "Labor",
            SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
        };
        headerLbl.AddThemeFontSizeOverride("font_size", 14);
        headerRow.AddChild(headerLbl);

        var closeBtn = new Button { Text = "✕", Flat = true };
        closeBtn.Pressed += () => panel.Visible = false;
        headerRow.AddChild(closeBtn);
        vbox.AddChild(headerRow);

        vbox.AddChild(new HSeparator());

        _laborSummaryLabel = new Label
        {
            AutowrapMode = TextServer.AutowrapMode.WordSmart,
        };
        _laborSummaryLabel.AddThemeFontSizeOverride("font_size", 12);
        vbox.AddChild(_laborSummaryLabel);

        var autoBtn = new Button { Text = "Auto-Assign Labor" };
        autoBtn.Pressed += () => _session?.AutoAssignLabor();
        vbox.AddChild(autoBtn);

        vbox.AddChild(new HSeparator());

        var opsHdr = new Label { Text = "Construction Operators" };
        opsHdr.AddThemeFontSizeOverride("font_size", 12);
        opsHdr.Modulate = new Color(0.65f, 0.85f, 1f);
        vbox.AddChild(opsHdr);

        _operatorsSummaryLabel = new Label { AutowrapMode = TextServer.AutowrapMode.WordSmart };
        _operatorsSummaryLabel.AddThemeFontSizeOverride("font_size", 11);
        vbox.AddChild(_operatorsSummaryLabel);

        vbox.AddChild(new HSeparator());

        var listHdr = new Label { Text = "Assignments" };
        listHdr.AddThemeFontSizeOverride("font_size", 12);
        listHdr.Modulate = new Color(0.65f, 0.85f, 1f);
        vbox.AddChild(listHdr);

        _laborPerBuildingList = new VBoxContainer();
        vbox.AddChild(_laborPerBuildingList);

        _laborPanel = panel;
        AddChild(panel);
    }

    private void BuildBuildingDetailsPanel()
    {
        var panel = new PanelContainer
        {
            AnchorLeft = 0, AnchorTop = 0,
            OffsetLeft = 8, OffsetTop = 56,
            Visible = false,
            MouseFilter = Control.MouseFilterEnum.Stop,
        };
        var vbox = new VBoxContainer { CustomMinimumSize = new Vector2(440, 0) };
        vbox.AddThemeConstantOverride("separation", 4);
        panel.AddChild(vbox);

        var headerRow = new HBoxContainer();
        _buildingDetailsTitle = new Label
        {
            Text = "Building",
            SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
        };
        _buildingDetailsTitle.AddThemeFontSizeOverride("font_size", 14);
        headerRow.AddChild(_buildingDetailsTitle);

        var closeBtn = new Button { Text = "✕", Flat = true };
        closeBtn.Pressed += () => { _selectedSlotId = null; panel.Visible = false; };
        headerRow.AddChild(closeBtn);
        vbox.AddChild(headerRow);

        vbox.AddChild(new HSeparator());

        _buildingDetailsBody = new VBoxContainer();
        _buildingDetailsBody.AddThemeConstantOverride("separation", 4);
        vbox.AddChild(_buildingDetailsBody);

        _buildingDetailsPanel = panel;
        AddChild(panel);
    }

    private void BuildEscMenu()
    {
        _escMenuOverlay = new ColorRect
        {
            AnchorLeft = 0, AnchorRight = 1,
            AnchorTop  = 0, AnchorBottom = 1,
            Color = new Color(0f, 0f, 0f, 0.6f),
            Visible = false,
            MouseFilter = Control.MouseFilterEnum.Stop,
        };

        var center = new CenterContainer
        {
            AnchorLeft = 0, AnchorRight = 1,
            AnchorTop  = 0, AnchorBottom = 1,
        };
        _escMenuOverlay.AddChild(center);

        var panel = new PanelContainer { CustomMinimumSize = new Vector2(320, 0) };
        center.AddChild(panel);

        var vbox = new VBoxContainer();
        vbox.AddThemeConstantOverride("separation", 6);
        panel.AddChild(vbox);

        var title = new Label
        {
            Text = "Paused",
            HorizontalAlignment = HorizontalAlignment.Center,
        };
        title.AddThemeFontSizeOverride("font_size", 22);
        vbox.AddChild(title);
        vbox.AddChild(new HSeparator());

        AddEscButton(vbox, "▶ Resume",         () => _escMenuOverlay.Visible = false);
        AddEscButton(vbox, "💾 Save…",         () => { _escMenuOverlay.Visible = false; OpenSaveModal(loadMode: false); });
        AddEscButton(vbox, "📂 Load…",         () => { _escMenuOverlay.Visible = false; OpenSaveModal(loadMode: true); });
        AddEscButton(vbox, "⚡ Quicksave",      () => { _escMenuOverlay.Visible = false; QuickSave(); });

        var settingsBtn = new Button
        {
            Text = "⚙ Settings… (coming soon)",
            Disabled = true,
            TooltipText = "Settings panel is not yet implemented.",
        };
        vbox.AddChild(settingsBtn);

        vbox.AddChild(new HSeparator());
        AddEscButton(vbox, "↩ Exit to Main Menu", ExitToMainMenu);
        AddEscButton(vbox, "✕ Exit to Desktop",   ExitToDesktop);

        AddChild(_escMenuOverlay);
    }

    private static void AddEscButton(VBoxContainer parent, string label, Action onPressed)
    {
        var btn = new Button
        {
            Text = label,
            SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
        };
        btn.AddThemeFontSizeOverride("font_size", 14);
        btn.Pressed += onPressed;
        parent.AddChild(btn);
    }

    private void ExitToMainMenu()
    {
        // Autosave is the most-recent state — leave it in place.
        GetTree().ChangeSceneToFile("res://scenes/ui/MainMenu.tscn");
    }

    private void ExitToDesktop()
    {
        GetTree().Quit();
    }

    private void BuildSaveModal()
    {
        _saveModalOverlay = new ColorRect
        {
            AnchorLeft = 0, AnchorRight = 1,
            AnchorTop  = 0, AnchorBottom = 1,
            Color = new Color(0f, 0f, 0f, 0.7f),
            Visible = false,
            MouseFilter = Control.MouseFilterEnum.Stop,
        };

        var center = new CenterContainer
        {
            AnchorLeft = 0, AnchorRight = 1,
            AnchorTop  = 0, AnchorBottom = 1,
        };
        _saveModalOverlay.AddChild(center);

        var panel = new PanelContainer { CustomMinimumSize = new Vector2(560, 0) };
        center.AddChild(panel);

        var vbox = new VBoxContainer();
        vbox.AddThemeConstantOverride("separation", 8);
        panel.AddChild(vbox);

        _saveModalTitle = new Label
        {
            Text = "Save Game",
            HorizontalAlignment = HorizontalAlignment.Center,
        };
        _saveModalTitle.AddThemeFontSizeOverride("font_size", 22);
        vbox.AddChild(_saveModalTitle);

        vbox.AddChild(new HSeparator());

        _saveModalSlots = new VBoxContainer();
        _saveModalSlots.AddThemeConstantOverride("separation", 4);
        vbox.AddChild(_saveModalSlots);

        vbox.AddChild(new HSeparator());

        var closeRow = new HBoxContainer();
        var spacer = new Control { SizeFlagsHorizontal = Control.SizeFlags.ExpandFill };
        closeRow.AddChild(spacer);

        var closeBtn = new Button { Text = "Close" };
        closeBtn.Pressed += () => _saveModalOverlay.Visible = false;
        closeRow.AddChild(closeBtn);
        vbox.AddChild(closeRow);

        AddChild(_saveModalOverlay);
    }

    private void OpenSaveModal(bool loadMode)
    {
        _saveModalTitle.Text = loadMode ? "Load Game" : "Save Game";
        RebuildSaveModalSlots(loadMode);
        _saveModalOverlay.Visible = true;
    }

    private void RebuildSaveModalSlots(bool loadMode)
    {
        foreach (Node child in _saveModalSlots.GetChildren())
            child.QueueFree();

        var slots = SaveSlotManager.ListAllSlots();
        foreach (var slot in slots)
        {
            var row = new HBoxContainer { SizeFlagsHorizontal = Control.SizeFlags.ExpandFill };

            var info = new Label
            {
                Text = FormatSlotInfo(slot),
                AutowrapMode = TextServer.AutowrapMode.WordSmart,
                SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
            };
            info.AddThemeFontSizeOverride("font_size", 11);
            info.Modulate = slot.Exists ? new Color(1, 1, 1) : new Color(0.55f, 0.55f, 0.55f);
            row.AddChild(info);

            if (loadMode)
            {
                var loadBtn = new Button { Text = "Load", Disabled = !slot.Exists };
                var capturedId = slot.SlotId;
                loadBtn.Pressed += () => DoLoad(capturedId);
                row.AddChild(loadBtn);
            }
            else
            {
                // Autosave is read-only — players shouldn't overwrite it manually.
                bool writable = slot.SlotId != SaveSlotManager.AutosaveId;
                var saveBtn = new Button { Text = "Save", Disabled = !writable };
                var capturedId = slot.SlotId;
                saveBtn.Pressed += () => DoSave(capturedId);
                row.AddChild(saveBtn);
            }

            var delBtn = new Button { Text = "Delete", Disabled = !slot.Exists };
            var capturedDel = slot.SlotId;
            delBtn.Pressed += () => DoDelete(capturedDel, loadMode);
            row.AddChild(delBtn);

            _saveModalSlots.AddChild(row);
        }
    }

    private static string FormatSlotInfo(SaveSlotManager.SlotInfo slot)
    {
        if (!slot.Exists) return $"{slot.Label}: (empty)";
        string name = string.IsNullOrEmpty(slot.ColonyName) ? "(unnamed)" : slot.ColonyName;
        string ts = slot.LastModified == DateTime.MinValue
            ? ""
            : $"  •  {slot.LastModified:yyyy-MM-dd HH:mm}";
        return $"{slot.Label}: {name} — Sol {slot.Sol}{ts}";
    }

    private void DoSave(string slotId)
    {
        var scene = GetParent() as ColonyScene;
        if (scene?.SaveToSlot(slotId) == true)
            FlashStatus($"Saved to {slotId}.");
        else
            FlashStatus("Save failed!");
        RebuildSaveModalSlots(loadMode: false);
    }

    private void DoLoad(string slotId)
    {
        var scene = GetParent() as ColonyScene;
        if (scene?.LoadFromSlot(slotId) == true)
        {
            FlashStatus($"Loaded {slotId}.");
            _saveModalOverlay.Visible = false;
        }
        else
        {
            FlashStatus("Load failed.");
        }
    }

    private void DoDelete(string slotId, bool loadMode)
    {
        var scene = GetParent() as ColonyScene;
        if (scene?.DeleteSlot(slotId) == true)
            FlashStatus($"Deleted {slotId}.");
        else
            FlashStatus("Delete failed.");
        RebuildSaveModalSlots(loadMode);
    }

    private void QuickSave()
    {
        var scene = GetParent() as ColonyScene;
        if (scene?.QuickSave() == true)
            FlashStatus("Quicksaved.");
        else
            FlashStatus("Quicksave failed.");
    }

    private void BuildDecisionOverlay()
    {
        _decisionOverlay = new ColorRect
        {
            AnchorLeft = 0, AnchorRight = 1,
            AnchorTop  = 0, AnchorBottom = 1,
            Color = new Color(0f, 0f, 0f, 0.7f),
            Visible = false,
            MouseFilter = Control.MouseFilterEnum.Stop, // swallow clicks while open
        };

        var center = new CenterContainer
        {
            AnchorLeft = 0, AnchorRight = 1,
            AnchorTop  = 0, AnchorBottom = 1,
        };
        _decisionOverlay.AddChild(center);

        var panel = new PanelContainer { CustomMinimumSize = new Vector2(520, 0) };
        center.AddChild(panel);

        var vbox = new VBoxContainer();
        vbox.AddThemeConstantOverride("separation", 10);
        panel.AddChild(vbox);

        _decisionTitle = new Label
        {
            Text = "Decision",
            HorizontalAlignment = HorizontalAlignment.Center,
        };
        _decisionTitle.AddThemeFontSizeOverride("font_size", 22);
        _decisionTitle.Modulate = new Color(1f, 0.9f, 0.55f);
        vbox.AddChild(_decisionTitle);

        vbox.AddChild(new HSeparator());

        _decisionDesc = new Label
        {
            Text = "",
            HorizontalAlignment = HorizontalAlignment.Left,
            AutowrapMode = TextServer.AutowrapMode.WordSmart,
        };
        _decisionDesc.AddThemeFontSizeOverride("font_size", 13);
        vbox.AddChild(_decisionDesc);

        vbox.AddChild(new HSeparator());

        _decisionChoices = new VBoxContainer();
        _decisionChoices.AddThemeConstantOverride("separation", 6);
        vbox.AddChild(_decisionChoices);

        AddChild(_decisionOverlay);
    }

    private void BuildPopulationPanel(VBoxContainer parent)
    {
        _populationHeaderLabel = new Label { AutowrapMode = TextServer.AutowrapMode.WordSmart };
        _populationHeaderLabel.AddThemeFontSizeOverride("font_size", 14);
        parent.AddChild(_populationHeaderLabel);

        _healthBar = MakeStatRow(parent, "Health", out _healthValueLabel);
        _moraleBar = MakeStatRow(parent, "Morale", out _moraleValueLabel);

        var needsHeader = new Label { Text = "Needs" };
        needsHeader.AddThemeFontSizeOverride("font_size", 11);
        needsHeader.Modulate = new Color(0.6f, 0.78f, 0.95f);
        parent.AddChild(needsHeader);

        _needsBars["food"]    = MakeNeedRow(parent, "Food",    out var foodLbl);    _needsValueLabels["food"]    = foodLbl;
        _needsBars["water"]   = MakeNeedRow(parent, "Water",   out var waterLbl);   _needsValueLabels["water"]   = waterLbl;
        _needsBars["oxygen"]  = MakeNeedRow(parent, "Oxygen",  out var oxygenLbl);  _needsValueLabels["oxygen"]  = oxygenLbl;
        _needsBars["housing"] = MakeNeedRow(parent, "Housing", out var housingLbl); _needsValueLabels["housing"] = housingLbl;

        _skillsLabel = new Label { AutowrapMode = TextServer.AutowrapMode.WordSmart };
        _skillsLabel.AddThemeFontSizeOverride("font_size", 11);
        parent.AddChild(_skillsLabel);
    }

    private static ProgressBar MakeStatRow(VBoxContainer parent, string title, out Label valueLabel)
    {
        var row = new HBoxContainer { SizeFlagsHorizontal = Control.SizeFlags.ExpandFill };
        var title_ = new Label { Text = title, CustomMinimumSize = new Vector2(60, 0) };
        title_.AddThemeFontSizeOverride("font_size", 11);
        row.AddChild(title_);

        var bar = new ProgressBar
        {
            MinValue = 0, MaxValue = 100, Value = 100,
            ShowPercentage = false,
            SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
            CustomMinimumSize = new Vector2(0, 10),
        };
        row.AddChild(bar);

        valueLabel = new Label { Text = "100", CustomMinimumSize = new Vector2(36, 0) };
        valueLabel.AddThemeFontSizeOverride("font_size", 11);
        valueLabel.HorizontalAlignment = HorizontalAlignment.Right;
        row.AddChild(valueLabel);

        parent.AddChild(row);
        return bar;
    }

    private static ProgressBar MakeNeedRow(VBoxContainer parent, string title, out Label valueLabel)
    {
        var row = new HBoxContainer { SizeFlagsHorizontal = Control.SizeFlags.ExpandFill };
        var title_ = new Label { Text = title, CustomMinimumSize = new Vector2(60, 0) };
        title_.AddThemeFontSizeOverride("font_size", 10);
        title_.Modulate = new Color(0.75f, 0.85f, 0.95f);
        row.AddChild(title_);

        var bar = new ProgressBar
        {
            MinValue = 0, MaxValue = 100, Value = 100,
            ShowPercentage = false,
            SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
            CustomMinimumSize = new Vector2(0, 8),
        };
        row.AddChild(bar);

        valueLabel = new Label { Text = "100%", CustomMinimumSize = new Vector2(40, 0) };
        valueLabel.AddThemeFontSizeOverride("font_size", 10);
        valueLabel.HorizontalAlignment = HorizontalAlignment.Right;
        row.AddChild(valueLabel);

        parent.AddChild(row);
        return bar;
    }

    private static int CategoryOrder(BuildingCategory c) => c switch
    {
        BuildingCategory.Power       => 0,
        BuildingCategory.LifeSupport => 1,
        BuildingCategory.Habitat     => 2,
        BuildingCategory.Production  => 3,
        BuildingCategory.Storage     => 4,
        _                            => 99,
    };

    public override void _UnhandledInput(InputEvent @event)
    {
        if (@event is not InputEventKey { Pressed: true, Echo: false } k) return;

        if (k.CtrlPressed && k.Keycode == Key.S)
        {
            QuickSave();
            GetViewport().SetInputAsHandled();
            return;
        }

        if (k.Keycode == Key.Escape)
        {
            // BuildingPlacer also listens for Esc to clear an in-progress
            // placement — only handle Esc here when no placement is active.
            if (_placer != null && _placer.IsActive) return;

            // Decision and terminal overlays block input — Esc is a no-op there.
            if (_decisionOverlay.Visible || _terminalOverlay.Visible) return;

            // Close any side / modal panels first, in priority order.
            if (_saveModalOverlay.Visible)         { _saveModalOverlay.Visible = false;         GetViewport().SetInputAsHandled(); return; }
            if (_buildingDetailsPanel.Visible)     { _buildingDetailsPanel.Visible = false; _selectedSlotId = null; GetViewport().SetInputAsHandled(); return; }
            if (_buildingsPanel.Visible)           { _buildingsPanel.Visible = false;           GetViewport().SetInputAsHandled(); return; }
            if (_fleetPanel.Visible)               { _fleetPanel.Visible = false;               GetViewport().SetInputAsHandled(); return; }
            if (_resourceDetailsPanel.Visible)     { _resourceDetailsPanel.Visible = false; _selectedResourceId = null; GetViewport().SetInputAsHandled(); return; }
            if (_techPanel.Visible)                { _techPanel.Visible = false;                GetViewport().SetInputAsHandled(); return; }
            if (_laborPanel.Visible)               { _laborPanel.Visible = false;               GetViewport().SetInputAsHandled(); return; }
            if (_popPanel.Visible)                 { _popPanel.Visible = false;                 GetViewport().SetInputAsHandled(); return; }
            if (_openBuildCategory.HasValue)       { SetOpenBuildCategory(null);                GetViewport().SetInputAsHandled(); return; }

            // Toggle Esc menu.
            _escMenuOverlay.Visible = !_escMenuOverlay.Visible;
            GetViewport().SetInputAsHandled();
        }
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

        if (_criticalBadgeLabel != null && _criticalBadgeLabel.Visible)
        {
            double t = Time.GetTicksMsec() / 1000.0;
            float pulse = 0.65f + 0.35f * (float)Math.Abs(Math.Sin(t * 3));
            _criticalBadgeLabel.Modulate = new Color(1f, 0.35f, 0.35f, pulse);
        }
    }

    private void OnPlacerSelectionChanged(string? buildingId)
    {
        foreach (var kv in _buildButtons)
            kv.Value.ButtonPressed = kv.Key == buildingId;
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
        RefreshResourceFlow(s);
        RefreshPowerTile(s);

        int building = s.Grid.AllSlots.Count(b => b.State == BuildingState.UnderConstruction);
        int operational = s.Grid.AllSlots.Count(b => b.State == BuildingState.Operational);
        int damaged = s.Grid.AllSlots.Count(b => b.State == BuildingState.Damaged);

        RefreshPopulationPanel(s);
        RefreshBuildingsStatButton(operational, building, damaged);
        if (_buildingsPanel.Visible)
            RefreshBuildingsPanel(s, operational, building, damaged);

        _stormLabel.Visible = s.DustStormTurnsRemaining > 0;
        _stormLabel.Text = $"⚠ Dust storm — {s.DustStormTurnsRemaining} sol(s) remaining";

        RefreshBuildButtons(s);
        RefreshColonyStatus(s);
        RefreshDamagedPanel(s);
        RefreshUnderConstructionPanel(s);
        RefreshEventLog(s);
        RefreshDecisionOverlay(s);
        RefreshCriticalBadge(s);
        RefreshStatBars(s);
        RefreshLaborPanel(s);
        RefreshFleetPanel(s);
        if (_buildingDetailsPanel.Visible) RefreshBuildingDetailsPanel();
        if (_resourceDetailsPanel.Visible) RefreshResourceDetails();
    }

    private void RefreshStatBars(ColonyState s)
    {
        // Top-bar compact stats: pop count, workers, fleet, buildings.
        _popStatBtn.Text   = $"👥 {s.Population.Count}";
        _laborStatBtn.Text = $"⚒ {s.Labor.AllocatedWorkers}/{s.Labor.TotalWorkers}";
        _fleetStatBtn.Text = $"🚜 {s.Fleet.UsedSlots}/{s.Fleet.TotalSlots}";
        _fleetStatBtn.Modulate = s.Fleet.Available <= 0 && s.Fleet.TotalSlots > 0
            ? new Color(1f, 0.7f, 0.4f) // amber when fully booked
            : new Color(1f, 1f, 1f);
    }

    private void RefreshFleetPanel(ColonyState s)
    {
        if (!_fleetPanel.Visible) return;

        _fleetPanelSummary.Text =
            $"Total: {s.Fleet.TotalSlots} (base {s.Fleet.BaseSlots}"
            + (s.Fleet.FactorySlots > 0 ? $" + factory {s.Fleet.FactorySlots}" : "")
            + (s.Fleet.TechBonus    > 0 ? $" + tech {s.Fleet.TechBonus}"       : "")
            + $")\nUsed: {s.Fleet.UsedSlots}   Available: {s.Fleet.Available}";

        foreach (Node child in _fleetFactoryList.GetChildren()) child.QueueFree();

        var factories = s.Grid.AllSlots
            .Where(b => b.BuildingDefinitionId == "vehicle_factory"
                     && b.State == BuildingState.Operational)
            .ToList();
        if (factories.Count == 0)
        {
            var lbl = new Label { Text = "(no vehicle factories yet)" };
            lbl.AddThemeFontSizeOverride("font_size", 11);
            lbl.Modulate = new Color(0.6f, 0.6f, 0.6f);
            _fleetFactoryList.AddChild(lbl);
            return;
        }
        var def = BuildingRegistry.Get("vehicle_factory");
        int cycle = def.Recipe?.TurnsPerCycle ?? 60;
        foreach (var slot in factories)
        {
            int pct = (int)Math.Round(100.0 * slot.ProductionCycleProgress / Math.Max(1, cycle));
            var lbl = new Label { Text = $"Vehicle Factory  •  next vehicle: {pct}%  ({slot.ProductionCycleProgress}/{cycle})" };
            lbl.AddThemeFontSizeOverride("font_size", 11);
            _fleetFactoryList.AddChild(lbl);
        }
    }

    private void RefreshPowerTile(ColonyState s)
    {
        float gen = s.Power.TotalCapacity;
        float use = s.Power.TotalConsumption;
        float deficit = s.Power.Deficit;
        Color trend;
        string text;

        if (deficit > 0.01f)
        {
            trend = new Color(1f, 0.45f, 0.45f); // red — brownout
            text  = $"{gen:F0}/{use:F0} MW ⚠";
            _powerTile.TooltipText = $"Deficit: {deficit:F1} MW — non-essentials brown out.";
        }
        else if (gen > use + 0.01f)
        {
            trend = new Color(0.45f, 1f, 0.55f); // green — surplus
            text  = $"{gen:F0}/{use:F0} MW";
            _powerTile.TooltipText = $"Surplus: {gen - use:F1} MW available.";
        }
        else
        {
            trend = new Color(1f, 0.92f, 0.40f); // yellow — at capacity
            text  = $"{gen:F0}/{use:F0} MW";
            _powerTile.TooltipText = "Power: at capacity.";
        }
        _powerTileText.Text = text;
        _powerTileText.Modulate = trend;
    }

    private void RefreshBuildingsStatButton(int operational, int building, int damaged)
    {
        var sb = new System.Text.StringBuilder();
        sb.Append("🏗 ").Append(operational);
        if (building > 0) sb.Append(" +").Append(building);
        if (damaged > 0)  sb.Append(" ⚠").Append(damaged);
        _buildingsStatBtn.Text = sb.ToString();
        _buildingsStatBtn.TooltipText = $"Buildings: {operational} operational"
            + (building > 0 ? $", {building} under construction" : "")
            + (damaged > 0 ? $", {damaged} damaged" : "")
            + ". Click for details.";
    }

    private void RefreshBuildingsPanel(ColonyState s, int operational, int building, int damaged)
    {
        _buildingsPanelSummary.Text =
            $"{operational} operational  •  {building} under construction  •  {damaged} damaged";
    }

    private void RefreshLaborPanel(ColonyState s)
    {
        if (!_laborPanel.Visible) return; // skip layout work when hidden

        var labor = s.Labor;
        int idle = labor.IdleWorkers;
        string skillsTxt = FormatSkills(s.Population.Skills);

        _laborSummaryLabel.Text =
            $"Total: {labor.TotalWorkers}  •  Working: {labor.AllocatedWorkers}  •  Idle: {idle}\n"
            + skillsTxt;

        _operatorsSummaryLabel.Text =
            $"People: {s.Operators.People}  •  Robots: {s.Operators.Robots}  •  "
            + $"Allocated: {s.Operators.Allocated}  •  Available: {s.Operators.Available}";

        foreach (Node child in _laborPerBuildingList.GetChildren()) child.QueueFree();

        var ops = s.Grid.AllSlots
            .Where(b => b.State == BuildingState.Operational)
            .ToList();

        if (ops.Count == 0)
        {
            var lbl = new Label { Text = "(no operational buildings)" };
            lbl.AddThemeFontSizeOverride("font_size", 11);
            lbl.Modulate = new Color(0.6f, 0.6f, 0.6f);
            _laborPerBuildingList.AddChild(lbl);
            return;
        }

        foreach (var slot in ops)
        {
            var def = BuildingRegistry.Get(slot.BuildingDefinitionId);
            if (def.LaborRequired == 0) continue;
            int assigned = labor.AllAllocations.TryGetValue(slot.Id, out var a) ? a : 0;
            var skill = labor.GetSlotSkill(slot.Id);

            var row = new HBoxContainer();
            var lbl = new Label
            {
                Text = $"{def.DisplayName}  {assigned}/{def.LaborRequired}"
                       + (skill.HasValue ? $"  [{skill.Value}]" : ""),
                SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
            };
            lbl.AddThemeFontSizeOverride("font_size", 11);
            lbl.Modulate = assigned >= def.LaborRequired
                ? new Color(0.85f, 1f, 0.85f)
                : new Color(1f, 0.85f, 0.6f);
            row.AddChild(lbl);
            _laborPerBuildingList.AddChild(row);
        }
    }

    private void RefreshBuildingDetailsPanel()
    {
        if (_session == null || _selectedSlotId is not Guid id)
        {
            _buildingDetailsPanel.Visible = false;
            return;
        }

        var slot = _session.State.Grid.GetSlot(id);
        if (slot == null)
        {
            _selectedSlotId = null;
            _buildingDetailsPanel.Visible = false;
            return;
        }

        var def = BuildingRegistry.Get(slot.BuildingDefinitionId);
        _buildingDetailsTitle.Text = def.DisplayName;

        foreach (Node child in _buildingDetailsBody.GetChildren()) child.QueueFree();

        void AddLine(string text, Color? modulate = null, int size = 11)
        {
            var lbl = new Label { Text = text, AutowrapMode = TextServer.AutowrapMode.WordSmart };
            lbl.AddThemeFontSizeOverride("font_size", size);
            if (modulate.HasValue) lbl.Modulate = modulate.Value;
            _buildingDetailsBody.AddChild(lbl);
        }

        AddLine($"State: {slot.State}", StateTint(slot.State));

        if (slot.State == BuildingState.UnderConstruction && def.ConstructionTurns > 0)
        {
            int done = def.ConstructionTurns - slot.ConstructionTurnsRemaining;
            int pct = Math.Clamp((int)Math.Round(done * 100f / def.ConstructionTurns), 0, 100);
            AddLine($"Progress: {pct}%  ({done}/{def.ConstructionTurns} sols)");

            // Fleet / operator status — show "Waiting" badge when starved.
            bool hasAllocation = slot.AllocatedFleetSlots > 0 || def.RequiredFleetSlots == 0;
            if (hasAllocation)
            {
                AddLine($"Equipment: {slot.AllocatedFleetSlots} fleet  •  {slot.AllocatedOperators} operators",
                        new Color(0.6f, 1f, 0.7f));
            }
            else
            {
                bool needsFleet = _session.State.Fleet.Available < def.RequiredFleetSlots;
                bool needsOps   = _session.State.Operators.Available < def.RequiredOperators;
                string what = (needsFleet, needsOps) switch
                {
                    (true,  true)  => "no fleet or operators",
                    (true,  false) => "no fleet",
                    (false, true)  => "no operators",
                    _              => "pool",
                };
                AddLine($"⚠ Waiting: {what}  (needs {def.RequiredFleetSlots} fleet, {def.RequiredOperators} op)",
                        new Color(1f, 0.65f, 0.35f));
            }
        }
        else if (def.RequiredFleetSlots > 0)
        {
            AddLine($"Construction needs: {def.RequiredFleetSlots} fleet, {def.RequiredOperators} operators",
                    new Color(0.7f, 0.85f, 1f), 10);
        }

        if (def.LaborRequired > 0)
        {
            int assigned = _session.State.Labor.AllAllocations.TryGetValue(slot.Id, out var a) ? a : 0;
            var skill = _session.State.Labor.GetSlotSkill(slot.Id);
            AddLine($"Workers: {assigned}/{def.LaborRequired}"
                    + (skill.HasValue ? $"  [{skill.Value}]" : "")
                    + $"  •  Primary skill: {def.PrimarySkill}");

            // Per-building labor SpinBox — only when operational (assigning to
            // an under-construction or damaged slot has no in-sim meaning).
            if (slot.State == BuildingState.Operational)
            {
                var laborRow = new HBoxContainer();
                var label = new Label { Text = "Assign:" };
                label.AddThemeFontSizeOverride("font_size", 11);
                laborRow.AddChild(label);

                var spin = new SpinBox
                {
                    MinValue = 0,
                    MaxValue = def.LaborRequired,
                    Value    = assigned,
                    Step     = 1,
                    CustomMinimumSize = new Vector2(90, 0),
                };
                Guid capturedId = id;
                int capturedRequired = def.LaborRequired;
                spin.ValueChanged += (double newVal) =>
                {
                    int n = (int)Math.Clamp(newVal, 0, capturedRequired);
                    if (n == 0) _session?.DeallocateLabor(capturedId);
                    else        _session?.AssignLaborToBuilding(capturedId, n);
                };
                laborRow.AddChild(spin);

                var clearBtn = new Button { Text = "Clear" };
                clearBtn.AddThemeFontSizeOverride("font_size", 11);
                clearBtn.Pressed += () => _session?.DeallocateLabor(capturedId);
                laborRow.AddChild(clearBtn);

                _buildingDetailsBody.AddChild(laborRow);
            }
        }

        if (def.PowerProduction > 0)
            AddLine($"Power output: {def.PowerProduction:F1} MW",
                    new Color(0.7f, 1f, 0.85f));
        if (def.PowerConsumption > 0)
            AddLine($"Power draw: {def.PowerConsumption:F1} MW",
                    new Color(1f, 0.85f, 0.7f));

        if (def.Recipe != null)
        {
            string inputs = def.Recipe.Inputs.Count == 0
                ? "—"
                : string.Join(", ", def.Recipe.Inputs.Select(kv => $"{kv.Value} {kv.Key}"));
            string outputs = def.Recipe.Outputs.Count == 0
                ? "—"
                : string.Join(", ", def.Recipe.Outputs.Select(kv => $"{kv.Value} {kv.Key}"));
            AddLine($"Recipe: {inputs} → {outputs}   ({def.Recipe.TurnsPerCycle} sols/cycle)");
        }

        if (def.HousingCapacity > 0)
            AddLine($"Housing capacity: {def.HousingCapacity}");
        if (def.StorageCapacity > 0)
            AddLine($"Storage capacity: {def.StorageCapacity:F0}");

        // Action row
        var actions = new HBoxContainer();
        _buildingDetailsBody.AddChild(new HSeparator());
        _buildingDetailsBody.AddChild(actions);

        if (slot.State == BuildingState.UnderConstruction)
        {
            var cancelBtn = new Button { Text = "Cancel (50% refund)" };
            cancelBtn.Pressed += () =>
            {
                _session?.CancelConstruction(id);
                _selectedSlotId = null;
                _buildingDetailsPanel.Visible = false;
            };
            actions.AddChild(cancelBtn);
        }
        if (slot.State == BuildingState.Damaged)
        {
            var repairBtn = new Button { Text = "Repair (30% cost)" };
            repairBtn.Pressed += () =>
            {
                if (_session?.RepairBuilding(id) == false)
                    FlashStatus("Insufficient resources to repair!");
            };
            actions.AddChild(repairBtn);
        }
        if (slot.State == BuildingState.Operational && def.UpgradesTo != null)
        {
            var upgradeBtn = new Button { Text = $"Upgrade → {def.UpgradesTo}" };
            upgradeBtn.Pressed += () =>
            {
                var result = _session?.UpgradeBuilding(id);
                if (result is { Success: false })
                    FlashStatus(result.FailureReason ?? "Upgrade failed");
            };
            actions.AddChild(upgradeBtn);
        }
    }

    private static Color StateTint(BuildingState state) => state switch
    {
        BuildingState.Operational       => new Color(0.6f, 1f, 0.7f),
        BuildingState.UnderConstruction => new Color(1f, 0.85f, 0.4f),
        BuildingState.Damaged           => new Color(1f, 0.5f, 0.4f),
        BuildingState.Destroyed         => new Color(0.7f, 0.3f, 0.3f),
        BuildingState.Unpowered         => new Color(0.7f, 0.7f, 0.75f),
        _                               => new Color(1f, 1f, 1f),
    };

    private void RefreshDecisionOverlay(ColonyState s)
    {
        var d = s.CurrentDecision;
        if (d == null)
        {
            _decisionOverlay.Visible = false;
            return;
        }

        _decisionOverlay.Visible = true;
        _decisionTitle.Text = d.Title;
        _decisionDesc.Text  = d.Description;

        foreach (Node child in _decisionChoices.GetChildren())
            child.QueueFree();

        for (int i = 0; i < d.Choices.Count; i++)
        {
            int captured = i;
            var c = d.Choices[i];

            var btn = new Button
            {
                Text = c.Label,
                SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
            };
            btn.AddThemeFontSizeOverride("font_size", 14);
            btn.Pressed += () => _session?.AnswerDecision(captured);
            _decisionChoices.AddChild(btn);

            if (!string.IsNullOrEmpty(c.ConsequenceSummary))
            {
                var sub = new Label
                {
                    Text = c.ConsequenceSummary,
                    HorizontalAlignment = HorizontalAlignment.Center,
                    AutowrapMode = TextServer.AutowrapMode.WordSmart,
                };
                sub.AddThemeFontSizeOverride("font_size", 10);
                sub.Modulate = new Color(0.7f, 0.85f, 0.7f);
                _decisionChoices.AddChild(sub);
            }
        }
    }

    private void RefreshCriticalBadge(ColonyState s)
    {
        if (_session == null) return;
        int recent = s.EventLog.All.Count(
            e => e.Severity == ColonyEventSeverity.Critical
              && e.Sol >= _session.CurrentSol - 5);
        if (recent > 0)
        {
            _criticalBadgeLabel.Text = $"⚠ {recent}";
            _criticalBadgeLabel.Visible = true;
        }
        else
        {
            _criticalBadgeLabel.Visible = false;
        }
    }

    private void RefreshPopulationPanel(ColonyState s)
    {
        var pop = s.Population;
        var labor = s.Labor;
        int idle = labor.IdleWorkers;

        _populationHeaderLabel.Text =
            $"Population {pop.Count}  •  {labor.AllocatedWorkers}/{labor.TotalWorkers} working ({idle} idle)";

        _healthBar.Value = Math.Clamp(pop.Health, 0, 100);
        _moraleBar.Value = Math.Clamp(pop.Morale, 0, 100);
        _healthValueLabel.Text = $"{pop.Health:F0}";
        _moraleValueLabel.Text = $"{pop.Morale:F0}";
        _healthValueLabel.Modulate = BarColor(pop.Health);
        _moraleValueLabel.Modulate = BarColor(pop.Morale);

        var n = pop.LastNeeds;
        SetNeed("food",    n.Food);
        SetNeed("water",   n.Water);
        SetNeed("oxygen",  n.Oxygen);
        SetNeed("housing", n.Housing);

        _skillsLabel.Text = FormatSkills(pop.Skills);
    }

    private void SetNeed(string key, float met)
    {
        if (!_needsBars.TryGetValue(key, out var bar)) return;
        float pct = Math.Clamp(met * 100f, 0, 100);
        bar.Value = pct;
        var lbl = _needsValueLabels[key];
        lbl.Text = $"{pct:F0}%";
        lbl.Modulate = BarColor(pct);
    }

    private static Color BarColor(float pct) => pct switch
    {
        >= 80f => new Color(0.45f, 1f, 0.55f),
        >= 50f => new Color(0.95f, 0.9f, 0.4f),
        >= 25f => new Color(1f, 0.7f, 0.35f),
        _      => new Color(1f, 0.4f, 0.4f),
    };

    private static string FormatSkills(IReadOnlyDictionary<SkillType, int> skills)
    {
        if (skills.Count == 0) return "Skills: (none)";
        string Abbrev(SkillType s) => s switch
        {
            SkillType.Laborer   => "Lab",
            SkillType.Engineer  => "Eng",
            SkillType.Scientist => "Sci",
            SkillType.Farmer    => "Far",
            SkillType.Medic     => "Med",
            SkillType.Operator  => "Op",
            _                   => s.ToString().Substring(0, 3),
        };
        var parts = Enum.GetValues<SkillType>()
            .Select(sk => $"{Abbrev(sk)} {skills.GetValueOrDefault(sk, 0)}");
        return "Skills: " + string.Join(" • ", parts);
    }

    private void RefreshBuildButtons(ColonyState s)
    {
        if (_session == null) return;
        foreach (var def in _session.BuildableBuildings)
        {
            if (!_buildButtons.TryGetValue(def.Id, out var btn)) continue;
            var costFloat = def.ConstructionCost.ToDictionary(kv => kv.Key, kv => (float)kv.Value);
            bool affordable = s.Resources.HasEnough(costFloat);
            btn.Disabled = !affordable;
            btn.Modulate = affordable ? new Color(1, 1, 1) : new Color(0.55f, 0.55f, 0.55f);
            string tip = affordable
                ? def.Description
                : "Missing: " + string.Join(", ", costFloat
                    .Where(kv => s.Resources.Get(kv.Key) < kv.Value)
                    .Select(kv => $"{(int)(kv.Value - s.Resources.Get(kv.Key))} {kv.Key}"));
            btn.TooltipText = tip;
        }
    }

    private void RefreshUnderConstructionPanel(ColonyState s)
    {
        foreach (Node child in _underConstructionContainer.GetChildren())
            child.QueueFree();

        var building = s.Grid.AllSlots
            .Where(b => b.State == BuildingState.UnderConstruction)
            .ToList();

        _underConstructionHeader.Visible = building.Count > 0;
        _underConstructionContainer.Visible = building.Count > 0;
        if (building.Count == 0) return;

        foreach (var slot in building)
        {
            var def = BuildingRegistry.Get(slot.BuildingDefinitionId);
            int total = Math.Max(1, def.ConstructionTurns);
            int done  = total - slot.ConstructionTurnsRemaining;
            int pct   = Math.Clamp((int)Math.Round(done * 100f / total), 0, 100);

            var row = new HBoxContainer();
            var lbl = new Label
            {
                Text = $"{def.DisplayName}  {pct}% ({done}/{total})",
                SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
            };
            lbl.AddThemeFontSizeOverride("font_size", 11);
            lbl.Modulate = new Color(0.95f, 0.75f, 0.35f);

            var cancelBtn = new Button { Text = "Cancel" };
            cancelBtn.AddThemeFontSizeOverride("font_size", 10);
            var capturedId = slot.Id;
            cancelBtn.Pressed += () => _session?.CancelConstruction(capturedId);

            row.AddChild(lbl);
            row.AddChild(cancelBtn);
            _underConstructionContainer.AddChild(row);
        }
    }

    private void RefreshColonyStatus(ColonyState s)
    {
        bool terminal = s.Status is ColonyStatus.Destroyed or ColonyStatus.Abandoned;
        _terminalOverlay.Visible = terminal;

        (_colonyStatusLabel.Text, _colonyStatusLabel.Modulate) = s.Status switch
        {
            ColonyStatus.Thriving  => ("★ Colony Thriving", new Color(1f, 0.9f, 0.2f)),
            ColonyStatus.Critical  => ($"⚠ CRITICAL — morale collapse ({s.LowMoraleTurns} turns)",
                                       new Color(1f, 0.35f, 0.1f)),
            ColonyStatus.Abandoned => ("✕ Colony Abandoned", new Color(0.6f, 0.4f, 0.4f)),
            ColonyStatus.Destroyed => ("✕ Colony Destroyed", new Color(1f, 0.2f, 0.2f)),
            _ => ("", Colors.White),
        };
        _colonyStatusLabel.Visible = s.Status != ColonyStatus.Active;

        if (terminal)
        {
            // Update overlay message.
            var lbl = _terminalOverlay.GetNodeOrNull<Label>("Msg");
            if (lbl != null)
                lbl.Text = s.Status == ColonyStatus.Destroyed
                    ? "COLONY LOST\nAll colonists have perished."
                    : "COLONY ABANDONED\nMorale completely collapsed.";
        }
    }

    private void RefreshDamagedPanel(ColonyState s)
    {
        foreach (Node child in _damagedContainer.GetChildren())
            child.QueueFree();

        var damaged = s.Grid.AllSlots
            .Where(b => b.State == BuildingState.Damaged)
            .ToList();

        if (damaged.Count == 0) return;

        foreach (var slot in damaged)
        {
            var def = BuildingRegistry.Get(slot.BuildingDefinitionId);
            var repairCost = string.Join(", ",
                def.ConstructionCost.Select(kv => $"{(int)(kv.Value * 0.3f)} {kv.Key}"));

            var row = new HBoxContainer();
            var lbl = new Label
            {
                Text = $"⚠ {def.DisplayName}  (repair: {repairCost})",
                SizeFlagsHorizontal = Control.SizeFlags.ExpandFill,
            };
            lbl.AddThemeFontSizeOverride("font_size", 11);
            lbl.Modulate = new Color(1f, 0.85f, 0.3f);

            var repairBtn = new Button { Text = "Repair" };
            var capturedId = slot.Id;
            repairBtn.Pressed += () =>
            {
                if (_session?.RepairBuilding(capturedId) == false)
                    FlashStatus("Insufficient resources to repair!");
            };

            row.AddChild(lbl);
            row.AddChild(repairBtn);
            _damagedContainer.AddChild(row);
        }
    }

    private void RefreshEventLog(ColonyState s)
    {
        foreach (Node child in _eventLogContainer.GetChildren())
            child.QueueFree();

        var recent = s.EventLog.All
            .Reverse()
            .Take(10)
            .ToList();

        foreach (var evt in recent)
        {
            var lbl = new Label
            {
                Text = $"[Sol {evt.Sol}] {evt.Message}",
                AutowrapMode = TextServer.AutowrapMode.WordSmart,
            };
            lbl.AddThemeFontSizeOverride("font_size", 11);
            lbl.Modulate = evt.Severity switch
            {
                ColonyEventSeverity.Critical => new Color(1f, 0.35f, 0.35f),
                ColonyEventSeverity.Warning  => new Color(1f, 0.85f, 0.3f),
                _                            => new Color(0.75f, 0.95f, 0.75f),
            };
            _eventLogContainer.AddChild(lbl);
        }
    }

    private void RefreshResourceFlow(ColonyState s)
    {
        foreach (Node child in _resourceFlow.GetChildren())
            child.QueueFree();

        var amounts = s.Resources.Snapshot();
        var caps    = s.Resources.CapSnapshot();
        var deltas  = s.ResourceDeltas;

        var keys = amounts.Keys
            .Union(caps.Keys)
            .Where(k => amounts.GetValueOrDefault(k, 0f) > 0.01f || caps.ContainsKey(k))
            .OrderBy(k => k);

        bool any = false;
        foreach (var key in keys)
        {
            any = true;
            float amt = amounts.GetValueOrDefault(key, 0f);
            float cap = caps.TryGetValue(key, out var c) ? c : float.MaxValue;
            float delta = deltas.TryGetValue(key, out var d) ? d : 0f;
            _resourceFlow.AddChild(MakeResourceTile(key, amt, cap, delta));
        }

        if (!any)
        {
            var empty = new Label
            {
                Text = "Resources: (empty)",
                Modulate = new Color(0.6f, 0.6f, 0.6f),
            };
            empty.AddThemeFontSizeOverride("font_size", 11);
            _resourceFlow.AddChild(empty);
        }
    }

    private Control MakeResourceTile(string id, float amount, float cap, float delta)
    {
        // Single-line tile: NAME amount[/cap] [arrow±delta]
        // The value + delta text colour reflects the trend:
        //   green  = stockpile is rising
        //   red    = stockpile is falling
        //   yellow = neutral (no net change this sol)
        // The whole tile is a Button so clicking opens the resource details panel.
        bool capped = cap < float.MaxValue && cap > 0f;
        Color trend = TrendColor(delta);

        string txt = ShortResourceName(id) + "  "
                   + (capped ? $"{amount:F0}/{cap:F0}" : $"{amount:F0}");
        if (Math.Abs(delta) >= 0.05f)
            txt += "  " + (delta > 0f ? "▲" : "▼") + $"{Math.Abs(delta):F1}";

        var btn = new Button
        {
            Text = txt,
            Flat = true,
            Alignment = HorizontalAlignment.Left,
            TooltipText = $"{id} — click for production / consumption details",
            Modulate = trend,
        };
        btn.AddThemeFontSizeOverride("font_size", 11);
        string capturedId = id;
        btn.Pressed += () => OpenResourceDetails(capturedId);
        return btn;
    }

    private void OpenResourceDetails(string resourceId)
    {
        _selectedResourceId = resourceId;
        // Close other side panels.
        _popPanel.Visible = false;
        _laborPanel.Visible = false;
        _buildingsPanel.Visible = false;
        _fleetPanel.Visible = false;
        _buildingDetailsPanel.Visible = false;
        if (_techPanel != null) _techPanel.Visible = false;
        _resourceDetailsPanel.Visible = true;
        RefreshResourceDetails();
    }

    private static Color TrendColor(float delta)
    {
        if (delta >  0.05f) return new Color(0.45f, 1.00f, 0.55f); // green ↑
        if (delta < -0.05f) return new Color(1.00f, 0.45f, 0.45f); // red ↓
        return new Color(1.00f, 0.92f, 0.40f);                      // yellow steady
    }

    private static string ShortResourceName(string id)
    {
        // Short, human-readable label for the top-bar tile. Falls back to the id.
        return id switch
        {
            "iron_ore"             => "ore",
            "uranium_ore"          => "U-ore",
            "uranium_fuel"         => "fuel",
            "uranium_fuel_rod"     => "fuel-rod",
            "rare_earth_ore"       => "RE-ore",
            "rare_earth_metals"    => "RE",
            "silicon_ore"          => "Si-ore",
            "silicon_wafer"        => "Si",
            "carbon_compounds"     => "C-cmp",
            "carbon_fiber"         => "C-fib",
            "machine_parts"        => "parts",
            "medical_supplies"     => "meds",
            "structural_components"=> "struct",
            "construction_materials"=>"matrl",
            "research_data"        => "data",
            "electronics"          => "elec",
            "components"           => "comp",
            "nutrients"            => "nutr",
            _                       => id,
        };
    }

    private void BuildUi()
    {
        BuildTopBar();
        BuildRightPanel();
        BuildBottomBuildBar();
        BuildPopPanel();
        BuildLaborPanel();
        BuildBuildingsPanel();
        BuildFleetPanel();
        BuildResourceDetailsPanel();
        BuildTechPanel();
        BuildBuildingDetailsPanel();
        BuildDecisionOverlay();
        BuildSaveModal();
        BuildEscMenu();

        // Full-screen terminal overlay (hidden by default, shown on Destroyed/Abandoned).
        _terminalOverlay = new ColorRect
        {
            AnchorLeft = 0, AnchorRight = 1,
            AnchorTop = 0,  AnchorBottom = 1,
            Color = new Color(0f, 0f, 0f, 0.72f),
            Visible = false,
        };
        var overlayVbox = new VBoxContainer
        {
            AnchorLeft = 0.5f, AnchorRight = 0.5f,
            AnchorTop  = 0.5f, AnchorBottom = 0.5f,
            OffsetLeft = -200, OffsetRight = 200,
            OffsetTop  = -80,  OffsetBottom = 80,
        };
        var overlayMsg = new Label
        {
            Name = "Msg",
            HorizontalAlignment = HorizontalAlignment.Center,
            AutowrapMode = TextServer.AutowrapMode.WordSmart,
        };
        overlayMsg.AddThemeFontSizeOverride("font_size", 28);
        overlayMsg.Modulate = new Color(1f, 0.3f, 0.3f);
        overlayVbox.AddChild(overlayMsg);

        var restartNote = new Label
        {
            Text = "Close the scene to return to the main menu.",
            HorizontalAlignment = HorizontalAlignment.Center,
        };
        restartNote.AddThemeFontSizeOverride("font_size", 14);
        restartNote.Modulate = new Color(0.75f, 0.75f, 0.75f);
        overlayVbox.AddChild(restartNote);

        _terminalOverlay.AddChild(overlayVbox);
        AddChild(_terminalOverlay);
    }
}
