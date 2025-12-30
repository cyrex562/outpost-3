using System;
using System.Collections.Generic;
using System.Linq;

namespace Outpost3.Core.Domain;

/// <summary>
/// Represents a colony established on a celestial body.
/// </summary>
public record Colony
{
    public Ulid Id { get; init; }
    public string Name { get; init; } = "";

    /// <summary>
    /// The system this colony is in.
    /// </summary>
    public Ulid SystemId { get; init; }

    /// <summary>
    /// The body this colony is on (planet, moon, asteroid).
    /// </summary>
    public Ulid BodyId { get; init; }

    /// <summary>
    /// The surface sector where the colony is located.
    /// </summary>
    public SectorCoordinates LandingSite { get; init; } = new(0, 0);

    /// <summary>
    /// Time when the colony was founded.
    /// </summary>
    public double FoundedTime { get; init; } = 0.0;

    /// <summary>
    /// Colonists in this colony.
    /// </summary>
    public List<Ulid> ColonistIds { get; init; } = new();

    /// <summary>
    /// Structures built in this colony.
    /// </summary>
    public List<Structure> Structures { get; init; } = new();

    /// <summary>
    /// Vehicles deployed in this colony.
    /// </summary>
    public List<Ulid> VehicleIds { get; init; } = new();

    /// <summary>
    /// Robots deployed in this colony.
    /// </summary>
    public List<Ulid> RobotIds { get; init; } = new();

    /// <summary>
    /// Resource stockpiles.
    /// </summary>
    public Dictionary<ResourceType, double> Resources { get; init; } = new();

    /// <summary>
    /// Power generation in kilowatts.
    /// </summary>
    public double PowerGeneration { get; init; } = 0.0;

    /// <summary>
    /// Power consumption in kilowatts.
    /// </summary>
    public double PowerConsumption { get; init; } = 0.0;

    /// <summary>
    /// Oxygen generation rate (units per day).
    /// </summary>
    public double OxygenGeneration { get; init; } = 0.0;

    /// <summary>
    /// Oxygen consumption rate (units per day).
    /// </summary>
    public double OxygenConsumption { get; init; } = 0.0;

    /// <summary>
    /// Overall colony morale (0-100).
    /// </summary>
    public int Morale { get; init; } = 75;

    /// <summary>
    /// Colony specialization type (if any).
    /// </summary>
    public ColonyType? Specialization { get; init; } = null;

    /// <summary>
    /// Get the power status: surplus, balanced, or deficit.
    /// </summary>
    public PowerStatus GetPowerStatus()
    {
        var diff = PowerGeneration - PowerConsumption;
        if (diff > 10) return PowerStatus.Surplus;
        if (diff < -10) return PowerStatus.Deficit;
        return PowerStatus.Balanced;
    }

    /// <summary>
    /// Get the oxygen status.
    /// </summary>
    public LifeSupportStatus GetOxygenStatus()
    {
        var diff = OxygenGeneration - OxygenConsumption;
        if (diff > 10) return LifeSupportStatus.Healthy;
        if (diff < 0) return LifeSupportStatus.Critical;
        return LifeSupportStatus.Stable;
    }
}

/// <summary>
/// Represents a structure/building in a colony.
/// </summary>
public record Structure
{
    public Ulid Id { get; init; }
    public string Name { get; init; } = "";
    public StructureType Type { get; init; } = StructureType.Habitat;

    /// <summary>
    /// Location on the surface grid.
    /// </summary>
    public SectorCoordinates Location { get; init; } = new(0, 0);

    /// <summary>
    /// Health/condition (0-100).
    /// </summary>
    public int Condition { get; init; } = 100;

    /// <summary>
    /// Power consumption in kilowatts.
    /// </summary>
    public double PowerConsumptionKW { get; init; } = 0.0;

    /// <summary>
    /// Power generation in kilowatts (for generators).
    /// </summary>
    public double PowerGenerationKW { get; init; } = 0.0;

    /// <summary>
    /// Oxygen generation (for life support).
    /// </summary>
    public double OxygenGeneration { get; init; } = 0.0;

    /// <summary>
    /// Capacity (colonists for habitat, storage for warehouse, etc.).
    /// </summary>
    public int Capacity { get; init; } = 0;

    /// <summary>
    /// Whether this structure is operational.
    /// </summary>
    public bool IsOperational { get; init; } = true;

    /// <summary>
    /// Colonists assigned to work in this structure.
    /// </summary>
    public List<Ulid> AssignedColonists { get; init; } = new();
}

/// <summary>
/// Sector coordinates on a planetary surface.
/// </summary>
public record SectorCoordinates(int X, int Y)
{
    public override string ToString() => $"({X}, {Y})";
}

/// <summary>
/// Types of structures.
/// </summary>
public enum StructureType
{
    // Basic infrastructure
    Habitat,            // Living quarters
    LifeSupport,        // Oxygen/temperature control
    PowerGenerator,     // Power production
    SolarPanel,         // Solar power
    NuclearReactor,     // Nuclear power
    Battery,            // Power storage

    // Resource production
    Mine,               // Resource extraction
    Refinery,           // Ore processing
    Factory,            // Manufacturing
    Farm,               // Food production
    Greenhouse,         // Advanced agriculture
    WaterExtractor,     // Water mining

    // Storage and logistics
    Warehouse,          // General storage
    FuelDepot,          // Fuel storage
    Garage,             // Vehicle storage/maintenance

    // Research and development
    ResearchLab,        // Science and research
    EngineeringBay,     // Engineering projects

    // Utility
    MedicalCenter,      // Healthcare
    RepairShop,         // Maintenance and repair
    CommunicationHub,   // Communications
    CommandCenter,      // Colony administration

    // Defense (future)
    Defense,            // Defense structures
    Shield              // Shield generators
}

/// <summary>
/// Colony specialization types.
/// </summary>
public enum ColonyType
{
    Mining,         // Bonus to resource extraction
    Research,       // Bonus to research
    Agricultural,   // Bonus to food production
    Industrial,     // Bonus to manufacturing
    Administrative  // Bonus to morale and efficiency
}

/// <summary>
/// Power status indicators.
/// </summary>
public enum PowerStatus
{
    Surplus,    // More power than needed
    Balanced,   // Power generation matches consumption
    Deficit     // Not enough power
}

/// <summary>
/// Life support status indicators.
/// </summary>
public enum LifeSupportStatus
{
    Healthy,    // Abundant life support
    Stable,     // Adequate life support
    Critical    // Insufficient life support
}

/// <summary>
/// Resource types for colony stockpiles.
/// </summary>
public enum ResourceType
{
    // Basic needs
    Food,
    Water,
    Oxygen,
    Power,

    // Raw materials
    RawOre,
    Metal,
    Polymer,
    Ceramic,

    // Processed goods
    Electronics,
    Components,
    Fuel,
    MedicalSupplies,

    // Rare resources
    RareEarths,
    Uranium
}
