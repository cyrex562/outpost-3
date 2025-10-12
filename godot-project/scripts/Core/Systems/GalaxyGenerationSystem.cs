using System;
using System.Collections.Generic;
using Godot;
using Outpost3.Core.Domain;

namespace Outpost3.Core.Systems;

/// <summary>
/// Pure functional system for generating the galaxy at game start.
/// Generates 100 stars around Sol using Perlin noise for realistic clustering.
/// </summary>
public static class GalaxyGenerationSystem
{
    private const int DEFAULT_STAR_COUNT = 100;
    private const float DEFAULT_RADIUS_LY = 100f;

    /// <summary>
    /// Generates a galaxy with Sol at the center and surrounding stars.
    /// </summary>
    /// <param name="seed">Random seed for deterministic generation.</param>
    /// <param name="starCount">Total number of stars including Sol.</param>
    /// <param name="radiusLY">Maximum radius in light-years from Sol.</param>
    /// <returns>List of star systems.</returns>
    public static List<StarSystem> GenerateGalaxy(int seed, int starCount = DEFAULT_STAR_COUNT, float radiusLY = DEFAULT_RADIUS_LY)
    {
        var systems = new List<StarSystem>();
        var rng = new Random(seed);

        // Generate Sol (home system) at center
        systems.Add(GenerateSol(seed));

        // Generate remaining stars with deterministic IDs
        for (int i = 1; i < starCount; i++)
        {
            var position = SamplePositionWithPerlin(rng, radiusLY, i);
            var distance = position.Length();
            var starId = GenerateDeterministicUlid(seed, i);
            var system = GenerateRandomStar(starId, position, distance, rng);
            systems.Add(system);
        }

        return systems;
    }

    /// <summary>
    /// Generates a deterministic Ulid from a seed and index.
    /// This ensures the same seed always produces the same IDs.
    /// </summary>
    private static Ulid GenerateDeterministicUlid(int seed, int index)
    {
        // Create a deterministic random generator from seed and index
        var rng = new Random(seed ^ index);

        // Generate 16 random bytes deterministically
        var bytes = new byte[16];
        rng.NextBytes(bytes);

        return new Ulid(bytes);
    }

    /// <summary>
    /// Generates Sol (Earth's star system) at the center of the galaxy.
    /// </summary>
    private static StarSystem GenerateSol(int seed)
    {
        return new StarSystem
        {
            Id = GenerateDeterministicUlid(seed, 0), // Sol always gets index 0
            Name = "Sol",
            Position = Vector3.Zero,
            DistanceFromSol = 0f,
            SpectralClass = "G2V",
            Luminosity = 1.0f,
            DiscoveryLevel = DiscoveryLevel.Explored,  // Sol starts fully explored
            Bodies = GenerateSolSystem()
        };
    }

    /// <summary>
    /// Generates the bodies in the Sol system (simplified).
    /// </summary>
    private static List<CelestialBody> GenerateSolSystem()
    {
        return new List<CelestialBody>
        {
            new CelestialBody { Id = Ulid.NewUlid(), Name = "Mercury", BodyType = "Planet", Composition = "Rocky", Explored = true, Temperature = 440f, Gravity = 0.38f },
            new CelestialBody { Id = Ulid.NewUlid(), Name = "Venus", BodyType = "Planet", Composition = "Rocky", Explored = true, Temperature = 737f, Gravity = 0.9f, AtmosphereType = "Toxic CO2" },
            new CelestialBody { Id = Ulid.NewUlid(), Name = "Earth", BodyType = "Planet", Composition = "Rocky", Explored = true, Temperature = 288f, Gravity = 1.0f, AtmosphereType = "Nitrogen-Oxygen", SurfaceType = "Terrestrial" },
            new CelestialBody { Id = Ulid.NewUlid(), Name = "Mars", BodyType = "Planet", Composition = "Rocky", Explored = true, Temperature = 210f, Gravity = 0.38f, AtmosphereType = "Thin CO2" },
            new CelestialBody { Id = Ulid.NewUlid(), Name = "Jupiter", BodyType = "Planet", Composition = "Gas Giant", Explored = true, Temperature = 165f, Gravity = 2.5f, AtmosphereType = "Hydrogen-Helium" },
            new CelestialBody { Id = Ulid.NewUlid(), Name = "Saturn", BodyType = "Planet", Composition = "Gas Giant", Explored = true, Temperature = 134f, Gravity = 1.1f, AtmosphereType = "Hydrogen-Helium" },
            new CelestialBody { Id = Ulid.NewUlid(), Name = "Uranus", BodyType = "Planet", Composition = "Ice Giant", Explored = true, Temperature = 76f, Gravity = 0.9f, AtmosphereType = "Hydrogen-Helium-Methane" },
            new CelestialBody { Id = Ulid.NewUlid(), Name = "Neptune", BodyType = "Planet", Composition = "Ice Giant", Explored = true, Temperature = 72f, Gravity = 1.1f, AtmosphereType = "Hydrogen-Helium-Methane" }
        };
    }

