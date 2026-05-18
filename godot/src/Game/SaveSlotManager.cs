using System.Text.Json;
using System.Text.Json.Serialization;
using Godot;
using OutpostGame.Core.Simulation;

namespace OutpostGame.Game;

/// <summary>
/// Slot-based save persistence layer. Files live under <c>user://saves/</c>;
/// each slot maps to one JSON file. Five manual slots plus a dedicated
/// autosave + quicksave slot.
/// </summary>
public static class SaveSlotManager
{
    public const string AutosaveId  = "autosave";
    public const string QuicksaveId = "quicksave";

    public static readonly string[] ManualSlotIds = { "slot1", "slot2", "slot3", "slot4", "slot5" };

    private const string SaveDir = "user://saves";

    private static readonly JsonSerializerOptions _metaOpts = new()
    {
        Converters = { new JsonStringEnumConverter() },
        PropertyNameCaseInsensitive = true,
    };

    public sealed record SlotInfo(
        string SlotId,
        string Label,
        bool Exists,
        string? ColonyName,
        int Sol,
        DateTime LastModified
    );

    public static string Path(string slotId) => $"{SaveDir}/{slotId}.json";

    public static bool Exists(string slotId) => Godot.FileAccess.FileExists(Path(slotId));

    private static void EnsureDir()
    {
        DirAccess.MakeDirRecursiveAbsolute(ProjectSettings.GlobalizePath(SaveDir));
    }

    public static bool Save(string slotId, ColonySession session)
    {
        try
        {
            EnsureDir();
            using var file = Godot.FileAccess.Open(Path(slotId), Godot.FileAccess.ModeFlags.Write);
            if (file == null) return false;
            file.StoreString(session.CreateSave());
            return true;
        }
        catch
        {
            return false;
        }
    }

    public static bool Load(string slotId, ColonySession session)
    {
        try
        {
            if (!Exists(slotId)) return false;
            using var file = Godot.FileAccess.Open(Path(slotId), Godot.FileAccess.ModeFlags.Read);
            if (file == null) return false;
            session.ApplySave(file.GetAsText());
            return true;
        }
        catch
        {
            return false;
        }
    }

    public static bool Delete(string slotId)
    {
        if (!Exists(slotId)) return false;
        var err = DirAccess.RemoveAbsolute(ProjectSettings.GlobalizePath(Path(slotId)));
        return err == Error.Ok;
    }

    public static SlotInfo GetSlotInfo(string slotId, string label)
    {
        if (!Exists(slotId))
            return new SlotInfo(slotId, label, false, null, 0, DateTime.MinValue);

        string globalPath = ProjectSettings.GlobalizePath(Path(slotId));
        DateTime lastModified;
        try
        {
            lastModified = System.IO.File.GetLastWriteTime(globalPath);
        }
        catch
        {
            lastModified = DateTime.MinValue;
        }

        string? colonyName = null;
        int sol = 0;
        try
        {
            using var file = Godot.FileAccess.Open(Path(slotId), Godot.FileAccess.ModeFlags.Read);
            if (file != null)
            {
                var meta = JsonSerializer.Deserialize<SaveSlotMetadata>(file.GetAsText(), _metaOpts);
                if (meta != null)
                {
                    colonyName = meta.ColonyName;
                    sol        = meta.CurrentSol;
                }
            }
        }
        catch
        {
            // Treat unreadable saves as empty metadata — slot still listed as Exists.
        }

        return new SlotInfo(slotId, label, true, colonyName, sol, lastModified);
    }

    public static List<SlotInfo> ListAllSlots()
    {
        var list = new List<SlotInfo>();
        for (int i = 0; i < ManualSlotIds.Length; i++)
            list.Add(GetSlotInfo(ManualSlotIds[i], $"Slot {i + 1}"));
        list.Add(GetSlotInfo(QuicksaveId, "Quick Save"));
        list.Add(GetSlotInfo(AutosaveId,  "Autosave"));
        return list;
    }

    /// <summary>Returns the slot id with the most recent modified time, or null if none exist.</summary>
    public static string? FindMostRecentSlot()
    {
        return ListAllSlots()
            .Where(s => s.Exists)
            .OrderByDescending(s => s.LastModified)
            .FirstOrDefault()
            ?.SlotId;
    }

    private sealed class SaveSlotMetadata
    {
        public string ColonyName { get; set; } = "";
        public int CurrentSol { get; set; }
    }
}
