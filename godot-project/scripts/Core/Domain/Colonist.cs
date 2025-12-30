using System;
using System.Collections.Generic;

namespace Outpost3.Core.Domain;

/// <summary>
/// Represents an individual colonist with skills, health, and morale.
/// </summary>
public record Colonist
{
    public Ulid Id { get; init; }
    public string Name { get; init; } = "";
    public string Gender { get; init; } = ""; // Male, Female, Other
    public int Age { get; init; } = 30;

    /// <summary>
    /// Primary role: Engineer, Scientist, Medic, Laborer, Pilot, etc.
    /// </summary>
    public ColonistRole Role { get; init; } = ColonistRole.Laborer;

    /// <summary>
    /// Skill levels (0-100) for different disciplines.
    /// </summary>
    public Dictionary<SkillType, int> Skills { get; init; } = new();

    /// <summary>
    /// Health status (0-100, where 100 is perfect health).
    /// </summary>
    public int Health { get; init; } = 100;

    /// <summary>
    /// Morale status (0-100, where 100 is excellent morale).
    /// </summary>
    public int Morale { get; init; } = 75;

    /// <summary>
    /// Fatigue level (0-100, where 100 is exhausted).
    /// </summary>
    public int Fatigue { get; init; } = 0;

    /// <summary>
    /// Current assignment: building ID, task ID, or null if unassigned.
    /// </summary>
    public Ulid? AssignmentId { get; init; } = null;

    /// <summary>
    /// Special traits that affect performance and events.
    /// </summary>
    public List<ColonistTrait> Traits { get; init; } = new();

    /// <summary>
    /// Whether the colonist is in cryosleep (for sleeper ships).
    /// </summary>
    public bool InCryosleep { get; init; } = false;

    /// <summary>
    /// Experience points in current role (used for skill progression).
    /// </summary>
    public int RoleExperience { get; init; } = 0;
}

/// <summary>
/// Primary roles for colonists.
/// </summary>
public enum ColonistRole
{
    Engineer,
    Scientist,
    Medic,
    Pilot,
    Miner,
    Farmer,
    Laborer,
    Administrator,
    SecurityOfficer,
    Technician
}

/// <summary>
/// Skill types that colonists can possess.
/// </summary>
public enum SkillType
{
    Engineering,
    Science,
    Medicine,
    Piloting,
    Mining,
    Agriculture,
    Construction,
    Repair,
    Research,
    Leadership,
    Combat,
    Survival
}

/// <summary>
/// Traits that affect colonist behavior and performance.
/// </summary>
public enum ColonistTrait
{
    // Positive traits
    Hardy,          // Resistant to harsh conditions
    Optimistic,     // Better morale
    QuickLearner,   // Faster skill progression
    Resilient,      // Faster health recovery
    Leader,         // Boosts morale of nearby colonists
    Genius,         // Better research/engineering

    // Negative traits
    Pessimistic,    // Lower morale
    Fragile,        // More susceptible to injury
    SlowLearner,    // Slower skill progression
    Lazy,           // Lower productivity
    Claustrophobic, // Poor morale in enclosed spaces

    // Neutral/situational traits
    Insomniac,      // Less sleep needed but lower morale
    Workaholic,     // High productivity but faster fatigue
    SocialButterfly // Boosts morale through interaction
}
