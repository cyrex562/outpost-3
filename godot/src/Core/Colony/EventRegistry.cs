using OutpostGame.Core.Content;

namespace OutpostGame.Core.Colony;

/// <summary>
/// JSON-driven event definitions. The registry is data-only as of Phase 3.2 —
/// <see cref="OutpostGame.Core.Simulation.RandomEventProcessor"/> still drives
/// the simulation via its hardcoded handlers, but new events can be authored as
/// data in <c>godot/content/events.json</c> and read here. Future work: wire
/// the processor to consume registry entries directly.
/// </summary>
public static class EventRegistry
{
    private static readonly Dictionary<string, EventDefinition> _all = new();

    static EventRegistry()
    {
        LoadFrom(EmbeddedContent.EventsJson);
    }

    public static void LoadFrom(string json)
    {
        var defs = ContentLoader.LoadEvents(json);
        _all.Clear();
        foreach (var kv in defs) _all[kv.Key] = kv.Value;
    }

    public static EventDefinition Get(string id) =>
        _all.TryGetValue(id, out var def)
            ? def
            : throw new KeyNotFoundException($"Unknown event: {id}");

    public static bool TryGet(string id, out EventDefinition? def) =>
        _all.TryGetValue(id, out def);

    public static IReadOnlyDictionary<string, EventDefinition> All => _all;
}
