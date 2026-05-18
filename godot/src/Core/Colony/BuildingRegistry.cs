using OutpostGame.Core.Content;

namespace OutpostGame.Core.Colony;

/// <summary>
/// Building definitions for the colony simulation. As of Phase 3.1 the
/// definitions are loaded from JSON via <see cref="ContentLoader"/>, with the
/// authoritative source living in <see cref="EmbeddedContent.BuildingsJson"/>
/// (Phase 3.2 will swap that for a runtime file loader reading
/// <c>content/buildings.json</c>).
/// </summary>
public static class BuildingRegistry
{
    private static readonly Dictionary<string, BuildingDefinition> _all = new();

    static BuildingRegistry()
    {
        LoadFrom(EmbeddedContent.BuildingsJson);
    }

    /// <summary>Replace the registry contents with definitions from the given JSON string.
    /// Intended for runtime content reloading (mods, hot reload).</summary>
    public static void LoadFrom(string json)
    {
        var defs = ContentLoader.LoadBuildings(json);
        _all.Clear();
        foreach (var kv in defs) _all[kv.Key] = kv.Value;
    }

    public static BuildingDefinition Get(string id) =>
        _all.TryGetValue(id, out var def)
            ? def
            : throw new KeyNotFoundException($"Unknown building: {id}");

    public static bool TryGet(string id, out BuildingDefinition? def) =>
        _all.TryGetValue(id, out def);

    public static IReadOnlyDictionary<string, BuildingDefinition> All => _all;
}
