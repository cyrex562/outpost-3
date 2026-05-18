using OutpostGame.Core.Content;

namespace OutpostGame.Core.Colony;

/// <summary>
/// Tech tree definitions for the colony simulation. JSON-driven from
/// <see cref="EmbeddedContent.TechJson"/> (mirror at
/// <c>godot/content/tech.json</c>).
/// Simulation logic for research is future work; this registry is the
/// foundation it builds on.
/// </summary>
public static class TechRegistry
{
    private static readonly Dictionary<string, TechDefinition> _all = new();

    static TechRegistry()
    {
        LoadFrom(EmbeddedContent.TechJson);
    }

    public static void LoadFrom(string json)
    {
        var defs = ContentLoader.LoadTech(json);
        _all.Clear();
        foreach (var kv in defs) _all[kv.Key] = kv.Value;
    }

    public static TechDefinition Get(string id) =>
        _all.TryGetValue(id, out var def)
            ? def
            : throw new KeyNotFoundException($"Unknown tech: {id}");

    public static bool TryGet(string id, out TechDefinition? def) =>
        _all.TryGetValue(id, out def);

    public static IReadOnlyDictionary<string, TechDefinition> All => _all;
}
