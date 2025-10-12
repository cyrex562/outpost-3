using System.Collections.Generic;
using System;
using Godot;

namespace Outpost3.Core.Domain;

public record StarSystem
{
    public Ulid Id { get; init; }
    public string Name { get; init; } = "";

    /// <summary>
    /// Position in 3D space (light-years from Sol).
    /// Sol is at (0, 0, 0).
    /// </summary>
    public Vector3 Position { get; init; } = Vector3.Zero;

    /// <summary>
    /// Calculated distance from Sol in light-years.
    /// </summary>
    public float DistanceFromSol { get; init; } = 0f;

    public string SpectralClass { get; init; } = "";

    /// <summary>
    /// Luminosity relative to Sol (1.0 = Sol luminosity).
    /// Used for visual rendering (star size).
    /// </summary>
    public float Luminosity { get; init; } = 1.0f;

    /// <summary>
    /// Discovery state: Detected, Scanned, or Explored.
    /// </summary>
    public DiscoveryLevel DiscoveryLevel { get; init; } = DiscoveryLevel.Detected;

    public List<CelestialBody> Bodies { get; init; } = new();
}

public record CelestialBody
{
    public Ulid Id { get; init; }
    public string Name { get; init; } = "";
    public string BodyType { get; init; } = ""; // e.g. Planet, Moon, Asteroid Belt, Cometary Belt

    /// <summary>
    /// Composition: Rocky, Gas Giant, Ice Giant, Asteroid, Comet, Unknown
    /// </summary>
    public string Composition { get; init; } = "Unknown";

    /// <summary>
    /// Whether this body has been fully explored by a probe.
    /// </summary>
    public bool Explored { get; init; } = false;

    // Partial discovery data (revealed on system scan with random chance)

    /// <summary>
    /// Atmosphere type - null if not yet discovered.
    /// </summary>
    public string? AtmosphereType { get; init; } = null;

    /// <summary>
    /// Surface type - null if not yet discovered.
    /// </summary>
    public string? SurfaceType { get; init; } = null;

    // Full exploration data (revealed on body probe arrival)

    /// <summary>
    /// Surface temperature in Kelvin - null if not explored.
    /// </summary>
    public float? Temperature { get; init; } = null;

    /// <summary>
    /// Gravity in Earth gravities (1.0 = Earth) - null if not explored.
    /// </summary>
    public float? Gravity { get; init; } = null;

    /// <summary>
    /// Detected resources - null if not explored.
    /// </summary>
    public List<string>? Resources { get; init; } = null;

    /// <summary>
    /// Known hazards - null if not explored.
    /// </summary>
    public List<string>? Hazards { get; init; } = null;
}

