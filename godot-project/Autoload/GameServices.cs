using Godot;
using Outpost3.Core;
using Outpost3.Core.Domain;
using Outpost3.Core.Events;
using Outpost3.Core.Persistence;
using Outpost3.Core.Services;

namespace Outpost3;

/// <summary>
/// Autoload singleton for Outpost 3 core services.
/// Handles initialization of EventStore, StateStore, and persistence services.
/// Persists across all scene changes.
/// </summary>
public partial class GameServices : Node
{
    private IEventStore _eventStore = null!;
    private StateStore _stateStore = null!;
    private ISnapshotStore _snapshotStore = null!;
    private SaveLoadService _saveLoadService = null!;
    private Timer? _autoSaveTimer;

    /// <summary>
    /// Gets the EventStore instance.
    /// </summary>
    public IEventStore EventStore => _eventStore;

    /// <summary>
    /// Gets the StateStore instance.
    /// </summary>
    public StateStore StateStore => _stateStore;

    /// <summary>
    /// Gets the SaveLoadService instance.
    /// </summary>
    public SaveLoadService SaveLoadService => _saveLoadService;

    public override void _Ready()
    {
        GD.Print("GameServices: Initializing...");
        GD.Print("Godot version: " + Engine.GetVersionInfo()["string"]);

        // Initialize core services
        InitializeServices();

        // Initialize debug panel if in debug mode
        if (OS.IsDebugBuild())
        {
            InitializeDebugPanel();
        }

        GD.Print("GameServices: Ready");
    }

    /// <summary>
    /// Initializes core game services including EventStore and StateStore.
    /// </summary>
    private void InitializeServices()
    {
        GD.Print("GameServices: Initializing core services...");

        // Create event store with path in Godot's user directory
        var eventsPath = ProjectSettings.GlobalizePath("user://events.log");
        GD.Print($"GameServices: Event store path: {eventsPath}");

        _eventStore = new FileEventStore(eventsPath);
        GD.Print($"GameServices: EventStore initialized with {_eventStore.Count} existing events");

        // Create state store with event store
        _stateStore = new StateStore(_eventStore);

        // Set the EventStore on StateStore (for late binding)
        _stateStore.SetEventStore(_eventStore);

        GD.Print("GameServices: StateStore initialized");

        // Add StateStore as a child node so it can emit signals
        AddChild(_stateStore);
        _stateStore.Name = "StateStore";

        // Create snapshot store
        var savesPath = ProjectSettings.GlobalizePath("user://saves");
        _snapshotStore = new JsonSnapshotStore(savesPath);
        GD.Print($"GameServices: Snapshot store initialized: {savesPath}");

        // Create save/load service
        _saveLoadService = new SaveLoadService(_stateStore, _eventStore, _snapshotStore);
        GD.Print("GameServices: SaveLoadService initialized");

        // Add global input handler for quick save/load
        var inputHandler = new GlobalInputHandler();
        AddChild(inputHandler);
        inputHandler.Name = "GlobalInputHandler";
        GD.Print("GameServices: GlobalInputHandler initialized");

        // Setup auto-save timer (every 5 minutes)
        _autoSaveTimer = new Timer();
        _autoSaveTimer.WaitTime = 300.0; // 5 minutes
        _autoSaveTimer.Autostart = true;
        _autoSaveTimer.Timeout += OnAutoSave;
        AddChild(_autoSaveTimer);
        _autoSaveTimer.Name = "AutoSaveTimer";
        GD.Print("GameServices: Auto-save timer initialized (5 minute interval)");

        GD.Print("GameServices: Core services initialized successfully");
    }

    /// <summary>
    /// Initializes the debug event panel.
    /// </summary>
    private void InitializeDebugPanel()
    {
        GD.Print("GameServices: Initializing debug panel...");

        var debugPanelScene = GD.Load<PackedScene>("res://Scenes/UI/DebugEventPanel.tscn");
        if (debugPanelScene != null)
        {
            var debugPanel = debugPanelScene.Instantiate();
            AddChild(debugPanel);
            GD.Print("GameServices: Debug panel initialized (Press F3 to toggle)");
        }
        else
        {
            GD.PrintErr("GameServices: Could not load DebugEventPanel.tscn");
        }
    }

    /// <summary>
    /// Initializes a new galaxy with the given seed and star count.
    /// Should be called when starting a new game.
    /// </summary>
    public void InitializeNewGalaxy(int seed = 42, int starCount = 100)
    {
        GD.Print($"GameServices: Initializing new galaxy (seed={seed}, stars={starCount})...");
        var initGalaxyCommand = new Outpost3.Core.Commands.InitializeGalaxy(Seed: seed, StarCount: starCount);
        _stateStore.ApplyCommand(initGalaxyCommand);
        GD.Print($"GameServices: Galaxy initialized with {starCount} stars");
    }

    /// <summary>
    /// Auto-save timer callback.
    /// </summary>
    private void OnAutoSave()
    {
        _saveLoadService.AutoSave();
        GD.Print("⏰ Auto-save triggered");
    }
}
