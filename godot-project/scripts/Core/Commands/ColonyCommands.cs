using System;
using System.Collections.Generic;
using System.Linq;

namespace Outpost3.Core.Commands;
using Outpost3.Core.Domain;
using Outpost3.Core.Events;

/// <summary>
/// Command: Create and configure a new colony ship.
/// </summary>
public class CreateShipCommand : ICommand
{
    public string ShipName { get; init; } = "";
    public ShipType ShipType { get; init; } = ShipType.SleeperShip;
    public int ColonistCount { get; init; } = 100;
    public Dictionary<CargoType, int> CargoManifest { get; init; } = new();
    public List<VehicleType> VehicleTypes { get; init; } = new();
    public List<RobotType> RobotTypes { get; init; } = new();

    public List<IGameEvent> Execute(GameState state)
    {
        var events = new List<IGameEvent>();
        var shipId = Ulid.NewUlid();

        // Generate colonists
        var colonistIds = new List<Ulid>();
        for (int i = 0; i < ColonistCount; i++)
        {
            var colonistId = Ulid.NewUlid();
            colonistIds.Add(colonistId);

            // Random role distribution
            var role = GetRandomRole(i, ColonistCount);
            var colonistName = GenerateColonistName(i);

            events.Add(new ColonistCreatedEvent
            {
                Timestamp = state.GameTime,
                ColonistId = colonistId,
                ColonistName = colonistName,
                Role = role,
                Age = Random.Shared.Next(25, 55)
            });
        }

        // Generate vehicles
        var vehicleIds = new List<Ulid>();
        foreach (var vehicleType in VehicleTypes)
        {
            vehicleIds.Add(Ulid.NewUlid());
        }

        // Generate robots
        var robotIds = new List<Ulid>();
        foreach (var robotType in RobotTypes)
        {
            robotIds.Add(Ulid.NewUlid());
        }

        // Create ship
        events.Add(new ShipCreatedEvent
        {
            Timestamp = state.GameTime,
            ShipId = shipId,
            ShipName = ShipName,
            ShipType = ShipType,
            ColonistIds = colonistIds,
            VehicleIds = vehicleIds,
            RobotIds = robotIds,
            CargoItemCount = CargoManifest.Values.Sum()
        });

        events.Add(new ColonistsAssignedToShipEvent
        {
            Timestamp = state.GameTime,
            ShipId = shipId,
            ColonistIds = colonistIds
        });

        return events;
    }

    private static ColonistRole GetRandomRole(int index, int total)
    {
        // Distribution: 40% laborer, 20% engineer, 15% scientist, 25% other
        var ratio = (double)index / total;
        if (ratio < 0.20) return ColonistRole.Engineer;
        if (ratio < 0.35) return ColonistRole.Scientist;
        if (ratio < 0.40) return ColonistRole.Medic;
        if (ratio < 0.50) return ColonistRole.Technician;
        if (ratio < 0.55) return ColonistRole.Pilot;
        if (ratio < 0.65) return ColonistRole.Miner;
        if (ratio < 0.75) return ColonistRole.Farmer;
        return ColonistRole.Laborer;
    }

    private static string GenerateColonistName(int index)
    {
        var firstNames = new[] { "Alex", "Jordan", "Morgan", "Casey", "Riley", "Taylor", "Jamie", "Quinn", "Avery", "Sage" };
        var lastNames = new[] { "Chen", "Rodriguez", "Johnson", "Kim", "Patel", "Williams", "Garcia", "Martinez", "Davis", "Brown" };

        var firstName = firstNames[index % firstNames.Length];
        var lastName = lastNames[(index / firstNames.Length) % lastNames.Length];
        return $"{firstName} {lastName}";
    }
}

/// <summary>
/// Command: Launch a colony ship to a destination system.
/// </summary>
public class LaunchShipCommand : ICommand
{
    public Ulid ShipId { get; init; }
    public Ulid TargetSystemId { get; init; }
    public double TravelTimeYears { get; init; } = 10.0;

