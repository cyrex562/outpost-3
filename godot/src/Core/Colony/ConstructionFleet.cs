namespace OutpostGame.Core.Colony;

/// <summary>
/// Pool of construction equipment slots. A building under construction holds
/// some number of slots (and matching operators) until it completes; without
/// allocation, construction makes no progress (hard stop, status surfaced in
/// the HUD).
///
/// Total capacity = <see cref="BaseSlots"/> + <see cref="FactorySlots"/>
/// (grown by operational <c>vehicle_factory</c> buildings) + <see cref="TechBonus"/>
/// (from research like <c>expanded_logistics</c>).
/// </summary>
public sealed class ConstructionFleet
{
    public int BaseSlots { get; set; } = 10;
    public int FactorySlots { get; set; }
    public int TechBonus { get; set; }

    public int TotalSlots => BaseSlots + FactorySlots + TechBonus;
    public int UsedSlots { get; private set; }
    public int Available => Math.Max(0, TotalSlots - UsedSlots);

    public bool TryAllocate(int n)
    {
        if (n <= 0) return true;
        if (Available < n) return false;
        UsedSlots += n;
        return true;
    }

    public void Release(int n)
    {
        if (n <= 0) return;
        UsedSlots = Math.Max(0, UsedSlots - n);
    }

    /// <summary>Reset to a known state. Used by save-load.</summary>
    public void RestoreFromSave(int baseSlots, int factorySlots, int techBonus, int usedSlots)
    {
        BaseSlots    = Math.Max(0, baseSlots);
        FactorySlots = Math.Max(0, factorySlots);
        TechBonus    = Math.Max(0, techBonus);
        UsedSlots    = Math.Clamp(usedSlots, 0, TotalSlots);
    }
}
