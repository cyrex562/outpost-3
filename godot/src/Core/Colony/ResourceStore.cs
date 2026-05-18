namespace OutpostGame.Core.Colony;

public sealed class ResourceStore
{
    private readonly Dictionary<string, float> _amounts = new();
    private readonly Dictionary<string, float> _caps = new();

    public float Get(string resourceId) => _amounts.GetValueOrDefault(resourceId, 0f);
    public float Cap(string resourceId) => _caps.GetValueOrDefault(resourceId, float.MaxValue);

    public bool HasEnough(string resourceId, float amount) => Get(resourceId) >= amount;

    public bool HasEnough(IReadOnlyDictionary<string, float> requirements) =>
        requirements.All(kv => HasEnough(kv.Key, kv.Value));

    public void Add(string resourceId, float amount)
    {
        _amounts[resourceId] = Math.Min(Get(resourceId) + amount, Cap(resourceId));
    }

    public bool TryConsume(string resourceId, float amount)
    {
        if (!HasEnough(resourceId, amount)) return false;
        _amounts[resourceId] = Get(resourceId) - amount;
        return true;
    }

    public bool TryConsume(IReadOnlyDictionary<string, float> requirements)
    {
        if (!HasEnough(requirements)) return false;
        foreach (var kv in requirements) TryConsume(kv.Key, kv.Value);
        return true;
    }

    public void SetCap(string resourceId, float cap) => _caps[resourceId] = cap;

    public IReadOnlyDictionary<string, float> Snapshot() => new Dictionary<string, float>(_amounts);
    public IReadOnlyDictionary<string, float> CapSnapshot() => new Dictionary<string, float>(_caps);

    public void RestoreFromSnapshot(
        IReadOnlyDictionary<string, float> amounts,
        IReadOnlyDictionary<string, float> caps)
    {
        _amounts.Clear(); _caps.Clear();
        foreach (var kv in caps)    _caps[kv.Key]    = kv.Value;
        // Restore amounts verbatim — the saved values are authoritative and may
        // legitimately exceed the cap (e.g. when no warehouse is built yet).
        foreach (var kv in amounts) _amounts[kv.Key] = kv.Value;
    }
}