    public List<IGameEvent> Execute(GameState state)
    {
        var ship = state.Ships.FirstOrDefault(s => s.Id == ShipId);
        if (ship == null)
        {
            throw new InvalidOperationException($"Ship {ShipId} not found");
        }

        var targetSystem = state.Systems.FirstOrDefault(s => s.Id == TargetSystemId);
        if (targetSystem == null)
        {
            throw new InvalidOperationException($"System {TargetSystemId} not found");
        }

        var arrivalTime = state.GameTime + (TravelTimeYears * 365.25); // Convert years to days

        return new List<IGameEvent>
        {
            new ShipLaunchedEvent
            {
                Timestamp = state.GameTime,
                ShipId = ShipId,
                TargetSystemId = TargetSystemId,
                ArrivalTime = arrivalTime
            }
        };
    }
}

/// <summary>
/// Command: Generate a surface map for a celestial body.
/// </summary>
public class GenerateSurfaceMapCommand : ICommand
{
    public Ulid BodyId { get; init; }
    public int Width { get; init; } = 32;
    public int Height { get; init; } = 24;

    public List<IGameEvent> Execute(GameState state)
    {
        // Find the body
        CelestialBody? body = null;
        foreach (var system in state.Systems)
        {
            body = system.Bodies.FirstOrDefault(b => b.Id == BodyId);
            if (body != null) break;
        }

        if (body == null)
        {
            throw new InvalidOperationException($"Body {BodyId} not found");
        }

        // Count sectors
        int sectorCount = Width * Height;

        return new List<IGameEvent>
        {
            new SurfaceMapGeneratedEvent
            {
                Timestamp = state.GameTime,
                BodyId = BodyId,
                Width = Width,
                Height = Height,
                SectorCount = sectorCount
            }
        };
    }
}

/// <summary>
/// Command: Select a landing site on a body's surface.
/// </summary>
public class SelectLandingSiteCommand : ICommand
{
    public Ulid ShipId { get; init; }
    public Ulid BodyId { get; init; }
    public int SectorX { get; init; }
    public int SectorY { get; init; }

    public List<IGameEvent> Execute(GameState state)
    {
        var ship = state.Ships.FirstOrDefault(s => s.Id == ShipId);
        if (ship == null)
        {
            throw new InvalidOperationException($"Ship {ShipId} not found");
        }

        if (!state.SurfaceMaps.TryGetValue(BodyId, out var surfaceMap))
        {
            throw new InvalidOperationException($"No surface map for body {BodyId}");
        }

        var sector = surfaceMap.GetSector(SectorX, SectorY);
        if (sector == null)
        {
            throw new InvalidOperationException($"Sector ({SectorX}, {SectorY}) not found");
        }

        return new List<IGameEvent>
        {
            new LandingSiteSelectedEvent
            {
                Timestamp = state.GameTime,
                ShipId = ShipId,
                BodyId = BodyId,
                SectorX = SectorX,
                SectorY = SectorY,
                SuitabilityScore = sector.SuitabilityScore
            }
        };
    }
}

/// <summary>
/// Command: Initiate landing sequence for a ship.
/// </summary>
public class LandShipCommand : ICommand
{
    public Ulid ShipId { get; init; }
    public Ulid BodyId { get; init; }
    public int SectorX { get; init; }
    public int SectorY { get; init; }

