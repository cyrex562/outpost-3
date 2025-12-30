using System;
using System.Collections.Generic;
using System.Linq;

namespace Outpost3.Core.Services;
using Outpost3.Core.Domain;
using Outpost3.Core.Events;

/// <summary>
/// Applies colony-related events to the game state.
/// </summary>
public static class ColonyEventApplicator
{
    public static GameState Apply(GameState state, IGameEvent @event)
    {
        return @event switch
        {
            ColonistCreatedEvent e => ApplyColonistCreated(state, e),
            ShipCreatedEvent e => ApplyShipCreated(state, e),
            ColonistsAssignedToShipEvent e => ApplyColonistsAssignedToShip(state, e),
            ShipLaunchedEvent e => ApplyShipLaunched(state, e),
            ShipArrivedAtSystemEvent e => ApplyShipArrivedAtSystem(state, e),
            ShipEnteredOrbitEvent e => ApplyShipEnteredOrbit(state, e),
            SurfaceMapGeneratedEvent e => ApplySurfaceMapGenerated(state, e),
            LandingSiteSelectedEvent e => ApplyLandingSiteSelected(state, e),
            LandingInitiatedEvent e => ApplyLandingInitiated(state, e),
            ShipLandedEvent e => ApplyShipLanded(state, e),
            LandingFailedEvent e => ApplyLandingFailed(state, e),
            ColonyEstablishedEvent e => ApplyColonyEstablished(state, e),
            ColonistsTransferredToColonyEvent e => ApplyColonistsTransferredToColony(state, e),
            CargoDeployedEvent e => ApplyCargoDeployed(state, e),
            StructureBuiltEvent e => ApplyStructureBuilt(state, e),
            ColonyMoraleChangedEvent e => ApplyColonyMoraleChanged(state, e),
            ColonyPowerStatusChangedEvent e => ApplyColonyPowerStatusChanged(state, e),
            ColonyLifeSupportStatusChangedEvent e => ApplyColonyLifeSupportStatusChanged(state, e),
            _ => state
        };
    }

    private static GameState ApplyColonistCreated(GameState state, ColonistCreatedEvent e)
    {
        var colonist = new Colonist
        {
            Id = e.ColonistId,
            Name = e.ColonistName,
            Role = e.Role,
            Age = e.Age,
            Health = 100,
            Morale = 75,
            Fatigue = 0,
            Skills = InitializeSkills(e.Role)
        };

        return state.WithColonistsAdded(new List<Colonist> { colonist });
    }

    private static GameState ApplyShipCreated(GameState state, ShipCreatedEvent e)
    {
        // Get colonists
        var colonists = state.Colonists.Where(c => e.ColonistIds.Contains(c.Id)).ToList();

        // Create initial cargo
        var cargo = CreateInitialCargo();

        // Create vehicles
        var vehicles = e.VehicleIds.Select((id, idx) => CreateVehicle(id, idx)).ToList();

        // Create robots
        var robots = e.RobotIds.Select((id, idx) => CreateRobot(id, idx)).ToList();

        var ship = new Ship
        {
            Id = e.ShipId,
            Name = e.ShipName,
            Type = e.ShipType,
            Colonists = colonists,
            Cargo = cargo,
            Vehicles = vehicles,
            Robots = robots,
            Location = ShipLocation.InTransit(Ulid.Empty, 0),
            MassCapacityTons = 10000.0,
            VolumeCapacityM3 = 50000.0,
            PowerBudgetKW = 5000.0
        };

        state = state.WithVehiclesAdded(vehicles);
        state = state.WithRobotsAdded(robots);
        return state.WithShipAdded(ship);
    }

    private static GameState ApplyColonistsAssignedToShip(GameState state, ColonistsAssignedToShipEvent e)
    {
        // Colonists already in ship from ShipCreated event
        return state;
    }

    private static GameState ApplyShipLaunched(GameState state, ShipLaunchedEvent e)
    {
        var ship = state.Ships.First(s => s.Id == e.ShipId);
        var updatedShip = ship with
        {
            Location = ShipLocation.InTransit(e.TargetSystemId, e.ArrivalTime)
        };
        return state.WithShipUpdated(updatedShip);
    }

    private static GameState ApplyShipArrivedAtSystem(GameState state, ShipArrivedAtSystemEvent e)
    {
        var ship = state.Ships.First(s => s.Id == e.ShipId);
        var system = state.Systems.First(s => s.Id == e.SystemId);
        var firstBody = system.Bodies.FirstOrDefault();

        var updatedShip = ship with
        {
            Location = firstBody != null
                ? ShipLocation.InOrbit(e.SystemId, firstBody.Id)
                : ShipLocation.InTransit(e.SystemId, state.GameTime)
        };
        return state.WithShipUpdated(updatedShip);
    }

