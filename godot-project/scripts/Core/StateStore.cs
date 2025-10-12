using Godot;
using Outpost3.Core.Domain;
using Outpost3.Core.Commands;
using Outpost3.Core.Systems;
using Outpost3.Core.Events;
using System;
using System.Linq;

namespace Outpost3.Core;

/// <summary>
/// Central state store that manages game state and event persistence.
/// Implements single-writer pattern with event sourcing.
/// </summary>
public partial class StateStore : Node
{
    private GameState _state = GameState.NewGame();
    private readonly IEventStore _eventStore;

    [Signal]
    public delegate void StateChangedEventHandler();

    public GameState State => _state;

    /// <summary>
    /// Creates a new StateStore with event persistence.
    /// </summary>
    /// <param name="eventStore">The event store for persisting events.</param>
    public StateStore(IEventStore eventStore)
    {
        _eventStore = eventStore ?? throw new ArgumentNullException(nameof(eventStore));
    }

    /// <summary>
    /// Default constructor for Godot scene instantiation.
    /// Note: EventStore must be injected separately when using this constructor.
    /// </summary>
    public StateStore()
    {
        // This constructor is needed for Godot scene instantiation
        // EventStore will need to be set via dependency injection or autoload
        _eventStore = null;
    }

    /// <summary>
    /// Sets the event store for this StateStore instance.
    /// Used when StateStore is instantiated via Godot scenes.
    /// </summary>
    /// <param name="eventStore">The event store to use for persistence.</param>
    public void SetEventStore(IEventStore eventStore)
    {
        if (_eventStore != null)
        {
            GD.PrintErr("Warning: EventStore already set. Ignoring new EventStore.");
            return;
        }
        
        if (eventStore == null)
        {
            throw new ArgumentNullException(nameof(eventStore));
        }
        
        // Use reflection to set the readonly field
        var field = typeof(StateStore).GetField("_eventStore", 
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        field?.SetValue(this, eventStore);
        
        GD.Print($"EventStore set. Current offset: {eventStore.CurrentOffset}, Count: {eventStore.Count}");
    }

    public override void _Ready()
    {
        GD.Print("StateStore initialized");
        
        if (_eventStore != null)
        {
            GD.Print($"EventStore connected. Current offset: {_eventStore.CurrentOffset}, Count: {_eventStore.Count}");
        }
        else
        {
            GD.PrintErr("Warning: StateStore initialized without EventStore. Events will not be persisted.");
        }
    }

    /// <summary>
    /// Applies a command to the current state, producing new state and events.
    /// Events are enriched with metadata and persisted to the event store.
    /// </summary>
    /// <param name="command">The command to apply.</param>
    public void ApplyCommand(Core.Commands.ICommand command)
    {
        GD.Print($"Applying command: {command.GetType().Name}");

        // Run reducer to get new state and events
        var (newState, events) = TimeSystem.Reduce(_state, command);

        // Only persist and emit if there are changes
        if (events.Count > 0 && _eventStore != null)
        {
            try
            {
                // Enrich events with metadata
                var enrichedEvents = events
                    .Cast<GameEvent>()
                    .Select(e => e with 
                    { 
                        GameTime = (float)newState.GameTime,
                        RealTime = DateTime.UtcNow
                        // Note: Offset is assigned by EventStore.Append
                    })
                    .ToArray();

                // Persist to event store
                var startingOffset = _eventStore.Append(enrichedEvents);
                
                GD.Print($"Persisted {enrichedEvents.Length} event(s) starting at offset {startingOffset}");
                
                foreach (var evt in enrichedEvents)
                {
                    GD.Print($"  Event: {evt.EventType} at game time {evt.GameTime:F2}");
                }
            }
            catch (EventStoreException ex)
            {
                GD.PrintErr($"Failed to persist events: {ex.Message}");
                // Continue even if persistence fails (for development/debugging)
            }
        }
        else if (events.Count > 0)
        {
            // Events generated but no event store - log warning
            GD.PrintErr($"Warning: {events.Count} events generated but EventStore is null. Events not persisted.");
            foreach (var evt in events)
            {
                GD.Print($"  Unpersisted Event: {evt.GetType().Name}");
            }
        }

        // Update current state
        _state = newState;

        // Notify observers
        EmitSignal(SignalName.StateChanged);
    }

    /// <summary>
    /// Loads a saved state, replacing the current state.
    /// Used during game load operations.
    /// </summary>
    public void LoadState(GameState loadedState)
    {
        _state = loadedState;
        GD.Print($"State loaded: GameTime = {_state.GameTime:F2}h");
        EmitSignal(SignalName.StateChanged);
    }
}