    public List<IGameEvent> Execute(GameState state)
    {
        var ship = state.Ships.FirstOrDefault(s => s.Id == ShipId);
        if (ship == null)
        {
            throw new InvalidOperationException($"Ship {ShipId} not found");
        }

        if (!state.SurfaceMaps.TryGetValue(BodyId, out var surfaceMap))
        {
            throw new InvalidOperationException($"No surface map for body {BodyId}");
        }

        var sector = surfaceMap.GetSector(SectorX, SectorY);
        if (sector == null)
        {
            throw new InvalidOperationException($"Sector ({SectorX}, {SectorY}) not found");
        }

        // Calculate landing risk based on terrain and hazards
        var baseRisk = 0.05; // 5% base risk
        var terrainRisk = sector.Terrain switch
        {
            TerrainType.Mountains => 0.15,
            TerrainType.Volcanic => 0.20,
            TerrainType.Lava => 0.30,
            TerrainType.Canyon => 0.10,
            _ => 0.0
        };

        var hazardRisk = sector.Hazards.Sum(h => h.Severity / 1000.0); // Convert severity to risk
        var totalRisk = Math.Min(0.95, baseRisk + terrainRisk + hazardRisk);

        var events = new List<IGameEvent>
        {
            new LandingInitiatedEvent
            {
                Timestamp = state.GameTime,
                ShipId = ShipId,
                BodyId = BodyId,
                LandingRisk = totalRisk,
                SectorX = SectorX,
                SectorY = SectorY
            }
        };

        // Determine landing outcome
        var success = Random.Shared.NextDouble() > totalRisk;

        if (success)
        {
            // Successful landing
            var cargoDrift = Random.Shared.NextDouble() < (totalRisk / 2); // Cargo might drift even on success
            var casualties = totalRisk > 0.10 && Random.Shared.NextDouble() < 0.1
                ? Random.Shared.Next(1, Math.Max(2, ship.Colonists.Count / 100))
                : 0;

            events.Add(new ShipLandedEvent
            {
                Timestamp = state.GameTime,
                ShipId = ShipId,
                BodyId = BodyId,
                SectorX = SectorX,
                SectorY = SectorY,
                CargoDriftOccurred = cargoDrift,
                ColonistCasualties = casualties
            });
        }
        else
        {
            // Landing failed
            var casualties = Random.Shared.Next(
                ship.Colonists.Count / 10,
                ship.Colonists.Count / 4
            );
            var cargoLoss = Random.Shared.Next(20, 60);

            events.Add(new LandingFailedEvent
            {
                Timestamp = state.GameTime,
                ShipId = ShipId,
                BodyId = BodyId,
                Reason = "Atmospheric conditions caused loss of control",
                ColonistCasualties = casualties,
                CargoLoss = cargoLoss
            });
        }

        return events;
    }
}

/// <summary>
/// Command: Establish a colony after successful landing.
/// </summary>
public class EstablishColonyCommand : ICommand
{
    public Ulid ShipId { get; init; }
    public string ColonyName { get; init; } = "";
    public Ulid BodyId { get; init; }
    public Ulid SystemId { get; init; }
    public int SectorX { get; init; }
    public int SectorY { get; init; }

    public List<IGameEvent> Execute(GameState state)
    {
        var ship = state.Ships.FirstOrDefault(s => s.Id == ShipId);
        if (ship == null)
        {
            throw new InvalidOperationException($"Ship {ShipId} not found");
        }

        var colonyId = Ulid.NewUlid();
        var colonistIds = ship.Colonists.Select(c => c.Id).ToList();

        var events = new List<IGameEvent>
        {
            new ColonyEstablishedEvent
            {
                Timestamp = state.GameTime,
                ColonyId = colonyId,
                ColonyName = ColonyName,
                SystemId = SystemId,
                BodyId = BodyId,
                InitialPopulation = colonistIds.Count,
                SectorX = SectorX,
                SectorY = SectorY
            },
            new ColonistsTransferredToColonyEvent
            {
                Timestamp = state.GameTime,
                ShipId = ShipId,
                ColonyId = colonyId,
                ColonistIds = colonistIds
            },
            new CargoDeployedEvent
            {
                Timestamp = state.GameTime,
                ShipId = ShipId,
                ColonyId = colonyId,
                DeployedCargoIds = ship.Cargo.Select(c => c.Id).ToList(),
                DeployedVehicleIds = ship.Vehicles.Select(v => v.Id).ToList(),
                DeployedRobotIds = ship.Robots.Select(r => r.Id).ToList()
            }
        };

        return events;
    }
}

/// <summary>
/// Command: Build a structure in a colony.
/// </summary>
public class BuildStructureCommand : ICommand
{
    public Ulid ColonyId { get; init; }
    public StructureType StructureType { get; init; }
    public string StructureName { get; init; } = "";
    public int SectorX { get; init; }
    public int SectorY { get; init; }

    public List<IGameEvent> Execute(GameState state)
    {
        var colony = state.Colonies.FirstOrDefault(c => c.Id == ColonyId);
        if (colony == null)
        {
            throw new InvalidOperationException($"Colony {ColonyId} not found");
        }

        var structureId = Ulid.NewUlid();

        return new List<IGameEvent>
        {
            new StructureBuiltEvent
            {
                Timestamp = state.GameTime,
                ColonyId = ColonyId,
                StructureId = structureId,
                StructureType = StructureType,
                StructureName = StructureName,
                SectorX = SectorX,
                SectorY = SectorY
            }
        };
    }
}
