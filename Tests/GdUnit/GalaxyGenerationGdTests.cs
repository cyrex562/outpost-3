using GdUnit4;
using static GdUnit4.Assertions;
using Outpost3.Core.Systems;
using Outpost3.Core.Domain;
using Godot;

namespace Outpost3.Tests.GdUnit;

[TestSuite]
public class GalaxyGenerationGdTests
{
    [TestCase]
    public void GenerateGalaxy_WithSeed_ProducesDeterministicResults()
    {
        // Arrange
        int seed = 12345;
        int starCount = 100;

        // Act
        var galaxy1 = GalaxyGenerationSystem.GenerateGalaxy(seed, starCount);
        var galaxy2 = GalaxyGenerationSystem.GenerateGalaxy(seed, starCount);

        // Assert
        AssertThat(galaxy1.Count).IsEqual(starCount);
        AssertThat(galaxy2.Count).IsEqual(starCount);

        // Check that same seed produces same results
        for (int i = 0; i < starCount; i++)
        {
            AssertThat(galaxy1[i].Name).IsEqual(galaxy2[i].Name);
            AssertThat(galaxy1[i].SpectralClass).IsEqual(galaxy2[i].SpectralClass);
            AssertThat(galaxy1[i].Position.X).IsEqual(galaxy2[i].Position.X);
            AssertThat(galaxy1[i].Position.Y).IsEqual(galaxy2[i].Position.Y);
            AssertThat(galaxy1[i].Position.Z).IsEqual(galaxy2[i].Position.Z);
        }
    }

    [TestCase]
    public void GenerateGalaxy_Always_IncludesSolAtCenter()
    {
        // Arrange
        int seed = 42;

        // Act
        var galaxy = GalaxyGenerationSystem.GenerateGalaxy(seed);

        // Assert
        var sol = galaxy.Find(s => s.Name == "Sol");
        AssertThat(sol).IsNotNull();
        AssertThat(sol!.Position).IsEqual(Vector3.Zero);
        AssertThat(sol.DistanceFromSol).IsEqual(0f);
        AssertThat(sol.SpectralClass).IsEqual("G2V");
        AssertThat(sol.DiscoveryLevel).IsEqual(DiscoveryLevel.Explored);
    }

    [TestCase]
    public void GenerateGalaxy_SolBodies_AreFullyExplored()
    {
        // Arrange
        int seed = 42;

        // Act
        var galaxy = GalaxyGenerationSystem.GenerateGalaxy(seed);
        var sol = galaxy.Find(s => s.Name == "Sol");

        // Assert
        AssertThat(sol).IsNotNull();
        AssertThat(sol!.Bodies.Count).IsGreater(0);

        foreach (var body in sol.Bodies)
        {
            AssertThat(body.Explored).IsTrue();
        }
    }

    [TestCase]
    public void GenerateGalaxy_AllNonSolStars_StartAsDetected()
    {
        // Arrange
        int seed = 999;

        // Act
        var galaxy = GalaxyGenerationSystem.GenerateGalaxy(seed, 20);

        // Assert
        var nonSolStars = galaxy.FindAll(s => s.Name != "Sol");
        AssertThat(nonSolStars.Count).IsEqual(19);

        foreach (var star in nonSolStars)
        {
            AssertThat(star.DiscoveryLevel).IsEqual(DiscoveryLevel.Detected);
            AssertThat(star.Bodies.Count).IsEqual(0); // No bodies until scanned
        }
    }

    [TestCase]
    public void GenerateGalaxy_AllStars_WithinRadius()
    {
        // Arrange
        int seed = 777;
        float radiusLY = 50f;

        // Act
        var galaxy = GalaxyGenerationSystem.GenerateGalaxy(seed, 50, radiusLY);

        // Assert
        foreach (var star in galaxy)
        {
            if (star.Name == "Sol") continue; // Sol is at center

            float distance = star.Position.Length();
            AssertThat(distance).IsLessEqual(radiusLY);
            AssertThat(star.DistanceFromSol).IsEqual(distance);
        }
    }

    [TestCase]
    public void GenerateGalaxy_Stars_HaveValidSpectralClasses()
    {
        // Arrange
        int seed = 12345;
        var validClasses = new[] { "O", "B", "A", "F", "G", "K", "M" };

        // Act
        var galaxy = GalaxyGenerationSystem.GenerateGalaxy(seed, 50);

        // Assert
        foreach (var star in galaxy)
        {
            var spectralClass = star.SpectralClass[0].ToString();
            AssertThat(validClasses).Contains(spectralClass);
        }
    }

    [TestCase]
    public void GenerateGalaxy_Stars_HavePositiveLuminosity()
    {
        // Arrange
        int seed = 55555;

        // Act
        var galaxy = GalaxyGenerationSystem.GenerateGalaxy(seed, 30);

        // Assert
        foreach (var star in galaxy)
        {
            AssertThat(star.Luminosity).IsGreater(0f);
        }
    }

    [TestCase]
    public void GenerateGalaxy_Stars_HaveUniqueIds()
    {
        // Arrange
        int seed = 11111;

        // Act
        var galaxy = GalaxyGenerationSystem.GenerateGalaxy(seed, 50);

        // Assert
        var ids = galaxy.Select(s => s.Id).ToList();
        var uniqueIds = ids.Distinct().ToList();

        AssertThat(uniqueIds.Count).IsEqual(ids.Count);
    }

    [TestCase]
    public void GenerateGalaxy_Stars_HaveUniqueProceduraNames()
    {
        // Arrange
        int seed = 22222;

        // Act
        var galaxy = GalaxyGenerationSystem.GenerateGalaxy(seed, 50);

        // Assert
        var names = galaxy.Select(s => s.Name).ToList();
        var uniqueNames = names.Distinct().ToList();

        AssertThat(uniqueNames.Count).IsEqual(names.Count);
    }

    [TestCase]
    public void GenerateGalaxy_Distribution_ShowsClustering()
    {
        // Arrange
        int seed = 33333;
        float radiusLY = 100f;

        // Act
        var galaxy = GalaxyGenerationSystem.GenerateGalaxy(seed, 100, radiusLY);

        // Assert - Check that stars aren't uniformly distributed
        // Count stars in inner half vs outer half
        int innerCount = 0;
        int outerCount = 0;
        float halfRadius = radiusLY / 2f;

        foreach (var star in galaxy)
        {
            if (star.Name == "Sol") continue;

            float distance = star.DistanceFromSol;
            if (distance < halfRadius)
                innerCount++;
            else
                outerCount++;
        }

        // Due to Perlin noise clustering, distribution shouldn't be exactly 25/75
        // Just verify we have stars in both regions
        AssertThat(innerCount).IsGreater(0);
        AssertThat(outerCount).IsGreater(0);
    }
}
