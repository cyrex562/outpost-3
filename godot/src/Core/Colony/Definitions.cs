namespace OutpostGame.Core.Colony;

public enum ResourceTier { Raw, Refined, Advanced, Virtual }
public enum ResourceCategory { Mineral, Chemical, Biological, Energy, Manufactured, Virtual }
public enum SkillType { Laborer, Engineer, Scientist, Farmer, Medic, Operator }
public enum BuildingState { Planned, UnderConstruction, Operational, Powered, Unpowered, Damaged, Destroyed }
public enum BiomeType { Barren, Rocky, Polar, Desert, Volcanic, MarginalHabitable }
public enum TerrainType { Flat, Slope, Rough, Crater, Impassable }
public enum ClimateZone { Frigid, Cold, Temperate, Hot, Extreme }
public enum DifficultyPreset { Sandbox, Easy, Normal, Hard, Brutal }
public enum ProjectCategory { Planetary, SystemWide, Infrastructure, Research, Military }
public enum ProjectStatus { Available, Active, Paused, Complete, Failed }
public enum BuildingCategory { Power, Production, Habitat, Storage, LifeSupport }

public enum ColonyStatus
{
    Active,     // Normal play
    Critical,   // Morale < 20 for ≥5 consecutive turns — warning, reversible
    Abandoned,  // Morale < 20 for ≥15 consecutive turns — terminal
    Destroyed,  // Population reached 0 — terminal
    Thriving,   // Population ≥50, morale ≥80, health ≥80 for ≥30 turns — milestone
}
