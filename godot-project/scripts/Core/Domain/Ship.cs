using System;
using System.Collections.Generic;
using System.Linq;

namespace Outpost3.Core.Domain;

/// <summary>
/// Represents a colony ship carrying colonists and cargo to a destination system.
/// </summary>
public record Ship
{
    public Ulid Id { get; init; }
    public string Name { get; init; } = "";

    /// <summary>
    /// Current location: either traveling to a system or in orbit around one.
    /// </summary>
    public ShipLocation Location { get; init; } = ShipLocation.InTransit(Ulid.Empty, 0);

    /// <summary>
    /// List of colonists aboard the ship.
    /// </summary>
    public List<Colonist> Colonists { get; init; } = new();

    /// <summary>
    /// Cargo manifest - all items being transported.
    /// </summary>
    public List<CargoItem> Cargo { get; init; } = new();

    /// <summary>
    /// Vehicles stored on the ship (rovers, excavators, etc.).
    /// </summary>
    public List<Vehicle> Vehicles { get; init; } = new();

    /// <summary>
    /// Robots stored on the ship.
    /// </summary>
    public List<Robot> Robots { get; init; } = new();

    /// <summary>
    /// Ship type: GenerationShip or SleeperShip.
    /// </summary>
    public ShipType Type { get; init; } = ShipType.SleeperShip;

    /// <summary>
    /// Total mass capacity in metric tons.
    /// </summary>
    public double MassCapacityTons { get; init; } = 10000.0;

    /// <summary>
    /// Total volume capacity in cubic meters.
    /// </summary>
    public double VolumeCapacityM3 { get; init; } = 50000.0;

    /// <summary>
    /// Power budget in kilowatts.
    /// </summary>
    public double PowerBudgetKW { get; init; } = 5000.0;

    /// <summary>
    /// Calculate current total mass of all cargo, vehicles, robots, and colonists.
    /// </summary>
    public double GetTotalMass()
    {
        var cargoMass = Cargo.Sum(c => c.MassKg) / 1000.0; // Convert to tons
        var vehicleMass = Vehicles.Sum(v => v.MassKg) / 1000.0;
        var robotMass = Robots.Sum(r => r.MassKg) / 1000.0;
        var colonistMass = Colonists.Count * 80.0 / 1000.0; // ~80kg per person
        return cargoMass + vehicleMass + robotMass + colonistMass;
    }

    /// <summary>
    /// Calculate current total volume used.
    /// </summary>
    public double GetTotalVolume()
    {
        var cargoVolume = Cargo.Sum(c => c.VolumeM3);
        var vehicleVolume = Vehicles.Sum(v => v.VolumeM3);
        var robotVolume = Robots.Sum(r => r.VolumeM3);
        var colonistVolume = Colonists.Count * 2.0; // ~2m³ per person in cryo/quarters
        return cargoVolume + vehicleVolume + robotVolume + colonistVolume;
    }

    /// <summary>
    /// Calculate current power consumption.
    /// </summary>
    public double GetTotalPowerConsumption()
    {
        // Base ship systems consume 1000 KW
        var basePower = 1000.0;

        // Generation ships need life support power
        var lifeSupportPower = Type == ShipType.GenerationShip
            ? Colonists.Count * 0.5 // 0.5 KW per colonist
            : Colonists.Count * 0.01; // Cryo maintenance: 0.01 KW per colonist

        return basePower + lifeSupportPower;
    }
}

/// <summary>
/// Ship location - either in transit or in orbit.
/// </summary>
public record ShipLocation
{
    public ShipLocationState State { get; init; }
    public Ulid? TargetSystemId { get; init; }
    public Ulid? OrbitBodyId { get; init; }
    public double ArrivalTime { get; init; }

    public static ShipLocation InTransit(Ulid targetSystemId, double arrivalTime) =>
        new() { State = ShipLocationState.InTransit, TargetSystemId = targetSystemId, ArrivalTime = arrivalTime };

    public static ShipLocation InOrbit(Ulid systemId, Ulid bodyId) =>
        new() { State = ShipLocationState.InOrbit, TargetSystemId = systemId, OrbitBodyId = bodyId };

    public static ShipLocation Landed(Ulid systemId, Ulid bodyId) =>
        new() { State = ShipLocationState.Landed, TargetSystemId = systemId, OrbitBodyId = bodyId };
}

public enum ShipLocationState
{
    InTransit,
    InOrbit,
    Landed
}

public enum ShipType
{
    /// <summary>
    /// Colonists in cryosleep - less power, no social events during travel.
    /// </summary>
    SleeperShip,

    /// <summary>
    /// Colonists awake - more power, social events, births/deaths during travel.
    /// </summary>
    GenerationShip
}
