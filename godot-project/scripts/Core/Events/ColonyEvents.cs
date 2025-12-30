using System;
using System.Collections.Generic;

namespace Outpost3.Core.Events;
using Outpost3.Core.Domain;

/// <summary>
/// Event: A colony ship was created and configured.
/// </summary>
public record ShipCreatedEvent : GameEvent
{
    public Ulid ShipId { get; init; }
    public string ShipName { get; init; } = "";
    public ShipType ShipType { get; init; }
    public List<Ulid> ColonistIds { get; init; } = new();
    public List<Ulid> VehicleIds { get; init; } = new();
    public List<Ulid> RobotIds { get; init; } = new();
    public int CargoItemCount { get; init; } = 0;

    public override string GetDescription() =>
        $"Colony ship '{ShipName}' created with {ColonistIds.Count} colonists";
}

/// <summary>
/// Event: A colony ship has begun its journey to a destination system.
/// </summary>
public record ShipLaunchedEvent : GameEvent
{
    public Ulid ShipId { get; init; }
    public Ulid TargetSystemId { get; init; }
    public double ArrivalTime { get; init; }

    public override string GetDescription() =>
        $"Ship launched to system, arrival at {ArrivalTime:F2}";
}

/// <summary>
/// Event: A colony ship has arrived at its destination system.
/// </summary>
public record ShipArrivedAtSystemEvent : GameEvent
{
    public Ulid ShipId { get; init; }
    public Ulid SystemId { get; init; }

    public override string GetDescription() =>
        $"Ship arrived at destination system";
}

/// <summary>
/// Event: Ship moved to orbit around a celestial body.
/// </summary>
public record ShipEnteredOrbitEvent : GameEvent
{
    public Ulid ShipId { get; init; }
    public Ulid BodyId { get; init; }

    public override string GetDescription() =>
        $"Ship entered orbit around body";
}

/// <summary>
/// Event: Surface map generated for a celestial body.
/// </summary>
public record SurfaceMapGeneratedEvent : GameEvent
{
    public Ulid BodyId { get; init; }
    public int Width { get; init; }
    public int Height { get; init; }
    public int SectorCount { get; init; }

    public override string GetDescription() =>
        $"Surface map generated: {Width}x{Height} sectors";
}

/// <summary>
/// Event: A landing site was selected on a body.
/// </summary>
public record LandingSiteSelectedEvent : GameEvent
{
    public Ulid ShipId { get; init; }
    public Ulid BodyId { get; init; }
    public int SectorX { get; init; }
    public int SectorY { get; init; }
    public int SuitabilityScore { get; init; }

    public override string GetDescription() =>
        $"Landing site selected at ({SectorX}, {SectorY}) - suitability: {SuitabilityScore}";
}

/// <summary>
/// Event: Ship landing sequence initiated.
/// </summary>
public record LandingInitiatedEvent : GameEvent
{
    public Ulid ShipId { get; init; }
    public Ulid BodyId { get; init; }
    public double LandingRisk { get; init; }
    public int SectorX { get; init; }
    public int SectorY { get; init; }

    public override string GetDescription() =>
        $"Landing initiated at ({SectorX}, {SectorY}) - risk: {LandingRisk:P1}";
}

/// <summary>
/// Event: Ship successfully landed on a body.
/// </summary>
public record ShipLandedEvent : GameEvent
{
    public Ulid ShipId { get; init; }
    public Ulid BodyId { get; init; }
    public int SectorX { get; init; }
    public int SectorY { get; init; }
    public bool CargoDriftOccurred { get; init; }
    public int ColonistCasualties { get; init; }

    public override string GetDescription()
    {
        var casualties = ColonistCasualties > 0 ? $" - {ColonistCasualties} casualties" : "";
        var drift = CargoDriftOccurred ? " - some cargo drifted" : "";
        return $"Ship landed at ({SectorX}, {SectorY}){casualties}{drift}";
    }
}

/// <summary>
/// Event: Landing failed/crashed.
/// </summary>
public record LandingFailedEvent : GameEvent
{
    public Ulid ShipId { get; init; }
    public Ulid BodyId { get; init; }
    public string Reason { get; init; } = "";
    public int ColonistCasualties { get; init; }
    public int CargoLoss { get; init; }

