using Godot;
using OutpostGame.Core.Colony;
using OutpostGame.Core.World;
using OutpostGame.Game;

namespace OutpostGame.UI;

/// <summary>
/// Main menu and new-game setup panel, built entirely from code.
/// Flow: title screen → New Game opens overlay → Start Colony transitions to ColonyScene.
/// </summary>
public partial class MainMenuScene : Control
{
    private static readonly (BiomeType Biome, string Label, string Hint)[] Biomes =
    {
        (BiomeType.Barren,           "Barren",              "Standard desert rock. Balanced starting conditions."),
        (BiomeType.Rocky,            "Rocky",               "Rugged terrain. More impassable cells."),
        (BiomeType.Polar,            "Polar",               "Ice-covered. Rich in water, scarce in nutrients."),
        (BiomeType.Desert,           "Desert",              "Arid and flat. More open space, scarce water."),
        (BiomeType.Volcanic,         "Volcanic",            "Hazardous terrain. Uranium-rich, steel-scarce."),
        (BiomeType.MarginalHabitable,"Marginal Habitable",  "Best natural conditions. Easier early survival."),
    };

    private static readonly (DifficultyPreset Preset, string Label, string Hint)[] Difficulties =
    {
        (DifficultyPreset.Sandbox, "Sandbox",  "No events, no consumption. Build freely."),
        (DifficultyPreset.Easy,    "Easy",     "Generous resources, slow consumption, rare events."),
        (DifficultyPreset.Normal,  "Normal",   "Balanced challenge."),
        (DifficultyPreset.Hard,    "Hard",     "Scarce resources, fast consumption, frequent events."),
        (DifficultyPreset.Brutal,  "Brutal",   "Punishing. Events every 15 sols."),
    };

    private const string ColonyScenePath = "res://scenes/colony/ColonyScene.tscn";

    // New-game panel controls
    private LineEdit _colonyNameEdit = null!;
    private OptionButton _biomeSelector = null!;
    private OptionButton _difficultySelector = null!;
    private SpinBox _seedSpinBox = null!;
    private Label _hintLabel = null!;
    private Control _newGameOverlay = null!;
    private Button _continueBtn = null!;
    private Button _loadGameBtn = null!;
    private Control _loadGameOverlay = null!;
    private VBoxContainer _loadGameSlotList = null!;

    public override void _Ready()
    {
        // Load registries from res://content/ on the main-menu entry so that
        // every subsequent scene sees the same content set.
        ContentBootstrap.EnsureLoaded();
        AnchorRight = 1;
        AnchorBottom = 1;
        BuildUi();
    }

