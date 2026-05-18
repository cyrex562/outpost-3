using Godot;
using OutpostGame.Core.Colony;
using OutpostGame.Core.Content;

namespace OutpostGame.Game;

/// <summary>
/// Loads building and resource definitions from JSON files at startup,
/// overriding the registries' default (embedded) content. Looks at the
/// bundled <c>res://content/</c> first, then a per-user <c>user://content/</c>
/// override that mods can drop in to replace any individual file.
///
/// Failures fall back silently to the embedded content baked into
/// <see cref="EmbeddedContent"/> — the simulation always has *some* definitions
/// to operate on, even if a file is missing or malformed.
/// </summary>
public static class ContentBootstrap
{
    public const string BundledBuildingsPath = "res://content/buildings.json";
    public const string BundledResourcesPath = "res://content/resources.json";
    public const string BundledTechPath      = "res://content/tech.json";
    public const string BundledEventsPath    = "res://content/events.json";
    public const string UserBuildingsPath    = "user://content/buildings.json";
    public const string UserResourcesPath    = "user://content/resources.json";
    public const string UserTechPath         = "user://content/tech.json";
    public const string UserEventsPath       = "user://content/events.json";

    private static bool _loaded;

    /// <summary>
    /// Idempotent — safe to call from every scene's <c>_Ready</c>. Subsequent
    /// calls are no-ops once the registries are populated from disk.
    /// </summary>
    public static void EnsureLoaded()
    {
        if (_loaded) return;
        _loaded = true;
        LoadFromBundledFiles();
    }

    /// <summary>Forces a reload regardless of prior state — used by hot-reload.</summary>
    public static void LoadFromBundledFiles()
    {
        // Bundled assets ship inside the game; user-scope override (if any) wins.
        LoadInto(BundledBuildingsPath, UserBuildingsPath, BuildingRegistry.LoadFrom, "buildings");
        LoadInto(BundledResourcesPath, UserResourcesPath, ResourceRegistry.LoadFrom, "resources");
        LoadInto(BundledTechPath,      UserTechPath,      TechRegistry.LoadFrom,      "tech");
        LoadInto(BundledEventsPath,    UserEventsPath,    EventRegistry.LoadFrom,     "events");
    }

    private static void LoadInto(
        string bundledPath,
        string userPath,
        Action<string> apply,
        string label)
    {
        string? source = TryReadFile(userPath) ?? TryReadFile(bundledPath);
        // No file found — registry already has the embedded defaults from
        // its static constructor; nothing to do.
        if (source == null) return;

        try
        {
            apply(source);
        }
        catch (ContentLoader.ContentValidationException ex)
        {
            // `LoadFrom` validates before mutating the registry, so the previous
            // contents (embedded defaults) remain intact when the parse fails.
            GD.PrintErr($"ContentBootstrap: invalid {label} JSON — {ex.Message}. " +
                        "Embedded defaults are still in effect.");
        }
    }

    private static string? TryReadFile(string godotPath)
    {
        if (!Godot.FileAccess.FileExists(godotPath)) return null;
        try
        {
            using var file = Godot.FileAccess.Open(godotPath, Godot.FileAccess.ModeFlags.Read);
            return file?.GetAsText();
        }
        catch
        {
            return null;
        }
    }
}
