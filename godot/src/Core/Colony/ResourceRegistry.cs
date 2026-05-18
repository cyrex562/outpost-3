using OutpostGame.Core.Content;

namespace OutpostGame.Core.Colony;

/// <summary>
/// Resource definitions for the colony simulation. As of Phase 3.1 the
/// definitions are loaded from JSON via <see cref="ContentLoader"/>, with the
/// authoritative source living in <see cref="EmbeddedContent.ResourcesJson"/>
/// (Phase 3.2 will swap that for a runtime file loader reading
/// <c>content/resources.json</c>).
/// </summary>
public static class ResourceRegistry
{
    private static readonly Dictionary<string, ResourceDefinition> _all = new();

    static ResourceRegistry()
    {
        LoadFrom(EmbeddedContent.ResourcesJson);
    }

    /// <summary>Replace the registry contents with definitions from the given JSON string.
    /// Intended for runtime content reloading (mods, hot reload).</summary>
    public static void LoadFrom(string json)
    {
        var defs = ContentLoader.LoadResources(json);
        _all.Clear();
        foreach (var kv in defs) _all[kv.Key] = kv.Value;
    }

    public static ResourceDefinition Get(string id) =>
        _all.TryGetValue(id, out var def)
            ? def
            : throw new KeyNotFoundException($"Unknown resource: {id}");

    public static bool TryGet(string id, out ResourceDefinition? def) =>
        _all.TryGetValue(id, out def);

    public static IReadOnlyDictionary<string, ResourceDefinition> All => _all;
}
