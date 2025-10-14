namespace Outpost3.Core.Domain;

/// <summary>
/// Physics constants for realistic space travel calculations.
/// </summary>
public static class PhysicsConstants
{
    /// <summary>
    /// Speed of light in vacuum (m/s).
    /// </summary>
    public const double SpeedOfLightMs = 299_792_458.0;

    /// <summary>
    /// Speed of light in km/s.
    /// </summary>
    public const double SpeedOfLightKmS = SpeedOfLightMs / 1000.0;

    /// <summary>
    /// Number of seconds in a year (365.25 days * 24 hours * 3600 seconds).
    /// </summary>
    public const double SecondsPerYear = 365.25 * 24 * 3600;

    /// <summary>
    /// One light-year in kilometers.
    /// Distance = speed * time = c * 1 year
    /// </summary>
    public const double LightYearKm = SpeedOfLightKmS * SecondsPerYear;

    /// <summary>
    /// Probe travel speed as a fraction of the speed of light.
    /// Set to 90% (0.9c) for reasonable interstellar travel.
    /// </summary>
    public const double ProbeSpeedFractionOfC = 0.9;

    /// <summary>
    /// Actual probe speed in km/s.
    /// </summary>
    public const double ProbeSpeedKmS = SpeedOfLightKmS * ProbeSpeedFractionOfC;

    /// <summary>
    /// Calculate travel time in hours for a given distance in light-years.
    /// </summary>
    /// <param name="distanceLightYears">Distance to travel in light-years.</param>
    /// <returns>Travel time in hours.</returns>
    public static double CalculateProbeTraverTime(double distanceLightYears)
    {
        // Time = Distance / Speed
        // Convert light-years to km, divide by probe speed, convert result to hours
        var distanceKm = distanceLightYears * LightYearKm;
        var travelTimeSeconds = distanceKm / ProbeSpeedKmS;
        var travelTimeHours = travelTimeSeconds / 3600.0;

        return travelTimeHours;
    }

    /// <summary>
    /// Calculate travel time in game time units for display.
    /// Returns a tuple of (years, days, hours).
    /// </summary>
    /// <param name="distanceLightYears">Distance to travel in light-years.</param>
    /// <returns>Tuple of (years, days, hours) for the travel time.</returns>
    public static (int years, int days, double hours) CalculateProbeTraverTimeForDisplay(double distanceLightYears)
    {
        var totalHours = CalculateProbeTraverTime(distanceLightYears);

        var years = (int)(totalHours / (365.25 * 24));
        var remainingHours = totalHours - (years * 365.25 * 24);

        var days = (int)(remainingHours / 24);
        var hours = remainingHours - (days * 24);

        return (years, days, hours);
    }

    /// <summary>
    /// Format travel time for user display.
    /// </summary>
    /// <param name="distanceLightYears">Distance to travel in light-years.</param>
    /// <returns>Formatted string like "2 years, 45 days" or "15 days, 6 hours".</returns>
    public static string FormatProbeTraverTime(double distanceLightYears)
    {
        var (years, days, hours) = CalculateProbeTraverTimeForDisplay(distanceLightYears);

        if (years > 0)
        {
            return days > 0 ? $"{years} years, {days} days" : $"{years} years";
        }
        else if (days > 0)
        {
            return hours > 1 ? $"{days} days, {hours:F0} hours" : $"{days} days";
        }
        else
        {
            return $"{hours:F1} hours";
        }
    }
}