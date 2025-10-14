using System;
using System.Security.Cryptography;
using System.Text;
using Godot;

namespace Outpost3.Core.Domain;

/// <summary>
/// Orbital parameters for celestial bodies.
/// </summary>
public record OrbitalParameters(
    double SemiMajorAxisAU,      // Distance from star in AU
    double OrbitalPeriodDays,    // Time to complete one orbit
    double StartingAngleDegrees, // Initial position on orbit (0-360)
    double Eccentricity          // 0 = circular, >0 = elliptical
);

/// <summary>
/// Camera state for viewport persistence.
/// </summary>
public record CameraState(
    Vector2 PanPosition,  // Camera offset from center
    float ZoomLevel       // Zoom multiplier (1.0 = default)
);

/// <summary>
/// Screen navigation identifier.
/// </summary>
public record ScreenId(string Value)
{
    public static ScreenId GalaxyMap => new("GalaxyMap");
    public static ScreenId StarSystemMap => new("StarSystemMap");
    public static ScreenId ShipJourneyLog => new("ShipJourneyLog");
    public static ScreenId SystemDetails => new("SystemDetails");

    public override string ToString() => Value;
}

/// <summary>
/// Seed for deterministic procedural generation.
/// Alphanumeric, uppercase, 8 characters.
/// </summary>
public record SystemSeed(string Value)
{
    public static SystemSeed FromSystemId(Ulid systemId)
    {
        return new SystemSeed(GenerateSeedFromUlid(systemId.ToString()));
    }

    private static string GenerateSeedFromUlid(string ulid)
    {
        // Hash ULID to create 8-character alphanumeric seed
        var hash = SHA256.HashData(Encoding.UTF8.GetBytes(ulid));
        var base64 = Convert.ToBase64String(hash);

        // Remove non-alphanumeric characters and take first 8 chars
        var seed = new StringBuilder();
        foreach (var c in base64)
        {
            if (char.IsLetterOrDigit(c))
            {
                seed.Append(c);
            }
            if (seed.Length >= 8)
            {
                break;
            }
        }

        return seed.ToString().ToUpperInvariant();
    }

    public override string ToString() => Value;
}

/// <summary>
/// Game speed multiplier.
/// </summary>
public enum GameSpeed
{
    Paused = 0,
    Normal = 1,
    Fast = 2,
    Faster = 5,
    Fastest = 10
}
