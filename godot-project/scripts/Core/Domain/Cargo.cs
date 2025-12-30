using System;

namespace Outpost3.Core.Domain;

/// <summary>
/// Represents a cargo item stored on a ship or in a colony warehouse.
/// </summary>
public record CargoItem
{
    public Ulid Id { get; init; }
    public string Name { get; init; } = "";
    public CargoType Type { get; init; } = CargoType.Supplies;
    public int Quantity { get; init; } = 1;

    /// <summary>
    /// Mass per unit in kilograms.
    /// </summary>
    public double MassKg { get; init; } = 1.0;

    /// <summary>
    /// Volume per unit in cubic meters.
    /// </summary>
    public double VolumeM3 { get; init; } = 0.001;

    /// <summary>
    /// Optional description of the cargo.
    /// </summary>
    public string Description { get; init; } = "";

    /// <summary>
    /// Get total mass for this cargo stack.
    /// </summary>
    public double GetTotalMass() => MassKg * Quantity;

    /// <summary>
    /// Get total volume for this cargo stack.
    /// </summary>
    public double GetTotalVolume() => VolumeM3 * Quantity;
}

/// <summary>
/// Types of cargo items.
/// </summary>
public enum CargoType
{
    // Basic supplies
    Supplies,           // General supplies (food, water, medical)
    FoodRations,        // Preserved food
    Water,              // Water reserves
    Oxygen,             // Oxygen tanks
    Medicine,           // Medical supplies
    Fuel,               // Fuel for vehicles and ships

    // Construction materials
    ConstructionMaterial, // Generic building materials
    Metal,              // Processed metal
    Electronics,        // Electronic components
    Polymers,           // Plastic and polymer materials
    Ceramics,           // Heat-resistant materials

    // Equipment
    Equipment,          // Generic equipment
    LifeSupportUnit,    // Life support systems
    PowerGenerator,     // Power generation equipment
    CommunicationArray, // Communications equipment
    SolarPanel,         // Solar power equipment
    NuclearReactor,     // Nuclear power components
    MiningEquipment,    // Tools for mining
    ScienceEquipment,   // Research equipment
    FarmingEquipment,   // Agricultural tools

    // Prefabricated structures
    HabitatModule,      // Pre-fab living quarters
    LabModule,          // Pre-fab research lab
    WarehouseModule,    // Pre-fab storage
    FactoryModule,      // Pre-fab manufacturing

    // Raw resources (mined/extracted)
    RawOre,             // Unprocessed ore
    IceWater,           // Frozen water from mining
    RareEarths,         // Rare earth elements
    Uranium,            // Nuclear fuel source

    // Specialized
    Seeds,              // Agricultural seeds
    Embryos,            // Genetic diversity (animals/plants)
    DataArchive,        // Cultural/scientific data
    SparesParts         // Replacement parts for repairs
}

/// <summary>
/// Represents a vehicle that can be deployed on planetary surfaces.
/// </summary>
public record Vehicle
{
    public Ulid Id { get; init; }
    public string Name { get; init; } = "";
    public VehicleType Type { get; init; } = VehicleType.Rover;

    /// <summary>
    /// Mass in kilograms.
    /// </summary>
    public double MassKg { get; init; } = 500.0;

    /// <summary>
    /// Volume in cubic meters.
    /// </summary>
    public double VolumeM3 { get; init; } = 10.0;

    /// <summary>
    /// Maximum speed in km/h.
    /// </summary>
    public double MaxSpeedKmh { get; init; } = 30.0;

    /// <summary>
    /// Cargo capacity in kilograms.
    /// </summary>
    public double CargoCapacityKg { get; init; } = 1000.0;

    /// <summary>
    /// Fuel capacity (arbitrary units).
    /// </summary>
    public double FuelCapacity { get; init; } = 100.0;

    /// <summary>
    /// Current fuel level.
    /// </summary>
    public double CurrentFuel { get; init; } = 100.0;

    /// <summary>
    /// Health/condition (0-100, where 100 is pristine).
    /// </summary>
    public int Condition { get; init; } = 100;

    /// <summary>
    /// Current location: null if on ship, otherwise sector/building ID.
    /// </summary>
    public Ulid? LocationId { get; init; } = null;

    /// <summary>
    /// Current task assignment.
    /// </summary>
    public Ulid? TaskId { get; init; } = null;
}

/// <summary>
/// Types of vehicles.
/// </summary>
public enum VehicleType
{
    Rover,              // General-purpose exploration vehicle
    Excavator,          // Mining and digging vehicle
    Hauler,             // Heavy cargo transport
    Surveyor,           // Scientific survey vehicle
    Constructor,        // Mobile construction vehicle
    Tanker,             // Liquid transport (water, fuel)
    AmbulanceRover      // Medical emergency vehicle
}

/// <summary>
/// Represents an autonomous robot.
/// </summary>
public record Robot
{
    public Ulid Id { get; init; }
    public string Name { get; init; } = "";
    public RobotType Type { get; init; } = RobotType.GeneralPurpose;

    /// <summary>
    /// Mass in kilograms.
    /// </summary>
    public double MassKg { get; init; } = 100.0;

    /// <summary>
    /// Volume in cubic meters.
    /// </summary>
    public double VolumeM3 { get; init; } = 1.0;

    /// <summary>
    /// Power consumption in watts.
    /// </summary>
    public double PowerConsumptionW { get; init; } = 500.0;

    /// <summary>
    /// Battery capacity in watt-hours.
    /// </summary>
    public double BatteryCapacityWh { get; init; } = 5000.0;

    /// <summary>
    /// Current battery charge.
    /// </summary>
    public double CurrentCharge { get; init; } = 5000.0;

    /// <summary>
    /// Health/condition (0-100).
    /// </summary>
    public int Condition { get; init; } = 100;

    /// <summary>
    /// Current location: null if on ship, otherwise sector/building ID.
    /// </summary>
    public Ulid? LocationId { get; init; } = null;

    /// <summary>
    /// Current task assignment.
    /// </summary>
    public Ulid? TaskId { get; init; } = null;

    /// <summary>
    /// Efficiency rating (0-100, affected by condition and programming).
    /// </summary>
    public int Efficiency { get; init; } = 100;
}

/// <summary>
/// Types of robots.
/// </summary>
public enum RobotType
{
    GeneralPurpose,     // Jack-of-all-trades robot
    Constructor,        // Building and repair robot
    Miner,              // Mining robot
    Agricultural,       // Farming robot
    Research,           // Lab assistant robot
    Maintenance,        // Maintenance and repair robot
    Drone               // Flying drone (survey, delivery)
}