    private void BuildUi()
    {
        // ── Title screen ──────────────────────────────────────────────────────
        var bg = new ColorRect
        {
            AnchorRight = 1, AnchorBottom = 1,
            Color = new Color(0.06f, 0.07f, 0.09f),
        };
        AddChild(bg);

        var center = new CenterContainer { AnchorRight = 1, AnchorBottom = 1 };
        AddChild(center);

        var titleBox = new VBoxContainer { CustomMinimumSize = new Vector2(360, 0) };
        titleBox.AddThemeConstantOverride("separation", 16);
        center.AddChild(titleBox);

        var title = new Label { Text = "OUTPOST 3", HorizontalAlignment = HorizontalAlignment.Center };
        title.AddThemeFontSizeOverride("font_size", 48);
        title.Modulate = new Color(0.85f, 0.92f, 1f);
        titleBox.AddChild(title);

        var subtitle = new Label
        {
            Text = "Colony Survival & Grand Strategy",
            HorizontalAlignment = HorizontalAlignment.Center,
        };
        subtitle.AddThemeFontSizeOverride("font_size", 14);
        subtitle.Modulate = new Color(0.55f, 0.65f, 0.75f);
        titleBox.AddChild(subtitle);

        titleBox.AddChild(new HSeparator());

        var newGameBtn = new Button { Text = "New Colony" };
        newGameBtn.AddThemeFontSizeOverride("font_size", 18);
        newGameBtn.Pressed += ShowNewGamePanel;
        titleBox.AddChild(newGameBtn);

        _continueBtn = new Button { Text = "Continue" };
        _continueBtn.AddThemeFontSizeOverride("font_size", 18);
        _continueBtn.Disabled = !GameContext.HasAnySave;
        _continueBtn.Pressed += OnContinue;
        titleBox.AddChild(_continueBtn);

        _loadGameBtn = new Button { Text = "Load Game…" };
        _loadGameBtn.AddThemeFontSizeOverride("font_size", 16);
        _loadGameBtn.Disabled = !GameContext.HasAnySave;
        _loadGameBtn.Pressed += ShowLoadGamePanel;
        titleBox.AddChild(_loadGameBtn);

        // ── New-game overlay ──────────────────────────────────────────────────
        _newGameOverlay = new ColorRect
        {
            AnchorRight = 1, AnchorBottom = 1,
            Color = new Color(0f, 0f, 0f, 0.78f),
            Visible = false,
        };
        AddChild(_newGameOverlay);

        var panelCenter = new CenterContainer { AnchorRight = 1, AnchorBottom = 1 };
        _newGameOverlay.AddChild(panelCenter);

        var panel = new PanelContainer { CustomMinimumSize = new Vector2(440, 0) };
        panelCenter.AddChild(panel);

        var form = new VBoxContainer();
        form.AddThemeConstantOverride("separation", 10);
        panel.AddChild(form);

        var panelTitle = new Label { Text = "New Colony", HorizontalAlignment = HorizontalAlignment.Center };
        panelTitle.AddThemeFontSizeOverride("font_size", 22);
        form.AddChild(panelTitle);
        form.AddChild(new HSeparator());

        // Colony name
        form.AddChild(MakeLabel("Colony Name"));
        _colonyNameEdit = new LineEdit { Text = "Outpost Alpha", PlaceholderText = "Enter colony name…" };
        form.AddChild(_colonyNameEdit);

        // Difficulty
        form.AddChild(MakeLabel("Difficulty"));
        _difficultySelector = new OptionButton();
        for (int i = 0; i < Difficulties.Length; i++)
            _difficultySelector.AddItem(Difficulties[i].Label, i);
        _difficultySelector.Selected = 2; // Normal
        form.AddChild(_difficultySelector);

        // Biome
        form.AddChild(MakeLabel("Biome"));
        _biomeSelector = new OptionButton();
        for (int i = 0; i < Biomes.Length; i++)
            _biomeSelector.AddItem(Biomes[i].Label, i);
        _biomeSelector.Selected = 0; // Barren
        form.AddChild(_biomeSelector);

        // Hint label — updates when biome/difficulty selection changes
        _hintLabel = new Label
        {
            AutowrapMode = TextServer.AutowrapMode.WordSmart,
            CustomMinimumSize = new Vector2(0, 36),
        };
        _hintLabel.AddThemeFontSizeOverride("font_size", 11);
        _hintLabel.Modulate = new Color(0.6f, 0.75f, 0.6f);
        form.AddChild(_hintLabel);

        _biomeSelector.ItemSelected += _ => UpdateHint();
        _difficultySelector.ItemSelected += _ => UpdateHint();
        UpdateHint();

        // Seed
        form.AddChild(MakeLabel("World Seed"));
        var seedRow = new HBoxContainer();
        form.AddChild(seedRow);

        _seedSpinBox = new SpinBox
        {
            MinValue = 1, MaxValue = 999999,
            Value = new Random().Next(1, 999999),
            AllowGreater = false, AllowLesser = false,
            SizeFlagsHorizontal = SizeFlags.ExpandFill,
        };
        seedRow.AddChild(_seedSpinBox);

        var randomBtn = new Button { Text = "Random" };
        randomBtn.Pressed += () => _seedSpinBox.Value = new Random().Next(1, 999999);
        seedRow.AddChild(randomBtn);

        form.AddChild(new HSeparator());

        // Action buttons
        var actionRow = new HBoxContainer();
        form.AddChild(actionRow);

        var backBtn = new Button { Text = "Back", SizeFlagsHorizontal = SizeFlags.ExpandFill };
        backBtn.Pressed += () => _newGameOverlay.Visible = false;
        actionRow.AddChild(backBtn);

        var startBtn = new Button { Text = "Start Colony →", SizeFlagsHorizontal = SizeFlags.ExpandFill };
        startBtn.AddThemeFontSizeOverride("font_size", 15);
        startBtn.Pressed += OnStartColony;
        actionRow.AddChild(startBtn);

        BuildLoadGameOverlay();
    }