    private static GameState ApplyShipEnteredOrbit(GameState state, ShipEnteredOrbitEvent e)
    {
        var ship = state.Ships.First(s => s.Id == e.ShipId);

        // Find system containing this body
        StarSystem? system = null;
        foreach (var sys in state.Systems)
        {
            if (sys.Bodies.Any(b => b.Id == e.BodyId))
            {
                system = sys;
                break;
            }
        }

        if (system == null)
        {
            return state;
        }

        var updatedShip = ship with
        {
            Location = ShipLocation.InOrbit(system.Id, e.BodyId)
        };
        return state.WithShipUpdated(updatedShip);
    }

    private static GameState ApplySurfaceMapGenerated(GameState state, SurfaceMapGeneratedEvent e)
    {
        var surfaceMap = GenerateProceduralSurfaceMap(e.BodyId, e.Width, e.Height);
        return state.WithSurfaceMapAdded(e.BodyId, surfaceMap);
    }

    private static GameState ApplyLandingSiteSelected(GameState state, LandingSiteSelectedEvent e)
    {
        // Just tracking, no state change needed
        return state;
    }

    private static GameState ApplyLandingInitiated(GameState state, LandingInitiatedEvent e)
    {
        // Landing in progress
        return state;
    }

    private static GameState ApplyShipLanded(GameState state, ShipLandedEvent e)
    {
        var ship = state.Ships.First(s => s.Id == e.ShipId);
        var updatedShip = ship with
        {
            Location = ShipLocation.Landed(ship.Location.TargetSystemId!.Value, e.BodyId)
        };

        // Handle casualties
        if (e.ColonistCasualties > 0)
        {
            var casualties = ship.Colonists.Take(e.ColonistCasualties).Select(c => c.Id).ToList();
            var survivingColonists = ship.Colonists.Skip(e.ColonistCasualties).ToList();
            updatedShip = updatedShip with { Colonists = survivingColonists };

            // Remove casualties from global list
            var globalColonists = state.Colonists.Where(c => !casualties.Contains(c.Id)).ToList();
            state = state with { Colonists = globalColonists };
        }

        return state.WithShipUpdated(updatedShip);
    }

    private static GameState ApplyLandingFailed(GameState state, LandingFailedEvent e)
    {
        var ship = state.Ships.First(s => s.Id == e.ShipId);

        // Remove casualties
        var casualties = ship.Colonists.Take(e.ColonistCasualties).Select(c => c.Id).ToList();
        var survivingColonists = ship.Colonists.Skip(e.ColonistCasualties).ToList();

        // Remove cargo
        var cargoToRemove = ship.Cargo.Count * e.CargoLoss / 100;
        var remainingCargo = ship.Cargo.Skip(cargoToRemove).ToList();

        var updatedShip = ship with
        {
            Colonists = survivingColonists,
            Cargo = remainingCargo,
            Location = ShipLocation.Landed(ship.Location.TargetSystemId!.Value, e.BodyId)
        };

        // Remove casualties from global list
        var globalColonists = state.Colonists.Where(c => !casualties.Contains(c.Id)).ToList();
        state = state with { Colonists = globalColonists };

        return state.WithShipUpdated(updatedShip);
    }

    private static GameState ApplyColonyEstablished(GameState state, ColonyEstablishedEvent e)
    {
        var colony = new Colony
        {
            Id = e.ColonyId,
            Name = e.ColonyName,
            SystemId = e.SystemId,
            BodyId = e.BodyId,
            LandingSite = new SectorCoordinates(e.SectorX, e.SectorY),
            FoundedTime = state.GameTime,
            ColonistIds = new List<Ulid>(),
            Structures = new List<Structure>(),
            Resources = new Dictionary<ResourceType, double>(),
            Morale = 75
        };

        return state.WithColonyAdded(colony);
    }

    private static GameState ApplyColonistsTransferredToColony(GameState state, ColonistsTransferredToColonyEvent e)
    {
        var colony = state.Colonies.First(c => c.Id == e.ColonyId);
        var updatedColony = colony with
        {
            ColonistIds = e.ColonistIds
        };

        // Wake colonists from cryosleep
        var updatedColonists = new List<Colonist>();
        foreach (var colonist in state.Colonists)
        {
            if (e.ColonistIds.Contains(colonist.Id))
            {
                updatedColonists.Add(colonist with { InCryosleep = false });
            }
        }

        state = state.WithColonyUpdated(updatedColony);

        foreach (var colonist in updatedColonists)
        {
            state = state.WithColonistUpdated(colonist);
        }

        // Clear colonists from ship
        var ship = state.Ships.First(s => s.Id == e.ShipId);
        var updatedShip = ship with { Colonists = new List<Colonist>() };
        return state.WithShipUpdated(updatedShip);
    }

