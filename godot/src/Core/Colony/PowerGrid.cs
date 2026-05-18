namespace OutpostGame.Core.Colony;

public sealed class PowerGrid
{
    private readonly HashSet<Guid> _essentialBuildings = new();
    private readonly HashSet<Guid> _manuallyDisabled = new();
    private readonly Dictionary<Guid, float> _consumers = new();
    private readonly Dictionary<Guid, float> _producers = new();
    private readonly Dictionary<Guid, float> _producerEfficiency = new();

    public float TotalCapacity => _producers.Sum(kv =>
        kv.Value * _producerEfficiency.GetValueOrDefault(kv.Key, 1f));
    public float TotalConsumption => _consumers
        .Where(kv => !_manuallyDisabled.Contains(kv.Key))
        .Sum(kv => kv.Value);
    public float Deficit => Math.Max(0f, TotalConsumption - TotalCapacity);
    public float BrownoutLevel => TotalCapacity > 0 ? Math.Clamp(Deficit / TotalConsumption, 0f, 1f) : 1f;

    public void RegisterProducer(Guid slotId, float output) => _producers[slotId] = output;
    public void RegisterConsumer(Guid slotId, float demand) => _consumers[slotId] = demand;

    /// <summary>Scales a producer's output (e.g. 0.2 during a dust storm).</summary>
    public void SetProducerEfficiency(Guid slotId, float efficiency) =>
        _producerEfficiency[slotId] = efficiency;

    public void ResetProducerEfficiency(Guid slotId) =>
        _producerEfficiency.Remove(slotId);
    public void SetEssential(Guid slotId, bool essential)
    {
        if (essential) _essentialBuildings.Add(slotId);
        else _essentialBuildings.Remove(slotId);
    }
    public void TogglePower(Guid slotId)
    {
        if (!_manuallyDisabled.Remove(slotId)) _manuallyDisabled.Add(slotId);
    }
    public void Unregister(Guid slotId)
    {
        _producers.Remove(slotId);
        _consumers.Remove(slotId);
        _producerEfficiency.Remove(slotId);
        _essentialBuildings.Remove(slotId);
        _manuallyDisabled.Remove(slotId);
    }

    /// <summary>Clears all registrations — used when restoring a save.</summary>
    public void UnregisterAll()
    {
        _producers.Clear();
        _consumers.Clear();
        _producerEfficiency.Clear();
        _essentialBuildings.Clear();
        _manuallyDisabled.Clear();
    }

    public bool IsPowered(Guid slotId)
    {
        if (_manuallyDisabled.Contains(slotId)) return false;
        if (!_consumers.ContainsKey(slotId)) return true;
        if (_essentialBuildings.Contains(slotId)) return true;
        return BrownoutLevel < 0.01f;
    }
}
