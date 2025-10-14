using System;
using Outpost3.Core.Domain;

namespace Outpost3.Core.Projections;

/// <summary>
/// Helper for formatting game values for display.
/// Pure static methods for consistent UI formatting.
/// </summary>
public static class DisplayFormatter
{
    /// <summary>
    /// Formats a distance in AU for display.
    /// </summary>
    public static string FormatDistance(double distanceAU)
    {
        if (distanceAU < 0.01)
        {
            // Very close - show in kilometers
            var km = distanceAU * 149597870.7; // 1 AU in km
            return $"{km:F0} km";
        }
        else if (distanceAU < 1.0)
        {
            return $"{distanceAU:F2} AU";
        }
        else if (distanceAU < 100.0)
        {
            return $"{distanceAU:F1} AU";
        }
        else
        {
            return $"{distanceAU:F0} AU";
        }
    }

    /// <summary>
    /// Formats an orbital period in days for display.
    /// </summary>
    public static string FormatOrbitalPeriod(double periodDays)
    {
        if (periodDays < 1.0)
        {
            var hours = periodDays * 24.0;
            return $"{hours:F1} hours";
        }
        else if (periodDays < 365.25)
        {
            return $"{periodDays:F1} days";
        }
        else
        {
            var years = periodDays / 365.25;
            return $"{years:F1} years";
        }
    }

    /// <summary>
    /// Formats a mass in Earth masses for display.
    /// </summary>
    public static string FormatMass(double massEarthMasses)
    {
        if (massEarthMasses < 0.01)
        {
            return $"{massEarthMasses:E2} M⊕"; // Scientific notation
        }
        else if (massEarthMasses < 100.0)
        {
            return $"{massEarthMasses:F2} M⊕";
        }
        else
        {
            return $"{massEarthMasses:F0} M⊕";
        }
    }

    /// <summary>
    /// Formats a radius in kilometers for display.
    /// </summary>
    public static string FormatRadius(double radiusKm)
    {
        if (radiusKm < 1000.0)
        {
            return $"{radiusKm:F0} km";
        }
        else
        {
            return $"{(radiusKm / 1000.0):F1}k km";
        }
    }

    /// <summary>
    /// Formats game time for display.
    /// </summary>
    public static string FormatGameTime(double gameTimeHours)
    {
        var totalDays = gameTimeHours / 24.0;
        var years = (int)(totalDays / 365.25);
        var remainingDays = (int)(totalDays % 365.25);

        if (years > 0)
        {
            return $"Year {years}, Day {remainingDays}";
        }
        else
        {
            return $"Day {(int)totalDays}";
        }
    }

    /// <summary>
    /// Formats game speed for display.
    /// </summary>
    public static string FormatGameSpeed(GameSpeed speed)
    {
        return speed switch
        {
            GameSpeed.Paused => "Paused",
            GameSpeed.Normal => "1x",
            GameSpeed.Fast => "2x",
            GameSpeed.Faster => "5x",
            GameSpeed.Fastest => "10x",
            _ => speed.ToString()
        };
    }

    /// <summary>
    /// Formats game speed for display with context-aware labels.
    /// Shows what time period passes per real-world second.
    /// </summary>
    public static string FormatGameSpeedWithContext(GameSpeed speed, bool isGalaxyContext)
    {
        if (isGalaxyContext)
        {
            return speed switch
            {
                GameSpeed.Paused => "Paused",
                GameSpeed.Normal => "1 month/sec",
                GameSpeed.Fast => "3 months/sec",
                GameSpeed.Faster => "1 year/sec",
                GameSpeed.Fastest => "5 years/sec",
                _ => speed.ToString()
            };
        }
        else
        {
            return speed switch
            {
                GameSpeed.Paused => "Paused",
                GameSpeed.Normal => "4 hours/sec",
                GameSpeed.Fast => "12 hours/sec",
                GameSpeed.Faster => "1 week/sec",
                GameSpeed.Fastest => "1 month/sec",
                _ => speed.ToString()
            };
        }
    }

    /// <summary>
    /// Gets a multiplier for game speed.
    /// </summary>
    public static double GetSpeedMultiplier(GameSpeed speed)
    {
        return speed switch
        {
            GameSpeed.Paused => 0.0,
            GameSpeed.Normal => 1.0,
            GameSpeed.Fast => 2.0,
            GameSpeed.Faster => 5.0,
            GameSpeed.Fastest => 10.0,
            _ => 1.0
        };
    }

    /// <summary>
    /// Formats a spectral class with description.
    /// </summary>
    public static string FormatSpectralClass(string spectralClass)
    {
        var description = spectralClass.ToUpperInvariant() switch
        {
            "O" => "Blue supergiant",
            "B" => "Blue giant",
            "A" => "White star",
            "F" => "Yellow-white star",
            "G" => "Yellow star (Sun-like)",
            "K" => "Orange star",
            "M" => "Red dwarf",
            _ => "Unknown class"
        };

        return $"{spectralClass}-class ({description})";
    }

    /// <summary>
    /// Formats luminosity relative to the Sun.
    /// </summary>
    public static string FormatLuminosity(float luminosity)
    {
        if (luminosity < 0.1)
        {
            return $"{luminosity:F3} L☉";
        }
        else if (luminosity < 10.0)
        {
            return $"{luminosity:F2} L☉";
        }
        else
        {
            return $"{luminosity:F1} L☉";
        }
    }
}
