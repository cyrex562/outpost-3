using OutpostGame.Core.World;

namespace OutpostGame.Game;

/// <summary>
/// Static handoff point between scenes. MainMenu writes here; ColonyScene reads and clears.
/// </summary>
public static class GameContext
{
    private static SiteDefinition? _pendingSite;
    private static string? _pendingColonyName;

    /// <summary>Takes the pending site and clears it. Returns null if none was set.</summary>
    public static SiteDefinition? ConsumePendingSite()
    {
        var site = _pendingSite;
        _pendingSite = null;
        return site;
    }

    public static void SetPendingSite(SiteDefinition site, string? colonyName = null)
    {
        _pendingSite = site;
        _pendingColonyName = colonyName;
    }

    public static string? ConsumePendingColonyName()
    {
        var name = _pendingColonyName;
        _pendingColonyName = null;
        return name;
    }

    /// <summary>
    /// When non-null, ColonyScene._Ready will load the named slot immediately after
    /// binding. Cleared after consumption.
    /// </summary>
    public static string? LoadSlotOnReady { get; set; }

    public static bool HasAnySave => SaveSlotManager.ListAllSlots().Any(s => s.Exists);
}