    private static GameState ApplyCargoDeployed(GameState state, CargoDeployedEvent e)
    {
        var ship = state.Ships.First(s => s.Id == e.ShipId);
        var colony = state.Colonies.First(c => c.Id == e.ColonyId);

        // Clear ship cargo
        var updatedShip = ship with
        {
            Cargo = new List<CargoItem>(),
            Vehicles = new List<Vehicle>(),
            Robots = new List<Robot>()
        };

        // Add vehicle and robot IDs to colony
        var updatedColony = colony with
        {
            VehicleIds = e.DeployedVehicleIds,
            RobotIds = e.DeployedRobotIds
        };

        // Convert cargo to resources
        var resources = new Dictionary<ResourceType, double>(colony.Resources);
        foreach (var cargoId in e.DeployedCargoIds)
        {
            var cargo = ship.Cargo.FirstOrDefault(c => c.Id == cargoId);
            if (cargo != null)
            {
                var resourceType = MapCargoToResource(cargo.Type);
                if (!resources.ContainsKey(resourceType))
                {
                    resources[resourceType] = 0;
                }
                resources[resourceType] += cargo.Quantity;
            }
        }

        updatedColony = updatedColony with { Resources = resources };

        state = state.WithShipUpdated(updatedShip);
        return state.WithColonyUpdated(updatedColony);
    }

    private static GameState ApplyStructureBuilt(GameState state, StructureBuiltEvent e)
    {
        var colony = state.Colonies.First(c => c.Id == e.ColonyId);

        var structure = CreateStructure(e.StructureId, e.StructureType, e.StructureName, e.SectorX, e.SectorY);

        var structures = new List<Structure>(colony.Structures) { structure };
        var updatedColony = colony with { Structures = structures };

        // Update power and oxygen based on structure type
        updatedColony = RecalculateColonyStats(updatedColony);

        return state.WithColonyUpdated(updatedColony);
    }

    private static GameState ApplyColonyMoraleChanged(GameState state, ColonyMoraleChangedEvent e)
    {
        var colony = state.Colonies.First(c => c.Id == e.ColonyId);
        var updatedColony = colony with { Morale = e.NewMorale };
        return state.WithColonyUpdated(updatedColony);
    }

    private static GameState ApplyColonyPowerStatusChanged(GameState state, ColonyPowerStatusChangedEvent e)
    {
        var colony = state.Colonies.First(c => c.Id == e.ColonyId);
        var updatedColony = colony with
        {
            PowerGeneration = e.Generation,
            PowerConsumption = e.Consumption
        };
        return state.WithColonyUpdated(updatedColony);
    }

    private static GameState ApplyColonyLifeSupportStatusChanged(GameState state, ColonyLifeSupportStatusChangedEvent e)
    {
        var colony = state.Colonies.First(c => c.Id == e.ColonyId);
        var updatedColony = colony with
        {
            OxygenGeneration = e.OxygenGeneration,
            OxygenConsumption = e.OxygenConsumption
        };
        return state.WithColonyUpdated(updatedColony);
    }

    // Helper methods

    private static Dictionary<SkillType, int> InitializeSkills(ColonistRole role)
    {
        var skills = new Dictionary<SkillType, int>();

        // Base skills for role
        switch (role)
        {
            case ColonistRole.Engineer:
                skills[SkillType.Engineering] = Random.Shared.Next(60, 90);
                skills[SkillType.Repair] = Random.Shared.Next(50, 80);
                skills[SkillType.Construction] = Random.Shared.Next(40, 70);
                break;
            case ColonistRole.Scientist:
                skills[SkillType.Science] = Random.Shared.Next(60, 90);
                skills[SkillType.Research] = Random.Shared.Next(50, 80);
                break;
            case ColonistRole.Medic:
                skills[SkillType.Medicine] = Random.Shared.Next(60, 90);
                break;
            case ColonistRole.Pilot:
                skills[SkillType.Piloting] = Random.Shared.Next(60, 90);
                break;
            case ColonistRole.Miner:
                skills[SkillType.Mining] = Random.Shared.Next(60, 90);
                break;
            case ColonistRole.Farmer:
                skills[SkillType.Agriculture] = Random.Shared.Next(60, 90);
                break;
            default:
                skills[SkillType.Construction] = Random.Shared.Next(30, 60);
                break;
        }

        return skills;
    }

