using Godot;
using Outpost3.Core;
using Outpost3.Core.Domain;
using Outpost3.Core.Events;
using Outpost3.Core.Persistence;
using Outpost3.Core.Services;

namespace Outpost3;

/// <summary>
/// Main application node for Outpost 3.
/// Handles initialization of core services and dependency injection.
/// </summary>
public partial class App : Node
{
    private IEventStore _eventStore = null!;
    private StateStore _stateStore = null!;
    private ISnapshotStore _snapshotStore = null!;
    private SaveLoadService _saveLoadService = null!;
    private Timer? _autoSaveTimer;

    public override void _Ready()
    {
        GD.Print("Outpost3 - started");
        GD.Print("Godot version: " + Engine.GetVersionInfo()["string"]);

        // Initialize core services
        InitializeServices();
        
        // Initialize debug panel if in debug mode
        if (OS.IsDebugBuild())
        {
            InitializeDebugPanel();
        }
    }

    /// <summary>
    /// Initializes core game services including EventStore and StateStore.
    /// </summary>
    private void InitializeServices()
    {
        GD.Print("App: Initializing services...");

        // Create event store with path in Godot's user directory
        var eventsPath = ProjectSettings.GlobalizePath("user://events.log");
        GD.Print($"App: Event store path: {eventsPath}");
        
        _eventStore = new FileEventStore(eventsPath);
        GD.Print($"App: EventStore initialized with {_eventStore.Count} existing events");

        // Create initial game state
        var initialState = GameState.NewGame();
        
        // Create state store with event store
        _stateStore = new StateStore(_eventStore);
        
        // Set the EventStore on StateStore (for late binding)
        _stateStore.SetEventStore(_eventStore);
        
        GD.Print("App: StateStore initialized");

        // Add StateStore as a child node so it can emit signals
        AddChild(_stateStore);
        _stateStore.Name = "StateStore";

        // Create snapshot store
        var savesPath = ProjectSettings.GlobalizePath("user://saves");
        _snapshotStore = new JsonSnapshotStore(savesPath);
        GD.Print($"App: Snapshot store initialized: {savesPath}");
        
        // Create save/load service
        _saveLoadService = new SaveLoadService(_stateStore, _eventStore, _snapshotStore);
        GD.Print("App: SaveLoadService initialized");
        
        // Add global input handler for quick save/load
        var inputHandler = new GlobalInputHandler();
        AddChild(inputHandler);
        inputHandler.Name = "GlobalInputHandler";
        GD.Print("App: GlobalInputHandler initialized");
        
        // Setup auto-save timer (every 5 minutes)
        _autoSaveTimer = new Timer();
        _autoSaveTimer.WaitTime = 300.0; // 5 minutes
        _autoSaveTimer.Autostart = true;
        _autoSaveTimer.Timeout += OnAutoSave;
        AddChild(_autoSaveTimer);
        _autoSaveTimer.Name = "AutoSaveTimer";
        GD.Print("App: Auto-save timer initialized (5 minute interval)");

        GD.Print("App: Services initialized successfully");
    }

    /// <summary>
    /// Initializes the debug event panel.
    /// </summary>
    private void InitializeDebugPanel()
    {
        GD.Print("App: Initializing debug panel...");

        var debugPanelScene = GD.Load<PackedScene>("res://scenes/UI/DebugEventPanel.tscn");
        if (debugPanelScene != null)
        {
            var debugPanel = debugPanelScene.Instantiate();
            AddChild(debugPanel);
            GD.Print("App: Debug panel initialized (Press F3 to toggle)");
        }
        else
        {
            GD.PrintErr("App: Could not load DebugEventPanel.tscn");
        }
    }

    /// <summary>
    /// Gets the EventStore instance for dependency injection.
    /// Called by UI presenters via CallDeferred.
    /// </summary>
    public IEventStore GetEventStore()
    {
        return _eventStore;
    }

    /// <summary>
    /// Gets the StateStore instance for dependency injection.
    /// Called by UI presenters via CallDeferred.
    /// </summary>
    public StateStore GetStateStore()
    {
        return _stateStore;
    }

    /// <summary>
    /// Gets the SaveLoadService instance for dependency injection.
    /// </summary>
    public SaveLoadService GetSaveLoadService()
    {
        return _saveLoadService;
    }

    /// <summary>
    /// Auto-save timer callback.
    /// </summary>
    private void OnAutoSave()
    {
        _saveLoadService.AutoSave();
        GD.Print("⏰ Auto-save triggered");
    }

    public override void _Process(double delta)
    {
    }
}