    private void BuildLoadGameOverlay()
    {
        _loadGameOverlay = new ColorRect
        {
            AnchorRight = 1, AnchorBottom = 1,
            Color = new Color(0f, 0f, 0f, 0.78f),
            Visible = false,
            MouseFilter = Control.MouseFilterEnum.Stop,
        };
        AddChild(_loadGameOverlay);

        var panelCenter = new CenterContainer { AnchorRight = 1, AnchorBottom = 1 };
        _loadGameOverlay.AddChild(panelCenter);

        var panel = new PanelContainer { CustomMinimumSize = new Vector2(520, 0) };
        panelCenter.AddChild(panel);

        var form = new VBoxContainer();
        form.AddThemeConstantOverride("separation", 8);
        panel.AddChild(form);

        var title = new Label { Text = "Load Game", HorizontalAlignment = HorizontalAlignment.Center };
        title.AddThemeFontSizeOverride("font_size", 22);
        form.AddChild(title);
        form.AddChild(new HSeparator());

        _loadGameSlotList = new VBoxContainer();
        _loadGameSlotList.AddThemeConstantOverride("separation", 4);
        form.AddChild(_loadGameSlotList);

        form.AddChild(new HSeparator());

        var closeRow = new HBoxContainer();
        var spacer = new Control { SizeFlagsHorizontal = SizeFlags.ExpandFill };
        closeRow.AddChild(spacer);
        var closeBtn = new Button { Text = "Back" };
        closeBtn.Pressed += () => _loadGameOverlay.Visible = false;
        closeRow.AddChild(closeBtn);
        form.AddChild(closeRow);
    }

    private void ShowNewGamePanel()
    {
        _newGameOverlay.Visible = true;
    }

    private void UpdateHint()
    {
        int bi = _biomeSelector.Selected;
        int di = _difficultySelector.Selected;
        string biomeHint = bi >= 0 && bi < Biomes.Length ? Biomes[bi].Hint : "";
        string diffHint  = di >= 0 && di < Difficulties.Length ? Difficulties[di].Hint : "";
        _hintLabel.Text = $"{biomeHint}  {diffHint}".Trim();
    }

    private void OnStartColony()
    {
        int bi = _biomeSelector.Selected;
        int di = _difficultySelector.Selected;
        if (bi < 0 || di < 0) return;

        string name = _colonyNameEdit.Text.Trim();
        if (string.IsNullOrEmpty(name)) name = "Outpost Alpha";

        GameContext.SetPendingSite(
            new SiteDefinition(
                Seed:       (int)_seedSpinBox.Value,
                Biome:      Biomes[bi].Biome,
                Size:       new GridSize(64, 64),
                Difficulty: Difficulties[di].Preset
            ),
            colonyName: name
        );

        GetTree().ChangeSceneToFile(ColonyScenePath);
    }

    private void OnContinue()
    {
        var slot = SaveSlotManager.FindMostRecentSlot();
        if (slot == null) return;
        GameContext.LoadSlotOnReady = slot;
        GetTree().ChangeSceneToFile(ColonyScenePath);
    }

    private void ShowLoadGamePanel()
    {
        RebuildLoadGameSlots();
        _loadGameOverlay.Visible = true;
    }

    private void RebuildLoadGameSlots()
    {
        foreach (Node child in _loadGameSlotList.GetChildren())
            child.QueueFree();

        foreach (var slot in SaveSlotManager.ListAllSlots())
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

            var loadBtn = new Button { Text = "Load", Disabled = !slot.Exists };
            var captured = slot.SlotId;
            loadBtn.Pressed += () =>
            {
                GameContext.LoadSlotOnReady = captured;
                GetTree().ChangeSceneToFile(ColonyScenePath);
            };
            row.AddChild(loadBtn);

            _loadGameSlotList.AddChild(row);
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

    private static Label MakeLabel(string text)
    {
        var lbl = new Label { Text = text };
        lbl.AddThemeFontSizeOverride("font_size", 12);
        lbl.Modulate = new Color(0.7f, 0.8f, 0.7f);
        return lbl;
    }
}
