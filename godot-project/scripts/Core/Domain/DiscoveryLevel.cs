namespace Outpost3.Core.Domain;

/// <summary>
/// Discovery level for star systems and celestial bodies.
/// Represents how much information the player has about an object.
/// </summary>
public enum DiscoveryLevel
{
    /// <summary>
    /// Object is not known to exist.
    /// </summary>
    Unknown = 0,

    /// <summary>
    /// Object detected - position and name known, no details.
    /// </summary>
    Detected = 1,

    /// <summary>
    /// System scanned by probe - star characteristics and body list known.
    /// </summary>
    Scanned = 2,

    /// <summary>
    /// Fully explored - all details revealed.
    /// </summary>
    Explored = 3
}
