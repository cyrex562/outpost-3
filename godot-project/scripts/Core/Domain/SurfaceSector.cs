using System;
using System.Collections.Generic;

namespace Outpost3.Core.Domain;

/// <summary>
/// Represents a sector on a planetary surface.
/// Surface is divided into a grid of sectors for colony management.
/// </summary>
public record SurfaceSector
{
    public SectorCoordinates Coordinates { get; init; } = new(0, 0);
    public TerrainType Terrain { get; init; } = TerrainType.Plains;
    public int Elevation { get; init; } = 0; // Meters above sea level (or base level)

    /// <summary>
    /// Temperature in Celsius.
    /// </summary>
    public float Temperature { get; init; } = 20.0f;

    /// <summary>
    /// Whether this sector has been explored/scanned.
    /// </summary>
    public bool Explored { get; init; } = false;

    /// <summary>
    /// Resource deposits in this sector.
    /// </summary>
    public List<ResourceDeposit> ResourceDeposits { get; init; } = new();

    /// <summary>
    /// Hazards present in this sector.
    /// </summary>
    public List<SectorHazard> Hazards { get; init; } = new();

    /// <summary>
    /// Suitability score for colonization (0-100).
    /// Higher is better. Based on terrain, hazards, resources, etc.
    /// </summary>
    public int SuitabilityScore { get; init; } = 50;

    /// <summary>
    /// Structure built in this sector (null if none).
    /// </summary>
    public Ulid? StructureId { get; init; } = null;

    /// <summary>
    /// Whether there's a road built in this sector.
    /// </summary>
    public bool HasRoad { get; init; } = false;
}

/// <summary>
/// Surface map for a celestial body.
/// Contains all sectors and their properties.
/// </summary>
public record SurfaceMap
{
    public Ulid BodyId { get; init; }

    /// <summary>
    /// Map width in sectors.
    /// </summary>
    public int Width { get; init; } = 32;

    /// <summary>
    /// Map height in sectors.
    /// </summary>
    public int Height { get; init; } = 24;

    /// <summary>
    /// All sectors on this surface.
    /// Key: "{X},{Y}" coordinate string
    /// </summary>
    public Dictionary<string, SurfaceSector> Sectors { get; init; } = new();

    /// <summary>
    /// Get a sector at the given coordinates.
    /// </summary>
    public SurfaceSector? GetSector(int x, int y)
    {
        var key = $"{x},{y}";
        return Sectors.TryGetValue(key, out var sector) ? sector : null;
    }

    /// <summary>
    /// Get a sector at the given coordinates.
    /// </summary>
    public SurfaceSector? GetSector(SectorCoordinates coords)
    {
        return GetSector(coords.X, coords.Y);
    }

    /// <summary>
    /// Update a sector in the map.
    /// </summary>
    public SurfaceMap WithSectorUpdated(SurfaceSector sector)
    {
        var key = $"{sector.Coordinates.X},{sector.Coordinates.Y}";
        var newSectors = new Dictionary<string, SurfaceSector>(Sectors)
        {
            [key] = sector
        };
        return this with { Sectors = newSectors };
    }
}

/// <summary>
/// Terrain types for surface sectors.
/// </summary>
public enum TerrainType
{
    Plains,         // Flat, easy to build on
    Hills,          // Moderate elevation, decent buildability
    Mountains,      // High elevation, difficult to build
    Crater,         // Impact crater, varied resources
    Valley,         // Low elevation, sheltered
    Canyon,         // Deep valley, limited space
    Desert,         // Arid, flat terrain
    Polar,          // Ice-covered, cold
    Volcanic,       // Volcanic activity, hazardous
    Lava,           // Active lava flows
    Rocky,          // Rough, rocky terrain
    SaltFlat,       // Flat mineral deposits
    Dunes           // Sandy dunes
}

/// <summary>
/// Resource deposit in a sector.
/// </summary>
public record ResourceDeposit
{
    public Ulid Id { get; init; }
    public ResourceType ResourceType { get; init; } = ResourceType.Metal;

    /// <summary>
    /// Richness: how much resource is available (0-100).
    /// </summary>
    public int Richness { get; init; } = 50;

    /// <summary>
    /// Accessibility: how easy it is to extract (0-100).
    /// </summary>
    public int Accessibility { get; init; } = 50;

    /// <summary>
    /// Whether this deposit has been discovered.
    /// </summary>
    public bool Discovered { get; init; } = false;

    /// <summary>
    /// Amount remaining (decreases as mined).
    /// Null if infinite or not yet quantified.
    /// </summary>
    public double? AmountRemaining { get; init; } = null;
}

/// <summary>
/// Hazard in a sector.
/// </summary>
public record SectorHazard
{
    public HazardType Type { get; init; } = HazardType.Radiation;

    /// <summary>
    /// Severity (0-100, where 100 is extremely dangerous).
    /// </summary>
    public int Severity { get; init; } = 50;

    /// <summary>
    /// Whether this hazard is permanent or temporary.
    /// </summary>
    public bool Permanent { get; init; } = true;

    /// <summary>
    /// Time when the hazard will end (for temporary hazards).
    /// </summary>
    public double? EndTime { get; init; } = null;
}

/// <summary>
/// Types of hazards.
/// </summary>
public enum HazardType
{
    Radiation,      // Ionizing radiation
    ExtremeHeat,    // High temperatures
    ExtremeCold,    // Low temperatures
    Storms,         // Atmospheric storms
    Seismic,        // Earthquakes/moonquakes
    Volcanic,       // Volcanic eruptions
    MeteorShower,   // Meteor impacts
    ToxicAtmosphere,// Poisonous air
    LowGravity,     // Construction challenges
    HighGravity,    // Physical stress
    MagneticAnomaly,// Equipment interference
    Quicksand       // Unstable ground
}