    private static List<CargoItem> CreateInitialCargo()
    {
        var cargo = new List<CargoItem>();

        // Basic supplies
        cargo.Add(new CargoItem
        {
            Id = Ulid.NewUlid(),
            Name = "Food Rations",
            Type = CargoType.FoodRations,
            Quantity = 10000,
            MassKg = 1.0,
            VolumeM3 = 0.001
        });

        cargo.Add(new CargoItem
        {
            Id = Ulid.NewUlid(),
            Name = "Water",
            Type = CargoType.Water,
            Quantity = 50000,
            MassKg = 1.0,
            VolumeM3 = 0.001
        });

        cargo.Add(new CargoItem
        {
            Id = Ulid.NewUlid(),
            Name = "Habitat Module",
            Type = CargoType.HabitatModule,
            Quantity = 5,
            MassKg = 5000.0,
            VolumeM3 = 100.0
        });

        cargo.Add(new CargoItem
        {
            Id = Ulid.NewUlid(),
            Name = "Solar Panels",
            Type = CargoType.SolarPanel,
            Quantity = 10,
            MassKg = 500.0,
            VolumeM3 = 20.0
        });

        cargo.Add(new CargoItem
        {
            Id = Ulid.NewUlid(),
            Name = "Life Support Units",
            Type = CargoType.LifeSupportUnit,
            Quantity = 3,
            MassKg = 2000.0,
            VolumeM3 = 50.0
        });

        return cargo;
    }

    private static Vehicle CreateVehicle(Ulid id, int index)
    {
        var types = new[] { VehicleType.Rover, VehicleType.Excavator, VehicleType.Hauler, VehicleType.Surveyor };
        var type = types[index % types.Length];

        return new Vehicle
        {
            Id = id,
            Name = $"{type} {index + 1}",
            Type = type,
            MassKg = type == VehicleType.Hauler ? 2000.0 : 1000.0,
            VolumeM3 = type == VehicleType.Hauler ? 30.0 : 20.0,
            MaxSpeedKmh = type == VehicleType.Surveyor ? 50.0 : 30.0,
            CargoCapacityKg = type == VehicleType.Hauler ? 5000.0 : 1000.0,
            FuelCapacity = 100.0,
            CurrentFuel = 100.0,
            Condition = 100
        };
    }

    private static Robot CreateRobot(Ulid id, int index)
    {
        var types = new[] { RobotType.GeneralPurpose, RobotType.Constructor, RobotType.Miner, RobotType.Maintenance };
        var type = types[index % types.Length];

        return new Robot
        {
            Id = id,
            Name = $"{type} Robot {index + 1}",
            Type = type,
            MassKg = 150.0,
            VolumeM3 = 2.0,
            PowerConsumptionW = 500.0,
            BatteryCapacityWh = 5000.0,
            CurrentCharge = 5000.0,
            Condition = 100,
            Efficiency = 100
        };
    }

    private static SurfaceMap GenerateProceduralSurfaceMap(Ulid bodyId, int width, int height)
    {
        var sectors = new Dictionary<string, SurfaceSector>();

        for (int y = 0; y < height; y++)
        {
            for (int x = 0; x < width; x++)
            {
                var sector = GenerateSector(x, y);
                var key = $"{x},{y}";
                sectors[key] = sector;
            }
        }

        return new SurfaceMap
        {
            BodyId = bodyId,
            Width = width,
            Height = height,
            Sectors = sectors
        };
    }

    private static SurfaceSector GenerateSector(int x, int y)
    {
        var terrain = (TerrainType)Random.Shared.Next(0, 13);
        var elevation = Random.Shared.Next(-100, 500);
        var temperature = Random.Shared.Next(-50, 50);

        // Calculate suitability
        var suitability = 50;
        if (terrain == TerrainType.Plains || terrain == TerrainType.Valley)
        {
            suitability += 20;
        }
        else if (terrain == TerrainType.Mountains || terrain == TerrainType.Canyon)
        {
            suitability -= 10;
        }
        else if (terrain == TerrainType.Volcanic || terrain == TerrainType.Lava)
        {
            suitability -= 30;
        }

        if (temperature > -10 && temperature < 30)
        {
            suitability += 15;
        }

        suitability = Math.Clamp(suitability, 0, 100);

        return new SurfaceSector
        {
            Coordinates = new SectorCoordinates(x, y),
            Terrain = terrain,
            Elevation = elevation,
            Temperature = temperature,
            Explored = false,
            SuitabilityScore = suitability,
            ResourceDeposits = GenerateResourceDeposits(),
            Hazards = GenerateHazards(terrain)
        };
    }