    /// <summary>
    /// Samples a position using Perlin noise for realistic star clustering.
    /// </summary>
    private static Vector3 SamplePositionWithPerlin(Random rng, float radiusLY, int starIndex)
    {
        // Use a combination of random spherical distribution and Perlin noise
        // This creates realistic clustering while still filling the space

        // Generate random spherical coordinates
        float theta = (float)(rng.NextDouble() * 2.0 * Math.PI);  // 0 to 2π
        float phi = (float)(Math.Acos(2.0 * rng.NextDouble() - 1.0));  // 0 to π (uniform sphere)

        // Base distance with some variance
        float baseDistance = (float)(rng.NextDouble() * radiusLY);

        // Apply Perlin-like noise for clustering
        // Using simple noise function (Godot's FastNoiseLite could be used if needed)
        float noiseValue = (float)SimplexNoise(theta * 2.0, phi * 2.0, starIndex * 0.1);

        // Adjust distance based on noise (creates denser and sparser regions)
        float distanceMod = 0.7f + (noiseValue * 0.6f);  // 0.4 to 1.3 multiplier
        float finalDistance = baseDistance * distanceMod;

        // Clamp to radius
        finalDistance = Math.Min(finalDistance, radiusLY);

        // Convert spherical to Cartesian
        float x = finalDistance * Mathf.Sin(phi) * Mathf.Cos(theta);
        float y = finalDistance * Mathf.Sin(phi) * Mathf.Sin(theta);
        float z = finalDistance * Mathf.Cos(phi);

        return new Vector3(x, y, z);
    }

    /// <summary>
    /// Simple Perlin-like noise function (simplified).
    /// </summary>
    private static double SimplexNoise(double x, double y, double z)
    {
        // Simplified noise - using sine waves
        // For production, consider using Godot's FastNoiseLite
        double noise = Math.Sin(x * 1.5 + y * 2.3) *
                      Math.Cos(y * 1.7 - z * 1.1) *
                      Math.Sin(z * 2.1 + x * 1.3);
        return (noise + 1.0) / 2.0;  // Normalize to 0-1
    }

    /// <summary>
    /// Generates a random star system.
    /// </summary>
    private static StarSystem GenerateRandomStar(Ulid id, Vector3 position, float distance, Random rng)
    {
        var spectralClass = SelectSpectralClass(rng);
        var luminosity = CalculateLuminosity(spectralClass, rng);
        var name = GenerateStarName(id, spectralClass, distance);

        return new StarSystem
        {
            Id = id,
            Name = name,
            Position = position,
            DistanceFromSol = distance,
            SpectralClass = spectralClass,
            Luminosity = luminosity,
            DiscoveryLevel = DiscoveryLevel.Detected,  // All stars start as Detected
            Bodies = new List<CelestialBody>()  // Bodies generated when probe arrives
        };
    }

    /// <summary>
    /// Selects a spectral class with realistic distribution.
    /// M-class stars are most common, O/B stars are rare.
    /// </summary>
    private static string SelectSpectralClass(Random rng)
    {
        var roll = rng.NextDouble();

        // Realistic distribution based on stellar demographics:
        // M: 76%, K: 12%, G: 8%, F: 3%, A: 0.6%, B: 0.13%, O: 0.00003%
        if (roll < 0.76) return "M" + rng.Next(0, 10);  // M0-M9
        if (roll < 0.88) return "K" + rng.Next(0, 10);  // K0-K9
        if (roll < 0.96) return "G" + rng.Next(0, 10);  // G0-G9
        if (roll < 0.99) return "F" + rng.Next(0, 10);  // F0-F9
        if (roll < 0.996) return "A" + rng.Next(0, 10); // A0-A9
        if (roll < 0.999) return "B" + rng.Next(0, 10); // B0-B9
        return "O" + rng.Next(5, 10);  // O5-O9 (only hot O stars)
    }

    /// <summary>
    /// Calculates luminosity based on spectral class.
    /// </summary>
    public static float CalculateLuminosity(string spectralClass, Random rng)
    {
        // Luminosity relative to Sol (logarithmic scale)
        var baseClass = spectralClass[0];
        float baseLuminosity = baseClass switch
        {
            'O' => 30000f,  // O-class: 30,000 - 1,000,000 L☉
            'B' => 1000f,   // B-class: 25 - 30,000 L☉
            'A' => 10f,     // A-class: 5 - 25 L☉
            'F' => 3f,      // F-class: 1.5 - 5 L☉
            'G' => 1f,      // G-class: 0.6 - 1.5 L☉ (Sun is G2V)
            'K' => 0.3f,    // K-class: 0.08 - 0.6 L☉
            'M' => 0.05f,   // M-class: 0.0001 - 0.08 L☉
            _ => 1f
        };

        // Add variance
        float variance = 0.5f + (float)(rng.NextDouble() * 1.5);  // 0.5x to 2x
        return baseLuminosity * variance;
    }

    /// <summary>
    /// Generates a procedural star name.
    /// Uses the Ulid bytes directly to avoid hash collisions.
    /// </summary>
    private static string GenerateStarName(Ulid id, string spectralClass, float distance)
    {
        // Use mix of catalog styles: HD, Gliese, Kepler, etc.
        var idStr = id.ToString().ToUpper();
        var bytes = id.ToByteArray();

        // Use first byte to select catalog type (0-4)
        var catalogType = (bytes[0] % 5) switch
        {
            0 => $"HD-{idStr.Substring(0, 6)}",
            1 => $"Gliese-{idStr.Substring(0, 6)}",
            2 => $"Kepler-{idStr.Substring(0, 6)}",
            3 => $"LHS-{idStr.Substring(0, 6)}",
            _ => $"2MASS-{idStr.Substring(0, 8)}"
        };

        return catalogType;
    }
}