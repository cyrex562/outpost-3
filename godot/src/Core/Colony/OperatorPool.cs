namespace OutpostGame.Core.Colony;

/// <summary>
/// Pool of construction-equipment operators — people and robots, fungible 1:1
/// against fleet slots. A building under construction holds operators for the
/// duration; without allocation, construction stalls.
///
/// People and robots are tracked separately so future tiers can differentiate
/// them (e.g. robots work in hazardous biomes, people supervise complex tasks).
/// For now <see cref="TryAllocate"/> is fungible — it just consumes from the
/// total.
/// </summary>
public sealed class OperatorPool
{
    public int People { get; set; } = 10;
    public int Robots { get; set; } = 5;
    public int Allocated { get; private set; }

    public int Total => People + Robots;
    public int Available => Math.Max(0, Total - Allocated);

    public bool TryAllocate(int n)
    {
        if (n <= 0) return true;
        if (Available < n) return false;
        Allocated += n;
        return true;
    }

    public void Release(int n)
    {
        if (n <= 0) return;
        Allocated = Math.Max(0, Allocated - n);
    }

    public void RestoreFromSave(int people, int robots, int allocated)
    {
        People    = Math.Max(0, people);
        Robots    = Math.Max(0, robots);
        Allocated = Math.Clamp(allocated, 0, Total);
    }
}