    public override string GetDescription() =>
        $"Landing failed: {Reason} - {ColonistCasualties} casualties, {CargoLoss}% cargo lost";
}

/// <summary>
/// Event: A new colony was established.
/// </summary>
public record ColonyEstablishedEvent : GameEvent
{
    public Ulid ColonyId { get; init; }
    public string ColonyName { get; init; } = "";
    public Ulid SystemId { get; init; }
    public Ulid BodyId { get; init; }
    public int InitialPopulation { get; init; }
    public int SectorX { get; init; }
    public int SectorY { get; init; }

    public override string GetDescription() =>
        $"Colony '{ColonyName}' established with {InitialPopulation} colonists";
}

/// <summary>
/// Event: Cargo was deployed from ship to colony.
/// </summary>
public record CargoDeployedEvent : GameEvent
{
    public Ulid ShipId { get; init; }
    public Ulid ColonyId { get; init; }
    public List<Ulid> DeployedCargoIds { get; init; } = new();
    public List<Ulid> DeployedVehicleIds { get; init; } = new();
    public List<Ulid> DeployedRobotIds { get; init; } = new();

    public override string GetDescription() =>
        $"Deployed {DeployedCargoIds.Count} cargo items, {DeployedVehicleIds.Count} vehicles, {DeployedRobotIds.Count} robots";
}

/// <summary>
/// Event: A structure was built in a colony.
/// </summary>
public record StructureBuiltEvent : GameEvent
{
    public Ulid ColonyId { get; init; }
    public Ulid StructureId { get; init; }
    public StructureType StructureType { get; init; }
    public string StructureName { get; init; } = "";
    public int SectorX { get; init; }
    public int SectorY { get; init; }

    public override string GetDescription() =>
        $"Built {StructureName} ({StructureType}) at ({SectorX}, {SectorY})";
}

/// <summary>
/// Event: Colonists were assigned to a ship.
/// </summary>
public record ColonistsAssignedToShipEvent : GameEvent
{
    public Ulid ShipId { get; init; }
    public List<Ulid> ColonistIds { get; init; } = new();

    public override string GetDescription() =>
        $"Assigned {ColonistIds.Count} colonists to ship";
}

/// <summary>
/// Event: Colonists were transferred to a colony.
/// </summary>
public record ColonistsTransferredToColonyEvent : GameEvent
{
    public Ulid ShipId { get; init; }
    public Ulid ColonyId { get; init; }
    public List<Ulid> ColonistIds { get; init; } = new();

    public override string GetDescription() =>
        $"Transferred {ColonistIds.Count} colonists to colony";
}

/// <summary>
/// Event: A colonist was created/generated.
/// </summary>
public record ColonistCreatedEvent : GameEvent
{
    public Ulid ColonistId { get; init; }
    public string ColonistName { get; init; } = "";
    public ColonistRole Role { get; init; }
    public int Age { get; init; }

    public override string GetDescription() =>
        $"Colonist {ColonistName} recruited ({Role}, age {Age})";
}

/// <summary>
/// Event: Colony morale changed.
/// </summary>
public record ColonyMoraleChangedEvent : GameEvent
{
    public Ulid ColonyId { get; init; }
    public int OldMorale { get; init; }
    public int NewMorale { get; init; }
    public string Reason { get; init; } = "";

    public override string GetDescription() =>
        $"Colony morale changed: {OldMorale} → {NewMorale} ({Reason})";
}

/// <summary>
/// Event: Colony power status changed.
/// </summary>
public record ColonyPowerStatusChangedEvent : GameEvent
{
    public Ulid ColonyId { get; init; }
    public PowerStatus Status { get; init; }
    public double Generation { get; init; }
    public double Consumption { get; init; }

    public override string GetDescription() =>
        $"Power status: {Status} (Gen: {Generation:F1} KW, Use: {Consumption:F1} KW)";
}

/// <summary>
/// Event: Colony life support status changed.
/// </summary>
public record ColonyLifeSupportStatusChangedEvent : GameEvent
{
    public Ulid ColonyId { get; init; }
    public LifeSupportStatus Status { get; init; }
    public double OxygenGeneration { get; init; }
    public double OxygenConsumption { get; init; }

    public override string GetDescription() =>
        $"Life support status: {Status} (O₂ Gen: {OxygenGeneration:F1}, Use: {OxygenConsumption:F1})";
}
