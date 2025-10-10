using System;
using System.Collections.Generic;
using System.Globalization;
using Godot;
using HarshRealm.Data;

namespace HarshRealm.Services;

public readonly record struct OrbitalChange(string BodyName, double AngleDegrees, double DistanceKm, DateTime Date);

public sealed class SolarSystemService
{
    private readonly Dictionary<string, CelestialBody> _celestialBodies = new();
    private readonly Dictionary<string, double> _previousAngles = new();
    private DateTime _gameDate;

    public SolarSystemService(DateTime startDate)
    {
        _gameDate = startDate;
    }

    public IReadOnlyDictionary<string, CelestialBody> Bodies => _celestialBodies;

    public DateTime GameDate => _gameDate;

    public string FormattedDate => _gameDate.ToString("yyyy MMMM dd", CultureInfo.InvariantCulture);

    public bool LoadFromCsv(string resourcePath)
    {
        try
        {
            using var file = FileAccess.Open(resourcePath, FileAccess.ModeFlags.Read);
            if (file is null)
            {
                GD.PrintErr($"Unable to open resource at {resourcePath}");
                return false;
            }

            var text = file.GetAsText();
            var lines = text.Split('\n', StringSplitOptions.RemoveEmptyEntries);
            var headerSkipped = false;
            var loaded = 0;
            var skipped = 0;

            foreach (var rawLine in lines)
            {
                var line = rawLine.Trim();
                if (string.IsNullOrWhiteSpace(line))
                {
                    continue;
                }

                if (!headerSkipped)
                {
                    headerSkipped = true;
                    continue;
                }

                var parts = line.Split(',', StringSplitOptions.None);
                if (parts.Length < 9)
                {
                    skipped += 1;
                    continue;
                }

                var bodyName = parts[1].Trim();
                if (string.Equals(bodyName, "The Sun", StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                var bodyTypeRaw = parts.Length > 2 ? parts[2].Trim() : string.Empty;

                var semiMajorAxis = ParseDouble(parts, 5);
                var eccentricity = ParseDouble(parts, 6);
                var orbitalPeriod = ParseDouble(parts, 7);

                if (semiMajorAxis is null || eccentricity is null || orbitalPeriod is null)
                {
                    skipped += 1;
                    continue;
                }

                var meanAnomaly = ParseDouble(parts, 8) ?? 0.0;
                var mass = ParseDouble(parts, 12) ?? 0.0;
                var diameter = ParseDouble(parts, 13) ?? 0.0;

                var bodyType = DetermineBodyType(bodyName, bodyTypeRaw);

                var parameters = new OrbitalParameters(
                    semiMajorAxis.Value,
                    eccentricity.Value,
                    orbitalPeriod.Value,
                    meanAnomaly);

                var state = new OrbitalState(parameters, _gameDate);

                var body = new CelestialBody(bodyName, bodyType, mass, diameter);
                body.SetOrbitalState(state);

                _celestialBodies[bodyName] = body;
                _previousAngles[bodyName] = state.CurrentPosition.AngleRadians;
                loaded += 1;
            }

            GD.Print($"Loaded {loaded} celestial bodies (skipped {skipped}).");
            return true;
        }
        catch (Exception ex)
        {
            GD.PrintErr($"Failed to load solar system data: {ex.Message}");
            return false;
        }
    }

    public List<OrbitalChange> UpdateAllPositions(double daysElapsed)
    {
        var updates = new List<OrbitalChange>();

        foreach (var (name, body) in _celestialBodies)
        {
            if (body.OrbitalState is null)
            {
                continue;
            }

            var previousAngle = _previousAngles.TryGetValue(name, out var angle) ? angle : 0.0;

            body.OrbitalState.UpdatePosition(daysElapsed);

            if (body.OrbitalState.IsSignificantChange(previousAngle, 1.0))
            {
                updates.Add(new OrbitalChange(
                    name,
                    body.OrbitalState.AngleDegrees,
                    body.OrbitalState.CurrentPosition.DistanceKm,
                    body.OrbitalState.CurrentDate));

                _previousAngles[name] = body.OrbitalState.CurrentPosition.AngleRadians;
            }
        }

        _gameDate = _gameDate.AddDays(daysElapsed);
        return updates;
    }

    private static double? ParseDouble(string[] parts, int index)
    {
        if (index >= parts.Length)
        {
            return null;
        }

        var raw = parts[index].Trim();
        if (string.IsNullOrEmpty(raw) || raw == "?")
        {
            return null;
        }

        if (double.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out var value))
        {
            return value;
        }

        return null;
    }

    private static CelestialBodyType DetermineBodyType(string name, string? typeString)
    {
        var normalized = typeString?.ToLowerInvariant();
        if (normalized == "star")
        {
            return CelestialBodyType.Star;
        }

        if (normalized == "rocky planet" || normalized == "gas giant planet")
        {
            return CelestialBodyType.Planet;
        }

        if (normalized == "rocky moon")
        {
            return CelestialBodyType.Moon;
        }

        if (normalized == "dwarf planet")
        {
            return CelestialBodyType.DwarfPlanet;
        }

        var nameLower = name.ToLowerInvariant();
        if (nameLower.Contains("moon") || nameLower.Contains("luna"))
        {
            return CelestialBodyType.Moon;
        }

        if (nameLower.Contains("asteroid") || nameLower.Contains("ceres") || nameLower.Contains("pallas") || nameLower.Contains("vesta"))
        {
            return CelestialBodyType.Asteroid;
        }

        if (nameLower.Contains("comet"))
        {
            return CelestialBodyType.Comet;
        }

        if (nameLower.Contains("pluto") || nameLower.Contains("eris") || nameLower.Contains("makemake") || nameLower.Contains("haumea"))
        {
            return CelestialBodyType.DwarfPlanet;
        }

        return CelestialBodyType.Planet;
    }
}
