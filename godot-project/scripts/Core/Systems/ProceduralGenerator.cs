using System;
using System.Collections.Generic;
using System.Linq;
using Outpost3.Core.Domain;

namespace Outpost3.Core.Systems;

/// <summary>
/// Procedurally generates detailed star system properties.
/// All generation is deterministic based on a seed.
/// </summary>
public static class ProceduralGenerator
{
    /// <summary>
    /// Generate detailed orbital parameters and properties for all bodies in a system.
    /// </summary>
    /// <param name="system">The star system to generate details for.</param>
    /// <param name="seed">The seed for deterministic generation.</param>
    /// <returns>Tuple of (bodies with orbital params, asteroid belts, Oort cloud).</returns>
    public static (
        List<CelestialBody> bodies,
        List<AsteroidBelt> belts,
        OortCloud oortCloud
    ) GenerateSystemDetails(StarSystem system, SystemSeed seed)
    {
        var random = new Random(seed.Value.GetHashCode());

        // Generate orbital parameters for existing bodies
        var bodiesWithOrbits = new List<CelestialBody>();
        for (int i = 0; i < system.Bodies.Count; i++)
        {
            var body = system.Bodies[i];
            var distance = GenerateOrbitalDistance(i, random);
            var period = CalculateOrbitalPeriod(distance);
            var startAngle = random.NextDouble() * 360.0;
            var eccentricity = random.NextDouble() * 0.1; // Low eccentricity for simplicity

            var orbitalParams = new OrbitalParameters(
                distance,
                period,
                startAngle,
                eccentricity
            );

            var mass = GenerateMass(body.BodyType, random);
            var radius = GenerateRadius(body.BodyType, mass, random);

            bodiesWithOrbits.Add(body with
            {
                OrbitalParams = orbitalParams,
                MassEarthMasses = mass,
                RadiusKm = radius
            });
        }

        // Generate asteroid belts (0-2)
        var belts = GenerateAsteroidBelts(bodiesWithOrbits, random);

        // Generate Oort cloud
        var maxOrbit = bodiesWithOrbits.Count > 0
            ? bodiesWithOrbits.Max(b => b.OrbitalParams!.SemiMajorAxisAU)
            : 30.0; // Default if no bodies
        var oortCloud = new OortCloud { RadiusAU = maxOrbit * 100 }; // ~100x furthest planet

        return (bodiesWithOrbits, belts, oortCloud);
    }

    /// <summary>
    /// Generate orbital distance using a Titius-Bode-like law with randomization.
    /// </summary>
    private static double GenerateOrbitalDistance(int index, Random random)
    {
        // Titius-Bode formula: a = 0.4 + 0.3 * 2^n
        // Add randomization to make it less regular
        var baseDistance = 0.4 + (0.3 * Math.Pow(2, index));
        var randomFactor = 0.8 + random.NextDouble() * 0.4; // 80% to 120%
        return baseDistance * randomFactor;
    }

    /// <summary>
    /// Calculate orbital period using Kepler's third law.
    /// Assumes star mass = 1 solar mass.
    /// </summary>
    private static double CalculateOrbitalPeriod(double semiMajorAxisAU)
    {
        // Kepler's third law: T^2 = a^3 (for solar masses and AU)
        // T in Earth years, convert to days
        return 365.25 * Math.Pow(semiMajorAxisAU, 1.5);
    }

    /// <summary>
    /// Generate mass based on body type.
    /// </summary>
    private static double GenerateMass(string bodyType, Random random)
    {
        return bodyType switch
        {
            "Gas Giant" => 50 + random.NextDouble() * 300,   // 50-350 Earth masses
            "Ice Giant" => 10 + random.NextDouble() * 20,    // 10-30 Earth masses
            "Terrestrial" => 0.1 + random.NextDouble() * 5,  // 0.1-5 Earth masses
            "Dwarf" => 0.001 + random.NextDouble() * 0.1,    // Very small
            _ => 1.0 // Default to Earth-like
        };
    }

    /// <summary>
    /// Generate radius based on body type and mass.
    /// </summary>
    private static double GenerateRadius(string bodyType, double mass, Random random)
    {
        // Base radius with some randomization
        var baseRadius = bodyType switch
        {
            "Gas Giant" => 40000 + random.NextDouble() * 40000,    // 40k-80k km
            "Ice Giant" => 20000 + random.NextDouble() * 30000,    // 20k-50k km
            "Terrestrial" => 3000 + random.NextDouble() * 10000,   // 3k-13k km
            "Dwarf" => 1000 + random.NextDouble() * 3000,          // 1k-4k km
            _ => 6371.0 // Earth radius as default
        };

        // Add slight correlation with mass (more massive = slightly larger)
        var massInfluence = 1.0 + (Math.Log10(mass) * 0.1);
        return baseRadius * Math.Max(0.8, Math.Min(1.2, massInfluence));
    }

    /// <summary>
    /// Generate 0-2 asteroid belts positioned between planets.
    /// </summary>
    private static List<AsteroidBelt> GenerateAsteroidBelts(
        List<CelestialBody> bodies,
        Random random)
    {
        var belts = new List<AsteroidBelt>();
        var beltCount = random.Next(0, 3); // 0-2 belts

        for (int i = 0; i < beltCount; i++)
        {
            // Place belts at reasonable distances
            // Try to place between planets if possible
            double innerRadius;
            double outerRadius;

            if (bodies.Count > i + 1)
            {
                // Place between planet i and i+1
                var planet1Orbit = bodies[i].OrbitalParams?.SemiMajorAxisAU ?? (1.5 + i * 2.0);
                var planet2Orbit = bodies[i + 1].OrbitalParams?.SemiMajorAxisAU ?? (planet1Orbit + 2.0);

                // Belt in middle, with some width
                var middle = (planet1Orbit + planet2Orbit) / 2.0;
                var width = (planet2Orbit - planet1Orbit) * 0.3; // 30% of gap
                innerRadius = middle - width / 2.0;
                outerRadius = middle + width / 2.0;
            }
            else
            {
                // Place at arbitrary distance with random offset
                innerRadius = 1.5 + i * 2.0 + random.NextDouble();
                outerRadius = innerRadius + 0.5 + random.NextDouble() * 0.5;
            }

            belts.Add(new AsteroidBelt
            {
                Id = Ulid.NewUlid(),
                Name = $"Asteroid Belt {i + 1}",
                InnerRadiusAU = innerRadius,
                OuterRadiusAU = outerRadius
            });
        }

        return belts;
    }
}