    private static List<ResourceDeposit> GenerateResourceDeposits()
    {
        var deposits = new List<ResourceDeposit>();

        // 30% chance of having a resource deposit
        if (Random.Shared.NextDouble() < 0.3)
        {
            var resourceTypes = Enum.GetValues<ResourceType>();
            var resourceType = resourceTypes[Random.Shared.Next(resourceTypes.Length)];

            deposits.Add(new ResourceDeposit
            {
                Id = Ulid.NewUlid(),
                ResourceType = resourceType,
                Richness = Random.Shared.Next(20, 100),
                Accessibility = Random.Shared.Next(30, 100),
                Discovered = false,
                AmountRemaining = Random.Shared.Next(1000, 100000)
            });
        }

        return deposits;
    }

    private static List<SectorHazard> GenerateHazards(TerrainType terrain)
    {
        var hazards = new List<SectorHazard>();

        // Terrain-specific hazards
        if (terrain == TerrainType.Volcanic || terrain == TerrainType.Lava)
        {
            hazards.Add(new SectorHazard
            {
                Type = HazardType.ExtremeHeat,
                Severity = Random.Shared.Next(60, 100),
                Permanent = true
            });
        }
        else if (terrain == TerrainType.Polar)
        {
            hazards.Add(new SectorHazard
            {
                Type = HazardType.ExtremeCold,
                Severity = Random.Shared.Next(50, 90),
                Permanent = true
            });
        }

        // Random hazards (10% chance)
        if (Random.Shared.NextDouble() < 0.1)
        {
            var hazardTypes = Enum.GetValues<HazardType>();
            var hazardType = hazardTypes[Random.Shared.Next(hazardTypes.Length)];

            hazards.Add(new SectorHazard
            {
                Type = hazardType,
                Severity = Random.Shared.Next(20, 70),
                Permanent = Random.Shared.NextDouble() > 0.3
            });
        }

        return hazards;
    }

    private static ResourceType MapCargoToResource(CargoType cargoType)
    {
        return cargoType switch
        {
            CargoType.FoodRations => ResourceType.Food,
            CargoType.Water => ResourceType.Water,
            CargoType.Oxygen => ResourceType.Oxygen,
            CargoType.Metal => ResourceType.Metal,
            CargoType.Polymers => ResourceType.Polymer,
            CargoType.Ceramics => ResourceType.Ceramic,
            CargoType.Electronics => ResourceType.Electronics,
            CargoType.Fuel => ResourceType.Fuel,
            CargoType.Medicine => ResourceType.MedicalSupplies,
            CargoType.RawOre => ResourceType.RawOre,
            CargoType.RareEarths => ResourceType.RareEarths,
            CargoType.Uranium => ResourceType.Uranium,
            _ => ResourceType.Components
        };
    }

    private static Structure CreateStructure(Ulid id, StructureType type, string name, int x, int y)
    {
        var (power, generation, oxygen, capacity) = type switch
        {
            StructureType.Habitat => (5.0, 0.0, 0.0, 20),
            StructureType.LifeSupport => (10.0, 0.0, 100.0, 0),
            StructureType.PowerGenerator => (0.0, 50.0, 0.0, 0),
            StructureType.SolarPanel => (0.0, 20.0, 0.0, 0),
            StructureType.NuclearReactor => (0.0, 200.0, 0.0, 0),
            StructureType.Battery => (0.0, 0.0, 0.0, 0),
            _ => (1.0, 0.0, 0.0, 0)
        };

        return new Structure
        {
            Id = id,
            Name = name,
            Type = type,
            Location = new SectorCoordinates(x, y),
            Condition = 100,
            PowerConsumptionKW = power,
            PowerGenerationKW = generation,
            OxygenGeneration = oxygen,
            Capacity = capacity,
            IsOperational = true
        };
    }

    private static Colony RecalculateColonyStats(Colony colony)
    {
        var powerGen = colony.Structures.Sum(s => s.IsOperational ? s.PowerGenerationKW : 0);
        var powerCon = colony.Structures.Sum(s => s.IsOperational ? s.PowerConsumptionKW : 0);
        var oxygenGen = colony.Structures.Sum(s => s.IsOperational ? s.OxygenGeneration : 0);
        var oxygenCon = colony.ColonistIds.Count * 1.0; // 1 unit per colonist per day

        return colony with
        {
            PowerGeneration = powerGen,
            PowerConsumption = powerCon,
            OxygenGeneration = oxygenGen,
            OxygenConsumption = oxygenCon
        };
    }
}
